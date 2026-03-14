pub struct App {
    pub is_loading: bool,
    pub error: Option<String>,
    pub plan: Option<String>,
    pub usage_lines: Vec<UsageLine>,
    pub should_quit: bool,
}

pub enum UsageLine {
    Progress {
        label: String,
        used: f64,
        total: f64,
        resets_at: Option<String>,
    },
    Text {
        label: String,
        value: String,
    },
    Badge {
        label: String,
        value: String,
    },
}

impl App {
    pub fn new() -> Self {
        Self {
            is_loading: false,
            error: None,
            plan: None,
            usage_lines: Vec::new(),
            should_quit: false,
        }
    }
}
