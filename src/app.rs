use crate::api::codex_api::{CodexUsageResponse, HeaderUsage};

pub struct App {
    pub is_loading: bool,
    pub error: Option<String>,
    pub plan: Option<String>,
    pub usage_lines: Vec<UsageLine>,
    pub should_quit: bool,
    pub active_service: Service,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Service {
    Claude,
    Codex,
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
        color: Option<&'static str>,
    },
}

impl App {
    pub fn new() -> Self {
        Self {
            is_loading: true,
            error: None,
            plan: None,
            usage_lines: Vec::new(),
            should_quit: false,
            active_service: Service::Claude,
        }
    }

    pub fn with_service(mut self, service: Service) -> Self {
        self.active_service = service;
        self
    }

    pub fn add_claude_usage(
        &mut self,
        session: Option<f64>,
        weekly: Option<f64>,
        sonnet: Option<f64>,
        plan: Option<String>,
    ) {
        self.is_loading = false;
        self.plan = plan;

        if let Some(s) = session {
            self.usage_lines.push(UsageLine::Progress {
                label: "Session".to_string(),
                used: s,
                total: 100.0,
                resets_at: None,
            });
        }
        if let Some(w) = weekly {
            self.usage_lines.push(UsageLine::Progress {
                label: "Weekly".to_string(),
                used: w,
                total: 100.0,
                resets_at: None,
            });
        }
        if let Some(s) = sonnet {
            self.usage_lines.push(UsageLine::Progress {
                label: "Sonnet".to_string(),
                used: s,
                total: 100.0,
                resets_at: None,
            });
        }
    }

    pub fn add_codex_usage(
        &mut self,
        data: &CodexUsageResponse,
        headers: &HeaderUsage,
        plan: Option<String>,
    ) {
        self.is_loading = false;
        self.plan = plan.or(data.plan_type.clone());

        if let Some(s) = headers.session.or_else(|| {
            data.rate_limit
                .as_ref()
                .and_then(|r| r.primary_window.as_ref())
                .and_then(|w| w.used_percent)
        }) {
            self.usage_lines.push(UsageLine::Progress {
                label: "Session".to_string(),
                used: s,
                total: 100.0,
                resets_at: None,
            });
        }

        if let Some(w) = headers.weekly.or_else(|| {
            data.rate_limit
                .as_ref()
                .and_then(|r| r.secondary_window.as_ref())
                .and_then(|w| w.used_percent)
        }) {
            self.usage_lines.push(UsageLine::Progress {
                label: "Weekly".to_string(),
                used: w,
                total: 100.0,
                resets_at: None,
            });
        }

        if let Some(c) = headers
            .credits
            .or_else(|| data.credits.as_ref().and_then(|c| c.balance))
        {
            self.usage_lines.push(UsageLine::Text {
                label: "Credits".to_string(),
                value: format!("{:.0}", c),
            });
        }

        if let Some(additional) = &data.additional_rate_limits {
            for entry in additional {
                if let Some(name) = &entry.limit_name {
                    if let Some(rl) = &entry.rate_limit {
                        if let Some(p) = rl.primary_window.as_ref().and_then(|w| w.used_percent) {
                            let short_name = name.replace("GPT-", "").replace("Codex-", "");
                            self.usage_lines.push(UsageLine::Progress {
                                label: short_name,
                                used: p,
                                total: 100.0,
                                resets_at: None,
                            });
                        }
                    }
                }
            }
        }

        if self.usage_lines.is_empty() {
            self.usage_lines.push(UsageLine::Badge {
                label: "Status".to_string(),
                value: "No usage data".to_string(),
                color: Some("#a3a3a3"),
            });
        }
    }
}
