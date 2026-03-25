use peekoo_task_domain::TaskStatus;

use crate::{TaskDto, TaskEventDto};

pub trait TaskService: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<u32>,
        recurrence_rule: Option<&str>,
        recurrence_time_of_day: Option<&str>,
    ) -> Result<TaskDto, String>;
    fn list_tasks(&self) -> Result<Vec<TaskDto>, String>;
    #[allow(clippy::too_many_arguments)]
    fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        labels: Option<&[String]>,
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<Option<u32>>,
        recurrence_rule: Option<Option<&str>>,
        recurrence_time_of_day: Option<Option<&str>>,
    ) -> Result<TaskDto, String>;
    fn delete_task(&self, id: &str) -> Result<(), String>;
    fn toggle_task(&self, id: &str) -> Result<TaskDto, String>;
    fn get_task_activity(&self, task_id: &str, limit: u32) -> Result<Vec<TaskEventDto>, String>;
    fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String>;
    fn claim_task_for_agent(&self, task_id: &str) -> Result<bool, String>;
    fn update_agent_work_status(
        &self,
        task_id: &str,
        status: &str,
        session_id: Option<&str>,
    ) -> Result<(), String>;
    fn increment_attempt_count(&self, task_id: &str) -> Result<u32, String>;
    fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String>;
    fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String>;
    fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String>;
    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String>;
    fn load_task(&self, task_id: &str) -> Result<TaskDto, String>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopTaskService;

impl TaskService for NoopTaskService {
    fn create_task(
        &self,
        _title: &str,
        _priority: &str,
        _assignee: &str,
        _labels: &[String],
        _description: Option<&str>,
        _scheduled_start_at: Option<&str>,
        _scheduled_end_at: Option<&str>,
        _estimated_duration_min: Option<u32>,
        _recurrence_rule: Option<&str>,
        _recurrence_time_of_day: Option<&str>,
    ) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        Ok(vec![])
    }

    fn update_task(
        &self,
        _id: &str,
        _title: Option<&str>,
        _priority: Option<&str>,
        _status: Option<&str>,
        _assignee: Option<&str>,
        _labels: Option<&[String]>,
        _description: Option<&str>,
        _scheduled_start_at: Option<&str>,
        _scheduled_end_at: Option<&str>,
        _estimated_duration_min: Option<Option<u32>>,
        _recurrence_rule: Option<Option<&str>>,
        _recurrence_time_of_day: Option<Option<&str>>,
    ) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn delete_task(&self, _id: &str) -> Result<(), String> {
        Ok(())
    }

    fn toggle_task(&self, _id: &str) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn get_task_activity(&self, _task_id: &str, _limit: u32) -> Result<Vec<TaskEventDto>, String> {
        Ok(vec![])
    }

    fn add_task_comment(
        &self,
        _task_id: &str,
        _text: &str,
        _author: &str,
    ) -> Result<TaskEventDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn claim_task_for_agent(&self, _task_id: &str) -> Result<bool, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn update_agent_work_status(
        &self,
        _task_id: &str,
        _status: &str,
        _session_id: Option<&str>,
    ) -> Result<(), String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn increment_attempt_count(&self, _task_id: &str) -> Result<u32, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
        Ok(vec![])
    }

    fn add_task_label(&self, _task_id: &str, _label: &str) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn remove_task_label(&self, _task_id: &str, _label: &str) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn update_task_status(&self, _task_id: &str, _status: TaskStatus) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }

    fn load_task(&self, _task_id: &str) -> Result<TaskDto, String> {
        Err("NoopTaskService: not implemented".into())
    }
}
