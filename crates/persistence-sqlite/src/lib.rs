pub const MIGRATION_0001_INIT: &str = include_str!("../migrations/0001_init.sql");
pub const MIGRATION_0002_AGENT_SETTINGS: &str =
    include_str!("../migrations/0002_agent_settings.sql");
pub const MIGRATION_0003_PROVIDER_COMPAT: &str =
    include_str!("../migrations/0003_provider_compat.sql");
pub const MIGRATION_0004_GLOBAL_SETTINGS: &str =
    include_str!("../migrations/0004_global_settings.sql");
pub const MIGRATION_0005_PLUGINS: &str = include_str!("../migrations/0005_plugins.sql");
pub const MIGRATION_0005_TASK_EXTENSIONS: &str =
    include_str!("../migrations/0005_task_extensions.sql");
pub const MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE: &str =
    include_str!("../migrations/0006_task_scheduling_and_recurrence.sql");
pub const MIGRATION_0007_RECURRENCE_TIME_OF_DAY: &str =
    include_str!("../migrations/0007_recurrence_time_of_day.sql");
pub const MIGRATION_0008_TASK_ORDER_INDEX: &str =
    include_str!("../migrations/0008_task_order_index.sql");
pub const MIGRATION_0009_AGENT_TASK_ASSIGNMENT: &str =
    include_str!("../migrations/0009_agent_task_assignment.sql");
pub const MIGRATION_0010_POMODORO_RUNTIME: &str =
    include_str!("../migrations/0010_pomodoro_runtime.sql");
pub const MIGRATION_0011_TASK_FINISHED_AT: &str =
    include_str!("../migrations/0011_task_finished_at.sql");

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

    #[test]
    fn migration_contains_provider_config_table() {
        assert!(MIGRATION_0003_PROVIDER_COMPAT.contains("CREATE TABLE agent_provider_configs"));
    }

    #[test]
    fn migration_contains_global_settings_table() {
        assert!(MIGRATION_0004_GLOBAL_SETTINGS.contains("CREATE TABLE"));
        assert!(MIGRATION_0004_GLOBAL_SETTINGS.contains("app_settings"));
    }

    #[test]
    fn migration_contains_task_extensions() {
        assert!(MIGRATION_0005_TASK_EXTENSIONS.contains("assignee"));
        assert!(MIGRATION_0005_TASK_EXTENSIONS.contains("labels_json"));
    }

    #[test]
    fn migration_contains_task_scheduling_and_recurrence() {
        assert!(MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE.contains("scheduled_start_at"));
        assert!(MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE.contains("scheduled_end_at"));
        assert!(MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE.contains("estimated_duration_min"));
        assert!(MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE.contains("recurrence_rule"));
        assert!(MIGRATION_0006_TASK_SCHEDULING_AND_RECURRENCE.contains("parent_task_id"));
    }

    #[test]
    fn migration_contains_task_created_at() {
        assert!(MIGRATION_0008_TASK_ORDER_INDEX.contains("created_at"));
    }

    #[test]
    fn migration_contains_agent_task_assignment() {
        assert!(MIGRATION_0009_AGENT_TASK_ASSIGNMENT.contains("agent_work_status"));
        assert!(MIGRATION_0009_AGENT_TASK_ASSIGNMENT.contains("agent_registry"));
        assert!(MIGRATION_0009_AGENT_TASK_ASSIGNMENT.contains("peekoo-agent"));
        assert!(MIGRATION_0009_AGENT_TASK_ASSIGNMENT.contains("task_planning"));
    }

    #[test]
    fn migration_contains_pomodoro_runtime_tables() {
        assert!(
            MIGRATION_0010_POMODORO_RUNTIME.contains("CREATE TABLE IF NOT EXISTS pomodoro_state")
        );
        assert!(
            MIGRATION_0010_POMODORO_RUNTIME
                .contains("CREATE TABLE IF NOT EXISTS pomodoro_cycle_history")
        );
        assert!(MIGRATION_0010_POMODORO_RUNTIME.contains("INSERT OR IGNORE INTO pomodoro_state"));
    }

    #[test]
    fn migration_contains_task_finished_at() {
        assert!(MIGRATION_0011_TASK_FINISHED_AT.contains("ALTER TABLE tasks ADD COLUMN finished_at"));
        assert!(MIGRATION_0011_TASK_FINISHED_AT.contains("SET finished_at = updated_at"));
    }
}
