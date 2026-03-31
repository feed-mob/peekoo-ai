//! Archive extraction utilities for tar.gz and zip files
//! Replaces util::archive module from Zed

use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::{Component, Path, PathBuf};

fn safe_zip_entry_path(dest: &Path, entry_name: &str) -> Result<PathBuf> {
    let entry_path = Path::new(entry_name);
    let mut relative = PathBuf::new();

    for component in entry_path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("zip entry contains an unsafe path: {entry_name}");
            }
        }
    }

    if relative.as_os_str().is_empty() {
        anyhow::bail!("zip entry resolved to an empty path");
    }

    Ok(dest.join(relative))
}

fn safe_archive_entry_path(dest: &Path, entry_path: &Path) -> Result<PathBuf> {
    let mut relative = PathBuf::new();

    for component in entry_path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!(
                    "archive entry contains an unsafe path: {}",
                    entry_path.display()
                );
            }
        }
    }

    if relative.as_os_str().is_empty() {
        anyhow::bail!("archive entry resolved to an empty path");
    }

    Ok(dest.join(relative))
}

/// Extract a .tar.gz archive to the specified directory
pub async fn extract_targz(archive: Bytes, dest: &Path) -> Result<()> {
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        use flate2::read::GzDecoder;
        use std::io;
        use std::io::Cursor;
        use tar::Archive;
        use tar::EntryType;

        let cursor = Cursor::new(archive);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        for entry in archive
            .entries()
            .context("Failed to read tar.gz archive entries")?
        {
            let mut entry = entry.context("Failed to read tar.gz archive entry")?;
            let entry_type = entry.header().entry_type();

            if entry_type.is_symlink() || entry_type == EntryType::Link {
                anyhow::bail!("tar archive contains unsupported link entry");
            }

            let entry_path = entry
                .path()
                .context("Failed to resolve tar.gz entry path")?;
            let outpath = safe_archive_entry_path(&dest, &entry_path)?;

            if entry_type.is_dir() {
                std::fs::create_dir_all(&outpath)
                    .context("Failed to create directory from tar.gz")?;
                continue;
            }

            if !entry_type.is_file() {
                anyhow::bail!("tar archive contains unsupported entry type");
            }

            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create parent directory from tar.gz")?;
            }

            let mut outfile =
                std::fs::File::create(&outpath).context("Failed to create file from tar.gz")?;
            io::copy(&mut entry, &mut outfile).context("Failed to write tar.gz file content")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = entry.header().mode().unwrap_or(0o644);
                if mode & 0o111 != 0 {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                        .context("Failed to set permissions from tar.gz")?;
                }
            }
        }

        Ok(())
    })
    .await
    .context("Task panicked")??;

    Ok(())
}

/// Extract a .zip archive to the specified directory
pub async fn extract_zip(archive: Bytes, dest: &Path) -> Result<()> {
    let dest = dest.to_path_buf();

    tokio::task::spawn_blocking(move || {
        use std::io::Cursor;
        use zip::ZipArchive;

        let cursor = Cursor::new(archive);
        let mut archive = ZipArchive::new(cursor).context("Failed to read zip archive")?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .context("Failed to access zip file entry")?;

            let outpath = safe_zip_entry_path(&dest, file.name())?;

            if file.is_dir() {
                std::fs::create_dir_all(&outpath).context("Failed to create directory from zip")?;
            } else if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).context("Failed to create parent directory")?;
                }

                let mut outfile =
                    std::fs::File::create(&outpath).context("Failed to create file from zip")?;

                std::io::copy(&mut file, &mut outfile).context("Failed to write file content")?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode()
                        && mode & 0o111 != 0
                    {
                        std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                            .context("Failed to set permissions from zip")?;
                    }
                }
            }
        }

        Ok(())
    })
    .await
    .context("Task panicked")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tar::Builder;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    #[test]
    fn safe_zip_entry_path_rejects_traversal() {
        let root = Path::new("/tmp/root");
        assert!(safe_zip_entry_path(root, "../evil.txt").is_err());
        assert!(safe_zip_entry_path(root, "/absolute.txt").is_err());
    }

    #[test]
    fn extract_zip_rejects_unsafe_entries() {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            writer
                .start_file("../evil.txt", SimpleFileOptions::default())
                .expect("start zip file");
            writer.write_all(b"bad").expect("write zip entry");
            writer.finish().expect("finish zip");
        }

        let temp = tempdir().expect("temp dir");
        let result =
            tokio_test::block_on(extract_zip(Bytes::from(buffer.into_inner()), temp.path()));
        assert!(result.is_err());
    }

    #[test]
    fn safe_archive_entry_path_rejects_traversal() {
        let root = Path::new("/tmp/root");
        assert!(safe_archive_entry_path(root, Path::new("../evil.txt")).is_err());
        assert!(safe_archive_entry_path(root, Path::new("/absolute.txt")).is_err());
    }

    #[test]
    fn extract_targz_rejects_symlinks() {
        let mut tar_buffer = Vec::new();
        {
            let encoder =
                flate2::write::GzEncoder::new(&mut tar_buffer, flate2::Compression::default());
            let mut builder = Builder::new(encoder);

            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header.set_cksum();
            builder
                .append_link(&mut header, "agent", "../outside")
                .expect("append symlink");
            builder.finish().expect("finish tar");
        }

        let temp = tempdir().expect("temp dir");
        let result = tokio_test::block_on(extract_targz(Bytes::from(tar_buffer), temp.path()));
        assert!(result.is_err());
    }
}
