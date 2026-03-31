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

/// Extract a .tar.gz archive to the specified directory
pub async fn extract_targz(archive: Bytes, dest: &Path) -> Result<()> {
    let dest = dest.to_path_buf();

    // Use spawn_blocking for synchronous tar extraction
    tokio::task::spawn_blocking(move || {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        let cursor = Cursor::new(archive);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        archive
            .unpack(&dest)
            .context("Failed to extract tar.gz archive")
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
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)
                            .context("Failed to create parent directory")?;
                    }
                }

                let mut outfile =
                    std::fs::File::create(&outpath).context("Failed to create file from zip")?;

                std::io::copy(&mut file, &mut outfile).context("Failed to write file content")?;
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
}
