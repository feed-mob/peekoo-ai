pub const SHARED_PYTHON_SDK_ROOT: &str = "~/.peekoo/python-sdk";

pub fn shared_python_candidates() -> Vec<String> {
    vec![
        format!("{SHARED_PYTHON_SDK_ROOT}/python/bin/python3"),
        format!("{SHARED_PYTHON_SDK_ROOT}/python/bin/python"),
        format!("{SHARED_PYTHON_SDK_ROOT}/python/python.exe"),
        format!("{SHARED_PYTHON_SDK_ROOT}/python/bin/python.exe"),
    ]
}

pub fn plugin_local_python_candidates() -> Vec<String> {
    vec![
        "runtime/python/bin/python3".to_string(),
        "runtime/python/bin/python".to_string(),
        "runtime/python/python.exe".to_string(),
        "runtime/python/bin/python.exe".to_string(),
    ]
}

pub fn system_python_candidates() -> Vec<String> {
    vec![
        "python3".to_string(),
        "python".to_string(),
        "py".to_string(),
    ]
}

pub fn all_python_candidates() -> Vec<String> {
    let mut out = Vec::new();
    out.extend(shared_python_candidates());
    out.extend(plugin_local_python_candidates());
    out.extend(system_python_candidates());
    out
}

pub fn is_spawn_error_message(message: &str) -> bool {
    message.contains("Process spawn failed")
}
