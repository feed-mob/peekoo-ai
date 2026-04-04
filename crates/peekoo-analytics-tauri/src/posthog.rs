use tauri::{AppHandle, Runtime, plugin::TauriPlugin};
use tauri_plugin_posthog::{CaptureRequest, PostHogConfig, PostHogExt};

/// Build the Tauri PostHog plugin with the current compile-time environment
/// configuration. Returns a disabled client when analytics are not configured.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let config = peekoo_analytics::posthog::config_from_env();
    let (api_key, api_host) = match &config {
        Some(c) => (c.api_key().to_string(), c.api_host().to_string()),
        None => (String::new(), String::new()),
    };

    tauri_plugin_posthog::init(PostHogConfig {
        api_key,
        api_host,
        ..Default::default()
    })
}

/// Convert a core analytics payload into the Tauri PostHog request type.
pub fn capture_request(capture: &peekoo_analytics::posthog::PostHogCapture) -> CaptureRequest {
    CaptureRequest {
        event: capture.event().to_string(),
        properties: Some(capture.properties().clone()),
        distinct_id: None,
        groups: None,
        timestamp: None,
        anonymous: false,
    }
}

/// Fire the standard app startup event if PostHog is configured.
pub async fn capture_app_started<R: Runtime>(
    app: &AppHandle<R>,
    version: &str,
    os: &str,
    arch: &str,
) {
    if peekoo_analytics::posthog::config_from_env().is_none() {
        return;
    }

    let capture = peekoo_analytics::posthog::app_started_capture(version, os, arch);
    let _ = app.posthog().capture(capture_request(&capture)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_started_request_uses_capture_event_and_properties() {
        let capture = peekoo_analytics::posthog::app_started_capture("0.1.21", "linux", "x86_64");
        let request = capture_request(&capture);

        assert_eq!(request.event, peekoo_analytics::events::APP_STARTED);
        assert_eq!(
            request
                .properties
                .as_ref()
                .map(std::collections::HashMap::len),
            Some(3)
        );
        assert!(request.distinct_id.is_none());
        assert!(request.groups.is_none());
        assert!(request.timestamp.is_none());
        assert!(!request.anonymous);
    }
}
