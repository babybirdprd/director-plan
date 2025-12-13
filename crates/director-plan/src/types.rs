use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticket {
    pub meta: Meta,
    pub spec: Spec,
    pub verification: Verification,
    #[serde(default)]
    pub history: History,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    #[serde(rename = "type")]
    pub ticket_type: Option<TicketType>,
    pub owner: Option<String>,
    #[serde(default = "default_created_at")]
    pub created_at: toml_datetime::Datetime,
}

fn default_created_at() -> toml_datetime::Datetime {
    // Current time approximation or default
    // Converting chrono to toml_datetime is annoying, so for now just return a default
    // or we can use chrono with serde support in toml, but toml_edit uses toml_datetime.
    // Let's rely on the file having it, or a static default.
    let d = toml_datetime::Date { year: 2024, month: 1, day: 1 };
    let t = toml_datetime::Time { hour: 0, minute: 0, second: 0, nanosecond: 0 };
    toml_datetime::Datetime { date: Some(d), time: Some(t), offset: None }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Todo,
    InProgress,
    Review,
    Done,
    Archived,
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Todo => "todo".to_string(),
            Status::InProgress => "in_progress".to_string(),
            Status::Review => "review".to_string(),
            Status::Done => "done".to_string(),
            Status::Archived => "archived".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TicketType {
    Feature,
    Bug,
    Chore,
    Spike,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Spec {
    pub description: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub relevant_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Verification {
    pub command: String,
    pub golden_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct History {
    #[serde(default)]
    pub log: Vec<String>,
}

// For List output
#[derive(Debug, Serialize)]
pub struct TicketSummary {
    pub id: String,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
}
