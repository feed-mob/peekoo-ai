use rust_i18n::t;

pub fn set_tray_locale(language: &str) {
    rust_i18n::set_locale(language);
}

pub fn tray_toggle() -> String {
    t!("tray.toggle").to_string()
}

pub fn tray_settings() -> String {
    t!("tray.settings").to_string()
}

pub fn tray_about() -> String {
    t!("tray.about").to_string()
}

pub fn tray_quit() -> String {
    t!("tray.quit").to_string()
}
