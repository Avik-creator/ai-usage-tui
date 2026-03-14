use crate::api::codex_api::{CodexUsageResponse, HeaderUsage as CodexHeader};
use crate::auth::copilot_auth::CopilotUsageResponse as CopilotUsage;
use crate::auth::opencode_auth::OpenCodeAuth;

pub struct App {
    pub is_loading: bool,
    pub error: Option<String>,
    pub plan: Option<String>,
    pub usage_lines: Vec<UsageLine>,
    pub should_quit: bool,
    pub active_service: Service,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Claude,
    Codex,
    Copilot,
    OpenCode,
}

impl Service {
    pub fn all() -> [Self; 4] {
        [Self::Claude, Self::Codex, Self::Copilot, Self::OpenCode]
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Claude => Self::Codex,
            Self::Codex => Self::Copilot,
            Self::Copilot => Self::OpenCode,
            Self::OpenCode => Self::Claude,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Claude => Self::OpenCode,
            Self::Codex => Self::Claude,
            Self::Copilot => Self::Codex,
            Self::OpenCode => Self::Copilot,
        }
    }
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
    Graph {
        label: String,
        percentage: f64,
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
            self.usage_lines.push(UsageLine::Graph {
                label: "Session".to_string(),
                percentage: s,
            });
        }
        if let Some(w) = weekly {
            self.usage_lines.push(UsageLine::Progress {
                label: "Weekly".to_string(),
                used: w,
                total: 100.0,
                resets_at: None,
            });
            self.usage_lines.push(UsageLine::Graph {
                label: "Weekly".to_string(),
                percentage: w,
            });
        }
        if let Some(s) = sonnet {
            self.usage_lines.push(UsageLine::Progress {
                label: "Sonnet".to_string(),
                used: s,
                total: 100.0,
                resets_at: None,
            });
            self.usage_lines.push(UsageLine::Graph {
                label: "Sonnet".to_string(),
                percentage: s,
            });
        }
    }

    pub fn add_codex_usage(&mut self, data: &CodexUsageResponse, headers: &CodexHeader) {
        self.is_loading = false;
        self.plan = data.plan_type.clone();

        if let Some(s) = headers.session.or_else(|| {
            data.rate_limit
                .as_ref()
                .and_then(|r| r.primary_window.as_ref())
                .and_then(|w| w.as_data())
                .and_then(|d| d.used_percent)
        }) {
            self.usage_lines.push(UsageLine::Progress {
                label: "Session".to_string(),
                used: s,
                total: 100.0,
                resets_at: None,
            });
            self.usage_lines.push(UsageLine::Graph {
                label: "Session".to_string(),
                percentage: s,
            });
        }

        if let Some(w) = headers.weekly.or_else(|| {
            data.rate_limit
                .as_ref()
                .and_then(|r| r.secondary_window.as_ref())
                .and_then(|sw| sw.as_data())
                .and_then(|d| d.used_percent)
        }) {
            self.usage_lines.push(UsageLine::Progress {
                label: "Weekly".to_string(),
                used: w,
                total: 100.0,
                resets_at: None,
            });
            self.usage_lines.push(UsageLine::Graph {
                label: "Weekly".to_string(),
                percentage: w,
            });
        }

        if self.usage_lines.is_empty() {
            self.usage_lines.push(UsageLine::Badge {
                label: "Status".to_string(),
                value: "No usage data".to_string(),
                color: Some("#a3a3a3"),
            });
        }
    }

    pub fn add_copilot_usage(&mut self, data: &CopilotUsage) {
        self.is_loading = false;
        self.plan = data.copilot_plan.clone();

        // Paid tier
        if let Some(quota) = &data.quota_snapshots {
            if let Some(chat) = &quota.chat {
                if let Some(pct) = chat.percent_remaining {
                    let used = 100.0 - pct.min(100.0);
                    self.usage_lines.push(UsageLine::Progress {
                        label: "Chat".to_string(),
                        used,
                        total: 100.0,
                        resets_at: data.quota_reset_date.clone(),
                    });
                    self.usage_lines.push(UsageLine::Graph {
                        label: "Chat".to_string(),
                        percentage: used,
                    });
                }
            }
            if let Some(premium) = &quota.premium_interactions {
                if let Some(pct) = premium.percent_remaining {
                    let used = 100.0 - pct.min(100.0);
                    self.usage_lines.push(UsageLine::Progress {
                        label: "Premium".to_string(),
                        used,
                        total: 100.0,
                        resets_at: data.quota_reset_date.clone(),
                    });
                    self.usage_lines.push(UsageLine::Graph {
                        label: "Premium".to_string(),
                        percentage: used,
                    });
                }
            }
        }

        // Free tier
        if let (Some(lq), Some(mq)) = (&data.limited_user_quotas, &data.monthly_quotas) {
            if let (Some(chat_remaining), Some(chat_limit)) = (lq.chat, mq.chat) {
                let used = ((chat_limit - chat_remaining) as f64 / chat_limit as f64) * 100.0;
                self.usage_lines.push(UsageLine::Progress {
                    label: "Chat (Free)".to_string(),
                    used,
                    total: 100.0,
                    resets_at: data.limited_user_reset_date.clone(),
                });
                self.usage_lines.push(UsageLine::Graph {
                    label: "Chat (Free)".to_string(),
                    percentage: used,
                });
                self.usage_lines.push(UsageLine::Text {
                    label: "Chat Left".to_string(),
                    value: format!("{}", chat_remaining),
                });
            }
            if let (Some(comp_remaining), Some(comp_limit)) = (lq.completions, mq.completions) {
                let used = ((comp_limit - comp_remaining) as f64 / comp_limit as f64) * 100.0;
                self.usage_lines.push(UsageLine::Progress {
                    label: "Completions (Free)".to_string(),
                    used,
                    total: 100.0,
                    resets_at: data.limited_user_reset_date.clone(),
                });
                self.usage_lines.push(UsageLine::Graph {
                    label: "Completions (Free)".to_string(),
                    percentage: used,
                });
                self.usage_lines.push(UsageLine::Text {
                    label: "Completions Left".to_string(),
                    value: format!("{}", comp_remaining),
                });
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

    pub fn add_opencode_usage(&mut self, _auth: &OpenCodeAuth) {
        self.is_loading = false;

        self.usage_lines.push(UsageLine::Badge {
            label: "Status".to_string(),
            value: "Use CLI".to_string(),
            color: Some("#8b5cf6"),
        });
        self.usage_lines.push(UsageLine::Text {
            label: "Command".to_string(),
            value: "opencode stats".to_string(),
        });
    }

    pub fn add_opencode_usage_from_output(&mut self, output: &str) {
        self.is_loading = false;

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Try to parse lines like "Chat: 45,000 tokens" or "$1.23 spent"
            if line.contains(':') {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let label = parts[0].trim().to_string();
                    let value = parts[1].trim().to_string();

                    // Check if it's a percentage or has a number
                    if value.contains('%') {
                        if let Ok(pct) = value.replace('%', "").trim().parse::<f64>() {
                            self.usage_lines.push(UsageLine::Progress {
                                label: label.clone(),
                                used: pct,
                                total: 100.0,
                                resets_at: None,
                            });
                            self.usage_lines.push(UsageLine::Graph {
                                label,
                                percentage: pct,
                            });
                            continue;
                        }
                    }

                    self.usage_lines.push(UsageLine::Text { label, value });
                }
            } else {
                self.usage_lines.push(UsageLine::Text {
                    label: "Info".to_string(),
                    value: line.to_string(),
                });
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
