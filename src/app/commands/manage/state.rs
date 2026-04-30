use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    pub run_id: i64,
    pub workflow_name: String,
    pub status: String,
    pub env: String,
    pub pid: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub session_ids: Vec<String>,
    pub tmux_windows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRow {
    pub created_at: DateTime<Utc>,
    pub event_type: String,
    pub summary: String,
}

#[derive(Debug, Default)]
pub struct State {
    pub runs: Vec<RunRow>,
    pub selected_index: usize,
    pub events: Vec<EventRow>,
    pub status_line: String,
    pub status_is_error: bool,
}

impl State {
    pub fn set_runs(&mut self, runs: Vec<RunRow>) {
        self.runs = runs;
        if self.runs.is_empty() {
            self.selected_index = 0;
            return;
        }

        self.selected_index = self.selected_index.min(self.runs.len() - 1);
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.runs.len() {
            self.selected_index += 1;
        }
    }

    #[must_use]
    pub fn selected_run(&self) -> Option<&RunRow> {
        self.runs.get(self.selected_index)
    }

    pub fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status_line = message.into();
        self.status_is_error = is_error;
    }
}
