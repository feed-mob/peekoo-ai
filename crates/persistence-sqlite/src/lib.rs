pub const MIGRATION_0001_INIT: &str = include_str!("../migrations/0001_init.sql");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_contains_core_tables() {
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE tasks"));
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE pomodoro_sessions"));
        assert!(MIGRATION_0001_INIT.contains("CREATE TABLE calendar_accounts"));
    }
}
