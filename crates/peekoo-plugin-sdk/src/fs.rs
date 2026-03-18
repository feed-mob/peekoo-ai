//! Filesystem helpers for plugins.
//!
//! Requires the `fs:read` permission.

use extism_pdk::{Error, Json};

use crate::host_fns::{peekoo_fs_read, peekoo_fs_read_dir, FsReadDirRequest, FsReadRequest};
use crate::types::FsEntry;

/// Read a file from an allowed path.
pub fn read(path: &str) -> Result<Option<String>, Error> {
    read_with_options(path, None)
}

/// Read the last `tail_bytes` bytes from a file.
pub fn read_tail(path: &str, tail_bytes: u64) -> Result<Option<String>, Error> {
    read_with_options(path, Some(tail_bytes))
}

/// List directory entries from an allowed path.
pub fn read_dir(path: &str) -> Result<Vec<FsEntry>, Error> {
    let response = unsafe {
        peekoo_fs_read_dir(Json(FsReadDirRequest {
            path: path.to_string(),
        }))?
    };
    Ok(response.0.entries)
}

fn read_with_options(path: &str, tail_bytes: Option<u64>) -> Result<Option<String>, Error> {
    let response = unsafe {
        peekoo_fs_read(Json(FsReadRequest {
            path: path.to_string(),
            tail_bytes,
        }))?
    };
    Ok(response.0.content)
}
