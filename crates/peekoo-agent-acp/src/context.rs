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
