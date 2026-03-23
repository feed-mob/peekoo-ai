//! Natural language task parsing
//!
//! Parses user input like "Meeting with John tomorrow at 3pm for 1 hour high priority"
//! into structured task fields with fallback to using whole text as title.

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use lazy_static::lazy_static;
use regex::Regex;

/// Parsed task data from natural language input
#[derive(Debug, Default)]
pub struct ParsedTask {
    pub title: String,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub description: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub estimated_duration_min: Option<u32>,
    pub recurrence_rule: Option<String>,
    pub recurrence_time_of_day: Option<String>,
}

lazy_static! {
    // Priority patterns
    static ref PRIORITY_HIGH: Regex = Regex::new(
        r"(?i)\b(high priority|urgent|asap|critical|important)\b"
    ).unwrap();
    static ref PRIORITY_LOW: Regex = Regex::new(
        r"(?i)\b(low priority|whenever|someday|not urgent)\b"
    ).unwrap();

    // Assignee patterns
    static ref ASSIGNEE_AGENT: Regex = Regex::new(
        r"(?i)\b(assign to agent|for agent|@agent)\b"
    ).unwrap();

    // Label patterns
    static ref LABEL_BUG: Regex = Regex::new(r"(?i)\b(bug|fix|issue|error|broken)\b").unwrap();
    static ref LABEL_FEATURE: Regex = Regex::new(r"(?i)\b(feature|add|implement|new|create)\b").unwrap();
    static ref LABEL_URGENT: Regex = Regex::new(r"(?i)\b(urgent|asap|critical|deadline)\b").unwrap();
    static ref LABEL_DESIGN: Regex = Regex::new(r"(?i)\b(design|ui|ux|mockup|figma)\b").unwrap();
    static ref LABEL_DOCS: Regex = Regex::new(r"(?i)\b(doc|document|documentation|readme)\b").unwrap();
    static ref LABEL_REFACTOR: Regex = Regex::new(r"(?i)\b(refactor|cleanup|clean up|rewrite)\b").unwrap();

    // Duration patterns - e.g., "for 30 minutes", "1 hour", "2h"
    static ref DURATION_MINUTES: Regex = Regex::new(
        r"(?i)\bfor\s+(\d+)\s*(min|minute|minutes)\b"
    ).unwrap();
    static ref DURATION_HOURS: Regex = Regex::new(
        r"(?i)\bfor\s+(\d+)\s*(hr|hour|hours)\b"
    ).unwrap();
    static ref DURATION_SHORT: Regex = Regex::new(
        r"(?i)\b(\d+)(h|m)\b"
    ).unwrap();

    // Time patterns - e.g., "at 3pm", "at 14:00", "3:30 PM"
    static ref TIME_12H: Regex = Regex::new(
        r"(?i)\bat\s+(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b"
    ).unwrap();
    static ref TIME_24H: Regex = Regex::new(
        r"(?i)\bat\s+(\d{1,2}):(\d{2})\b"
    ).unwrap();

    // Date patterns
    static ref DATE_TODAY: Regex = Regex::new(r"(?i)\btoday\b").unwrap();
    static ref DATE_TOMORROW: Regex = Regex::new(r"(?i)\btomorrow\b").unwrap();
    static ref DATE_NEXT_WEEK: Regex = Regex::new(r"(?i)\bnext week\b").unwrap();
    static ref DATE_IN_DAYS: Regex = Regex::new(r"(?i)\bin\s+(\d+)\s*days?\b").unwrap();
    static ref DATE_DAY_OF_WEEK: Regex = Regex::new(
        r"(?i)\b(next\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b"
    ).unwrap();

    // Recurrence patterns
    static ref RECUR_DAILY: Regex = Regex::new(r"(?i)\b(every day|daily)\b").unwrap();
    static ref RECUR_WEEKDAYS: Regex = Regex::new(r"(?i)\b(every weekday|weekdays|monday to friday)\b").unwrap();
    static ref RECUR_WEEKLY: Regex = Regex::new(
        r"(?i)\bevery\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b"
    ).unwrap();
}

/// Parse natural language task description into structured fields
pub fn parse_task_text(text: &str) -> ParsedTask {
    let mut parsed = ParsedTask::default();
    let mut remaining = text.to_string();

    // Extract labels (do this first before removing keywords from text)
    if LABEL_BUG.is_match(text) {
        parsed.labels.push("bug".to_string());
    }
    if LABEL_FEATURE.is_match(text) {
        parsed.labels.push("feature".to_string());
    }
    // Also add "urgent" label if priority is high and "urgent" is in the text
    if LABEL_URGENT.is_match(text) {
        parsed.labels.push("urgent".to_string());
    }
    if LABEL_DESIGN.is_match(text) {
        parsed.labels.push("design".to_string());
    }
    if LABEL_DOCS.is_match(text) {
        parsed.labels.push("docs".to_string());
    }
    if LABEL_REFACTOR.is_match(text) {
        parsed.labels.push("refactor".to_string());
    }
    // Remove duplicates
    parsed.labels.sort();
    parsed.labels.dedup();

    // Extract priority
    if PRIORITY_HIGH.is_match(&remaining) {
        parsed.priority = Some("high".to_string());
        remaining = PRIORITY_HIGH.replace(&remaining, "").to_string();
    } else if PRIORITY_LOW.is_match(&remaining) {
        parsed.priority = Some("low".to_string());
        remaining = PRIORITY_LOW.replace(&remaining, "").to_string();
    }

    // Extract assignee
    if ASSIGNEE_AGENT.is_match(&remaining) {
        parsed.assignee = Some("agent".to_string());
        remaining = ASSIGNEE_AGENT.replace(&remaining, "").to_string();
    }

    // Extract duration
    if let Some(caps) = DURATION_MINUTES.captures(&remaining) {
        if let Ok(mins) = caps[1].parse::<u32>() {
            parsed.estimated_duration_min = Some(mins);
            remaining = DURATION_MINUTES.replace(&remaining, "").to_string();
        }
    } else if let Some(caps) = DURATION_HOURS.captures(&remaining) {
        if let Ok(hours) = caps[1].parse::<u32>() {
            parsed.estimated_duration_min = Some(hours * 60);
            remaining = DURATION_HOURS.replace(&remaining, "").to_string();
        }
    } else if let Some(caps) = DURATION_SHORT.captures(&remaining)
        && let Ok(val) = caps[1].parse::<u32>()
    {
        let unit = caps[2].to_lowercase();
        if unit == "h" {
            parsed.estimated_duration_min = Some(val * 60);
        } else {
            parsed.estimated_duration_min = Some(val);
        }
    }

    // Extract time and date
    let (time_h, time_m) = extract_time(&remaining);
    let date = extract_date(&remaining);

    // Combine date and time into scheduled_start_at
    if let Some(date_val) = date {
        let hour = time_h.unwrap_or(9); // Default to 9 AM
        let min = time_m.unwrap_or(0);

        let naive = NaiveDateTime::new(
            date_val,
            NaiveTime::from_hms_opt(hour, min, 0).unwrap_or(NaiveTime::MIN),
        );

        let datetime = Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        parsed.scheduled_start_at = Some(datetime.to_rfc3339());

        // Calculate end time based on duration
        if let Some(duration) = parsed.estimated_duration_min {
            let end_time = datetime + chrono::Duration::minutes(duration as i64);
            parsed.scheduled_end_at = Some(end_time.to_rfc3339());
        }

        // Store time of day for recurring tasks
        parsed.recurrence_time_of_day = Some(format!("{:02}:{:02}", hour, min));
    } else if let Some(hour) = time_h {
        // Time specified but no date - assume today
        let min = time_m.unwrap_or(0);
        let now = Local::now();
        let naive = NaiveDateTime::new(
            now.date_naive(),
            NaiveTime::from_hms_opt(hour, min, 0).unwrap_or(NaiveTime::MIN),
        );
        let datetime = Local
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        parsed.scheduled_start_at = Some(datetime.to_rfc3339());
        parsed.recurrence_time_of_day = Some(format!("{:02}:{:02}", hour, min));
    }

    // Extract recurrence
    if RECUR_DAILY.is_match(text) {
        parsed.recurrence_rule = Some("FREQ=DAILY".to_string());
    } else if RECUR_WEEKDAYS.is_match(text) {
        parsed.recurrence_rule = Some("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR".to_string());
    } else if let Some(caps) = RECUR_WEEKLY.captures(text) {
        let day = &caps[1].to_lowercase();
        let byday = match day.as_str() {
            "monday" => "MO",
            "tuesday" => "TU",
            "wednesday" => "WE",
            "thursday" => "TH",
            "friday" => "FR",
            "saturday" => "SA",
            "sunday" => "SU",
            _ => "MO",
        };
        parsed.recurrence_rule = Some(format!("FREQ=WEEKLY;BYDAY={}", byday));
    }

    // Clean up remaining text for title
    remaining = cleanup_title(&remaining);

    // If after all extraction we have nothing meaningful, use original text
    if remaining.trim().is_empty() {
        parsed.title = text.trim().to_string();
    } else {
        parsed.title = remaining.trim().to_string();
    }

    // Capitalize first letter of title
    if let Some(first) = parsed.title.chars().next() {
        let rest: String = parsed.title.chars().skip(1).collect();
        parsed.title = format!("{}{}", first.to_uppercase(), rest);
    }

    parsed
}

fn extract_time(text: &str) -> (Option<u32>, Option<u32>) {
    // Try 12-hour format first
    if let Some(caps) = TIME_12H.captures(text)
        && let Ok(hour) = caps[1].parse::<u32>()
    {
        let minute: u32 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        let ampm = caps[3].to_lowercase();

        let hour24 = match ampm.as_str() {
            "pm" if hour < 12 => hour + 12,
            "am" if hour == 12 => 0,
            _ => hour,
        };

        return (Some(hour24), Some(minute));
    }

    // Try 24-hour format
    if let Some(caps) = TIME_24H.captures(text)
        && let (Ok(hour), Ok(minute)) = (caps[1].parse::<u32>(), caps[2].parse::<u32>())
    {
        return (Some(hour), Some(minute));
    }

    (None, None)
}

fn extract_date(text: &str) -> Option<NaiveDate> {
    let now = Local::now();
    let today = now.date_naive();

    if DATE_TODAY.is_match(text) {
        return Some(today);
    }

    if DATE_TOMORROW.is_match(text) {
        return today.succ_opt();
    }

    if DATE_NEXT_WEEK.is_match(text) {
        return today.checked_add_signed(chrono::Duration::days(7));
    }

    if let Some(caps) = DATE_IN_DAYS.captures(text)
        && let Ok(days) = caps[1].parse::<i64>()
    {
        return today.checked_add_signed(chrono::Duration::days(days));
    }

    if let Some(caps) = DATE_DAY_OF_WEEK.captures(text) {
        let is_next = caps.get(1).is_some();
        let day_name = &caps[2].to_lowercase();

        let target_weekday = match day_name.as_str() {
            "monday" => chrono::Weekday::Mon,
            "tuesday" => chrono::Weekday::Tue,
            "wednesday" => chrono::Weekday::Wed,
            "thursday" => chrono::Weekday::Thu,
            "friday" => chrono::Weekday::Fri,
            "saturday" => chrono::Weekday::Sat,
            "sunday" => chrono::Weekday::Sun,
            _ => return None,
        };

        let days_until = (target_weekday.num_days_from_monday() as i64
            - today.weekday().num_days_from_monday() as i64
            + 7)
            % 7;

        let days_to_add = if is_next || days_until == 0 {
            days_until + 7
        } else {
            days_until
        };

        return today.checked_add_signed(chrono::Duration::days(days_to_add));
    }

    None
}

fn cleanup_title(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove time patterns
    cleaned = TIME_12H.replace(&cleaned, "").to_string();
    cleaned = TIME_24H.replace(&cleaned, "").to_string();

    // Remove date patterns
    cleaned = DATE_TODAY.replace(&cleaned, "").to_string();
    cleaned = DATE_TOMORROW.replace(&cleaned, "").to_string();
    cleaned = DATE_NEXT_WEEK.replace(&cleaned, "").to_string();
    cleaned = DATE_IN_DAYS.replace(&cleaned, "").to_string();
    cleaned = DATE_DAY_OF_WEEK.replace(&cleaned, "").to_string();

    // Remove recurrence patterns
    cleaned = RECUR_DAILY.replace(&cleaned, "").to_string();
    cleaned = RECUR_WEEKDAYS.replace(&cleaned, "").to_string();
    cleaned = RECUR_WEEKLY.replace(&cleaned, "").to_string();

    // Clean up extra whitespace and punctuation
    cleaned = cleaned
        .replace("  ", " ")
        .replace("at  ", "")
        .replace("for  ", "")
        .replace("  ", " ")
        .trim()
        .trim_matches(|c| c == ',' || c == '.' || c == ' ')
        .to_string();

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_task() {
        let parsed = parse_task_text("Buy groceries");
        assert_eq!(parsed.title, "Buy groceries");
        assert_eq!(parsed.priority, None);
    }

    #[test]
    fn test_parse_with_priority() {
        let parsed = parse_task_text("Fix bug urgent");
        assert_eq!(parsed.title, "Fix bug");
        assert_eq!(parsed.priority, Some("high".to_string()));
        assert!(parsed.labels.contains(&"bug".to_string()));
        assert!(parsed.labels.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_parse_with_time() {
        let parsed = parse_task_text("Meeting at 3pm");
        assert_eq!(parsed.title, "Meeting");
        assert!(parsed.scheduled_start_at.is_some());
        assert_eq!(parsed.recurrence_time_of_day, Some("15:00".to_string()));
    }

    #[test]
    fn test_parse_tomorrow() {
        let parsed = parse_task_text("Call mom tomorrow");
        assert_eq!(parsed.title, "Call mom");
        assert!(parsed.scheduled_start_at.is_some());
    }

    #[test]
    fn test_parse_duration() {
        let parsed = parse_task_text("Meeting for 30 minutes");
        assert_eq!(parsed.title, "Meeting");
        assert_eq!(parsed.estimated_duration_min, Some(30));
    }

    #[test]
    fn test_parse_complex() {
        let parsed = parse_task_text("Review code tomorrow at 2pm for 1 hour high priority");
        assert_eq!(parsed.title, "Review code");
        assert_eq!(parsed.priority, Some("high".to_string()));
        assert_eq!(parsed.estimated_duration_min, Some(60));
        assert!(parsed.scheduled_start_at.is_some());
        assert_eq!(parsed.recurrence_time_of_day, Some("14:00".to_string()));
    }
}
