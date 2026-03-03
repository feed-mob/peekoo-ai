pub const MIGRATION_0001_INIT: &str = include_str!("../migrations/0001_init.sql");
pub const MIGRATION_0002_AGENT_SETTINGS: &str =
    include_str!("../migrations/0002_agent_settings.sql");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_contains_core_tables() {
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE tasks"));
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE pomodoro_sessions"));
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE calendar_accounts"));
    }

    #[test]
    fn migration_contains_agent_settings_tables() {
        assert!(MIGRATION_0002_AGENT_SETTINGS.contains("CREATE TABLE agent_settings"));
        assert!(MIGRATION_0002_AGENT_SETTINGS.contains("CREATE TABLE agent_provider_auth"));
        assert!(MIGRATION_0002_AGENT_SETTINGS.contains("CREATE TABLE agent_skills"));
    }
}
