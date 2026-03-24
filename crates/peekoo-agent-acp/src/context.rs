use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub labels: Vec<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub estimated_duration_min: Option<u32>,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
}

impl TaskContext {
    pub fn to_prompt(&self) -> String {
        let mut prompt = String::new();

        prompt.push_str("# Task Assignment\n\n");
        prompt.push_str("You have been assigned the following task:\n\n");
        prompt.push_str(&format!("**Task ID:** `{}`\n", self.task_id));
        prompt.push_str(&format!("**Title:** {}\n", self.title));

        if let Some(desc) = &self.description {
            prompt.push_str(&format!("**Description:** {}\n", desc));
        }

        prompt.push_str(&format!("**Priority:** {}\n", self.priority));
        prompt.push_str(&format!("**Status:** {}\n", self.status));

        if let Some(start) = &self.scheduled_start_at {
            prompt.push_str(&format!("**Scheduled Start:** {}\n", start));
        }

        if let Some(end) = &self.scheduled_end_at {
            prompt.push_str(&format!("**Scheduled End:** {}\n", end));
        }

        if let Some(duration) = &self.estimated_duration_min {
            prompt.push_str(&format!("**Estimated Duration:** {} minutes\n", duration));
        }

        if !self.labels.is_empty() {
            prompt.push_str(&format!("**Labels:** {}\n", self.labels.join(", ")));
        }

        if !self.comments.is_empty() {
            prompt.push_str("\n## Previous Comments\n\n");
            for comment in &self.comments {
                prompt.push_str(&format!(
                    "- **[{}]({})**: {}\n",
                    comment.author, comment.created_at, comment.text
                ));
            }

            if let Some(latest_comment) = self.comments.last() {
                prompt.push_str("\n## Latest follow-up request\n\n");
                prompt.push_str(&format!(
                    "Most recent comment from {} at {}: {}\n\nTreat this as the latest instruction to respond to before older context.\n",
                    latest_comment.author, latest_comment.created_at, latest_comment.text
                ));
            }
        }

        prompt.push_str("\n## Instructions\n\n");
        prompt.push_str("Analyze this task and determine the appropriate action.\n\n");
        prompt.push_str("You must use the available task tools for any user-visible task updates. Do not only describe intended actions in plain text.\n\n");
        prompt.push_str("Available task tools (already scoped to this task):\n");
        prompt.push_str("- `task_comment(text)` to post comments or questions\n");
        prompt.push_str("- `update_task_status(status)` to change the task status\n");
        prompt.push_str("- `update_task_labels(add_labels?, remove_labels?)` to manage labels\n\n");
        prompt.push_str("Execution rules:\n");
        prompt.push_str("1. If the task is clear and can be completed now, do the work and then use task tools to leave a useful comment and update status/labels appropriately.\n");
        prompt.push_str("2. If the task is unclear, use `task_comment` to ask a specific clarifying question. Do not mark it done.\n");
        prompt.push_str("3. If the task needs a plan or partial progress, use `task_comment` to explain the plan and update status to `in_progress` if appropriate.\n");
        prompt.push_str("4. Never claim success without using the task tools to record the outcome on the task itself.\n");
        prompt.push_str("5. Keep comments concise, concrete, and helpful to the user reviewing task history.\n\n");
        prompt.push_str("Valid status values for `update_task_status` are: `pending`, `in_progress`, `done`, `cancelled`.\n");
        prompt.push_str("Example completion sequence: first call `task_comment` with the result, then call `update_task_status` with `done`.\n\n");

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::{Comment, TaskContext};

    #[test]
    fn prompt_highlights_latest_comment_as_follow_up_request() {
        let context = TaskContext {
            task_id: "task-123".into(),
            title: "Tell me a joke".into(),
            description: None,
            status: "todo".into(),
            priority: "medium".into(),
            labels: vec![],
            scheduled_start_at: None,
            scheduled_end_at: None,
            estimated_duration_min: None,
            comments: vec![
                Comment {
                    id: "1".into(),
                    author: "agent".into(),
                    text: "Here is a joke".into(),
                    created_at: "2026-03-24T08:00:00Z".into(),
                },
                Comment {
                    id: "2".into(),
                    author: "user".into(),
                    text: "@peekoo-agent introduce yourself, then tell me what you can do".into(),
                    created_at: "2026-03-24T08:10:00Z".into(),
                },
            ],
        };

        let prompt = context.to_prompt();

        assert!(prompt.contains("Latest follow-up request"));
        assert!(prompt.contains("introduce yourself, then tell me what you can do"));
    }
}
