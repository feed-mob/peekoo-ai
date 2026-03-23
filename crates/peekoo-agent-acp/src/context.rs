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
        prompt.push_str("Analyze this task and determine the appropriate action:\n\n");
        prompt.push_str("1. **If the task is clear and you can complete it automatically:**\n");
        prompt.push_str("   - Execute the task using available tools\n");
        prompt.push_str("   - Add a comment summarizing what you did\n");
        prompt.push_str("   - Mark the task as done\n\n");
        prompt.push_str("2. **If the task is unclear or needs more information:**\n");
        prompt.push_str("   - Add a comment with specific questions\n");
        prompt.push_str("   - Do not change the task status\n\n");
        prompt.push_str("3. **If the task is complex and needs a plan:**\n");
        prompt.push_str("   - Add a comment with your proposed approach\n");
        prompt.push_str("   - Break it down into steps if needed\n");
        prompt.push_str("   - Mark status as \"in_progress\"\n");
        prompt.push_str("   - Wait for user feedback before proceeding\n\n");

        prompt
    }
}
