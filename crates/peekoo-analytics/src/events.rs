/// Analytics event names and property builders.
///
/// All event definitions live here so that event names and their expected
/// properties are documented in one place, independent of any analytics
/// provider.
use std::collections::HashMap;

/// Fired on every app launch. Used for active-user and version-distribution
/// metrics.
pub const APP_STARTED: &str = "app_started";

/// Build the standard property map for an [`APP_STARTED`] event.
pub fn app_started_properties(
    version: &str,
    os: &str,
    arch: &str,
) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::with_capacity(3);
    props.insert("app_version".to_string(), serde_json::json!(version));
    props.insert("os".to_string(), serde_json::json!(os));
    props.insert("arch".to_string(), serde_json::json!(arch));
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_started_event_name_is_correct() {
        assert_eq!(APP_STARTED, "app_started");
    }

    #[test]
    fn app_started_properties_include_version() {
        let props = app_started_properties("0.1.21", "macos", "aarch64");
        let version = props.get("app_version").unwrap();
        assert_eq!(version, &serde_json::json!("0.1.21"));
    }

    #[test]
    fn app_started_properties_include_os() {
        let props = app_started_properties("0.1.21", "linux", "x86_64");
        let os = props.get("os").unwrap();
        assert_eq!(os, &serde_json::json!("linux"));
    }

    #[test]
    fn app_started_properties_include_arch() {
        let props = app_started_properties("0.1.21", "linux", "x86_64");
        let arch = props.get("arch").unwrap();
        assert_eq!(arch, &serde_json::json!("x86_64"));
    }

    #[test]
    fn app_started_properties_contain_exactly_three_keys() {
        let props = app_started_properties("0.1.0", "windows", "x86_64");
        assert_eq!(props.len(), 3);
    }
}
