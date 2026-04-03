use std::path::PathBuf;

pub fn peekoo_global_config_dir() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        let base =
            dirs::config_dir().ok_or_else(|| "Cannot determine config directory".to_string())?;
        return Ok(base.join("Peekoo").join("peekoo"));
    }

    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".peekoo"))
}

pub fn peekoo_global_data_dir() -> Result<PathBuf, String> {
    if cfg!(target_os = "windows") {
        let base = dirs::data_local_dir()
            .ok_or_else(|| "Cannot determine local data directory".to_string())?;
        return Ok(base.join("Peekoo").join("peekoo"));
    }

    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".peekoo"))
}

pub fn peekoo_legacy_home_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".peekoo"))
}

/// Returns the legacy home dir only when it differs from the current
/// `peekoo_global_config_dir()`. On non-Windows this returns `None`
/// because both resolve to `~/.peekoo`, avoiding duplicate directory scans.
pub fn peekoo_legacy_home_dir_if_distinct() -> Option<PathBuf> {
    let legacy = peekoo_legacy_home_dir()?;
    let primary = peekoo_global_config_dir().ok()?;
    if legacy == primary {
        None
    } else {
        Some(legacy)
    }
}

/// Returns legacy candidate paths for pi `models.json` that existed before
/// the platform-aware migration. Used for copy-if-missing migration only.
pub fn pi_legacy_models_paths() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    vec![
        home.join(".pi").join("models.json"),
        home.join(".pi").join("agent").join("models.json"),
    ]
}

pub fn peekoo_settings_db_path() -> Result<PathBuf, String> {
    Ok(peekoo_global_data_dir()?.join("peekoo.sqlite"))
}

pub fn peekoo_global_skills_dir() -> Result<PathBuf, String> {
    Ok(peekoo_global_config_dir()?.join("skills"))
}

pub fn peekoo_global_cache_dir() -> Result<PathBuf, String> {
    let data_dir = peekoo_global_data_dir()?;
    Ok(data_dir.join("cache"))
}

pub fn peekoo_log_dir() -> Result<PathBuf, String> {
    Ok(peekoo_global_data_dir()?.join("logs"))
}

pub fn pi_agent_dir() -> Result<PathBuf, String> {
    if let Some(override_dir) = std::env::var_os("PI_CODING_AGENT_DIR") {
        return Ok(PathBuf::from(override_dir));
    }

    if cfg!(target_os = "windows") {
        let base = dirs::data_local_dir()
            .ok_or_else(|| "Cannot determine local data directory".to_string())?;
        return Ok(base.join("Peekoo").join("pi").join("agent"));
    }

    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".pi").join("agent"))
}

pub fn pi_models_path() -> Result<PathBuf, String> {
    Ok(pi_agent_dir()?.join("models.json"))
}

pub fn ensure_windows_pi_agent_env() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if std::env::var_os("PI_CODING_AGENT_DIR").is_none() {
            let dir = pi_agent_dir()?;
            // SAFETY: called by app bootstrap on startup before threads are spawned.
            unsafe { std::env::set_var("PI_CODING_AGENT_DIR", dir) };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn pi_models_path_is_under_pi_agent_dir() {
        let dir = pi_agent_dir().expect("pi agent dir");
        let models = pi_models_path().expect("pi models path");
        assert_eq!(models, dir.join("models.json"));
    }

    #[test]
    fn peekoo_skills_path_is_under_config_dir() {
        let cfg = peekoo_global_config_dir().expect("peekoo config dir");
        let skills = peekoo_global_skills_dir().expect("peekoo skills dir");
        assert_eq!(skills, cfg.join("skills"));
    }

    #[test]
    fn settings_db_path_is_inside_data_dir() {
        let data = peekoo_global_data_dir().expect("peekoo data dir");
        let db = peekoo_settings_db_path().expect("peekoo settings db path");
        assert_eq!(db, data.join("peekoo.sqlite"));
    }

    #[test]
    fn log_dir_is_inside_data_dir() {
        let data = peekoo_global_data_dir().expect("peekoo data dir");
        let logs = peekoo_log_dir().expect("peekoo log dir");
        assert_eq!(logs, data.join("logs"));
    }

    #[test]
    fn legacy_home_dir_points_to_dot_peekoo() {
        if let Some(path) = peekoo_legacy_home_dir() {
            assert_eq!(path.file_name(), Some(std::ffi::OsStr::new(".peekoo")));
        }
    }

    #[test]
    fn windows_pi_env_function_is_safe_to_call() {
        let result = ensure_windows_pi_agent_env();
        assert!(result.is_ok());
    }

    #[test]
    fn pi_agent_dir_honors_env_override() {
        let original = std::env::var_os("PI_CODING_AGENT_DIR");
        let expected = std::env::temp_dir().join("peekoo-paths-env-override");

        // SAFETY: test process controls env access for this assertion.
        unsafe { std::env::set_var("PI_CODING_AGENT_DIR", &expected) };
        let resolved = pi_agent_dir().expect("pi agent dir with override");
        assert_eq!(resolved, expected);

        match original {
            Some(value) => {
                // SAFETY: restoring original env var value for test isolation.
                unsafe { std::env::set_var("PI_CODING_AGENT_DIR", value) };
            }
            None => {
                // SAFETY: restoring env state for test isolation.
                unsafe { std::env::remove_var("PI_CODING_AGENT_DIR") };
            }
        }
    }

    #[test]
    fn legacy_home_dir_if_distinct_returns_none_when_same_as_primary() {
        if cfg!(windows) {
            // On Windows they should differ, so legacy_if_distinct returns Some.
            assert!(peekoo_legacy_home_dir_if_distinct().is_some());
            return;
        }
        // On non-Windows both resolve to ~/.peekoo, so should be None.
        assert!(peekoo_legacy_home_dir_if_distinct().is_none());
    }

    #[test]
    fn pi_legacy_models_paths_returns_two_candidates() {
        let paths = pi_legacy_models_paths();
        // If we can resolve home_dir, expect 2 candidates; otherwise 0.
        if dirs::home_dir().is_some() {
            assert_eq!(paths.len(), 2);
            assert!(paths[0].ends_with(PathBuf::from(".pi").join("models.json")));
            assert!(paths[1].ends_with(PathBuf::from(".pi").join("agent").join("models.json")));
        } else {
            assert!(paths.is_empty());
        }
    }

    #[test]
    fn non_windows_pi_default_contains_dot_pi_agent() {
        if cfg!(windows) {
            return;
        }
        // Clear env override so we test the default path logic.
        let original = std::env::var_os("PI_CODING_AGENT_DIR");
        if original.is_some() {
            // SAFETY: test process controls env access for this assertion.
            unsafe { std::env::remove_var("PI_CODING_AGENT_DIR") };
        }

        let dir = pi_agent_dir().expect("pi agent dir");
        assert!(dir.to_string_lossy().contains(".pi"));
        assert!(dir.ends_with(PathBuf::from(".pi").join("agent")));

        // Restore original value for test isolation.
        if let Some(value) = original {
            // SAFETY: restoring original env var value for test isolation.
            unsafe { std::env::set_var("PI_CODING_AGENT_DIR", value) };
        }
    }
}
