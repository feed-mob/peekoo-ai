//! Process execution helpers for plugins.
//!
//! Requires the `process:exec` permission.

use extism_pdk::{Error, Json};

use crate::host_fns::{ProcessExecRequest, peekoo_process_exec};

#[derive(Debug, Clone)]
pub struct ProcessExecResult {
    pub ok: bool,
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a host process with optional working directory.
///
/// `cwd` is resolved by the host and must stay under the plugin directory.
pub fn exec(program: &str, args: &[String], cwd: Option<&str>) -> Result<ProcessExecResult, Error> {
    let response = unsafe {
        peekoo_process_exec(Json(ProcessExecRequest {
            program: program.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(ToString::to_string),
        }))?
    };

    Ok(ProcessExecResult {
        ok: response.0.ok,
        status_code: response.0.status_code,
        stdout: response.0.stdout,
        stderr: response.0.stderr,
    })
}
