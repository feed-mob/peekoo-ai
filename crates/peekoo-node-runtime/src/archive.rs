//! Archive extraction utilities for tar.gz and zip files
//! Replaces util::archive module from Zed

use anyhow::{Context, Result};
use bytes::Bytes;
use std::path::Path;

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

            let outpath = dest.join(file.name());

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
