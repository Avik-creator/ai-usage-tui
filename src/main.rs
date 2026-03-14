mod api;
mod app;
mod auth;
mod ui;

use app::{App, Service};
use ratatui::crossterm::event::{self, Event, KeyCode};
use std::env;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let headless = env::args().any(|arg| arg == "--headless" || arg == "-h");
    let initial_service = parse_service_arg();

    let mut app = App::new().with_service(initial_service);

    match app.active_service {
        Service::Claude => run_claude(&mut app, headless)?,
        Service::Codex => run_codex(&mut app, headless)?,
        Service::Copilot => run_copilot(&mut app, headless)?,
        Service::OpenCode => run_opencode(&mut app, headless)?,
    }

    if headless {
        return Ok(());
    }

    run_tui(app)?;
    Ok(())
}

fn parse_service_arg() -> Service {
    env::args()
        .find(|a| a.starts_with("--service="))
        .map(|s| match s.trim_start_matches("--service=") {
            "claude" => Service::Claude,
            "codex" => Service::Codex,
            "copilot" => Service::Copilot,
            "opencode" => Service::OpenCode,
            _ => Service::Claude,
        })
        .unwrap_or(Service::Claude)
}

fn run_claude(app: &mut App, headless: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut creds = match auth::load_credentials() {
        Ok(c) => c,
        Err(e) => {
            if headless {
                eprintln!("Error loading Claude credentials: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return Ok(());
        }
    };

    if auth::is_token_expired(&creds.claude_ai_oauth) {
        if let Err(e) = auth::refresh_token(&mut creds.claude_ai_oauth) {
            if headless {
                eprintln!("Error refreshing Claude token: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return Ok(());
        }
    }

    match api::fetch_usage(&creds.claude_ai_oauth) {
        Ok(usage) => {
            if headless {
                print_claude_headless(&usage, creds.claude_ai_oauth.subscription_type.as_deref());
                return Ok(());
            }
            let plan = creds.claude_ai_oauth.subscription_type.clone();
            app.add_claude_usage(
                usage.five_hour.map(|u| u.utilization),
                usage.seven_day.map(|u| u.utilization),
                usage.seven_day_sonnet.map(|u| u.utilization),
                plan,
            );
        }
        Err(e) => {
            if headless {
                eprintln!("Error fetching Claude usage: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e.to_string());
            app.is_loading = false;
        }
    }

    Ok(())
}

fn run_codex(app: &mut App, headless: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut codex_auth = match auth::load_codex_auth() {
        Ok(a) => a,
        Err(e) => {
            if headless {
                eprintln!("Error loading Codex credentials: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return Ok(());
        }
    };

    // Try to refresh
    let _ = auth::refresh_codex_token(&mut codex_auth);

    match api::fetch_codex_usage(&codex_auth) {
        Ok((usage, headers)) => {
            if headless {
                print_codex_headless(&usage, &headers);
                return Ok(());
            }
            app.add_codex_usage(&usage, &headers);
        }
        Err(e) => {
            if headless {
                eprintln!("Error fetching Codex usage: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e.to_string());
            app.is_loading = false;
        }
    }

    Ok(())
}

fn run_copilot(app: &mut App, headless: bool) -> Result<(), Box<dyn std::error::Error>> {
    let copilot_auth = match auth::load_copilot_auth() {
        Ok(a) => a,
        Err(e) => {
            if headless {
                eprintln!("Error loading Copilot credentials: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return Ok(());
        }
    };

    match auth::fetch_copilot_usage(&copilot_auth) {
        Ok(usage) => {
            if headless {
                print_copilot_headless(&usage);
                return Ok(());
            }
            app.add_copilot_usage(&usage);
        }
        Err(e) => {
            if headless {
                eprintln!("Error fetching Copilot usage: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e.to_string());
            app.is_loading = false;
        }
    }

    Ok(())
}

fn run_opencode(app: &mut App, headless: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Try to load auth first
    let auth_loaded = auth::load_opencode_auth().is_ok();

    // Try running opencode stats
    let output = std::process::Command::new("opencode")
        .args(["stats", "--days", "30"])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if headless {
                    println!("━━━ OpenCode Usage ━━━");
                    println!("{}", stdout);
                    println!("━━━━━━━━━━━━━━━━━━━━");
                    return Ok(());
                }
                app.add_opencode_usage_from_output(&stdout);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if headless {
                    eprintln!("Error running opencode: {}", stderr);
                    std::process::exit(1);
                }
                if !auth_loaded {
                    app.error = Some("OpenCode not logged in".to_string());
                } else {
                    app.error = Some(format!("OpenCode error: {}", stderr));
                }
                app.is_loading = false;
            }
        }
        Err(e) => {
            if headless {
                eprintln!("OpenCode CLI not found: {}", e);
                std::process::exit(1);
            }
            app.error = Some("OpenCode CLI not found".to_string());
            app.is_loading = false;
        }
    }

    Ok(())
}

fn run_tui(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab | KeyCode::Right => {
                        app.active_service = app.active_service.next();
                        reload_service(&mut app)?;
                    }
                    KeyCode::Left => {
                        app.active_service = app.active_service.prev();
                        reload_service(&mut app)?;
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn reload_service(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    app.usage_lines.clear();
    app.error = None;
    app.is_loading = true;
    app.plan = None;

    match app.active_service {
        Service::Claude => run_claude(app, false)?,
        Service::Codex => run_codex(app, false)?,
        Service::Copilot => run_copilot(app, false)?,
        Service::OpenCode => run_opencode(app, false)?,
    }

    Ok(())
}

fn print_claude_headless(usage: &crate::api::api::UsageResponse, plan: Option<&str>) {
    println!("━━━ Claude AI Usage ━━━");
    if let Some(p) = plan {
        println!("Plan: {}", p);
    }
    if let Some(session) = &usage.five_hour {
        println!("Session: {:.1}%", session.utilization);
    }
    if let Some(weekly) = &usage.seven_day {
        println!("Weekly: {:.1}%", weekly.utilization);
    }
    if let Some(sonnet) = &usage.seven_day_sonnet {
        println!("Sonnet: {:.1}%", sonnet.utilization);
    }
    println!("━━━━━━━━━━━━━━━━━━━━");
}

fn print_codex_headless(
    usage: &crate::api::codex_api::CodexUsageResponse,
    headers: &crate::api::codex_api::HeaderUsage,
) {
    println!("━━━ OpenAI Codex Usage ━━━");
    if let Some(ref plan) = usage.plan_type {
        println!("Plan: {}", plan);
    }
    if let Some(s) = headers.session {
        println!("Session: {:.1}%", s);
    }
    if let Some(w) = headers.weekly {
        println!("Weekly: {:.1}%", w);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━");
}

fn print_copilot_headless(usage: &crate::auth::copilot_auth::CopilotUsageResponse) {
    println!("━━━ GitHub Copilot Usage ━━━");
    if let Some(ref plan) = usage.copilot_plan {
        println!("Plan: {}", plan);
    }
    if let Some(quota) = &usage.quota_snapshots {
        if let Some(chat) = &quota.chat {
            if let Some(pct) = chat.percent_remaining {
                println!("Chat: {:.0}% remaining", pct);
            }
        }
        if let Some(premium) = &quota.premium_interactions {
            if let Some(pct) = premium.percent_remaining {
                println!("Premium: {:.0}% remaining", pct);
            }
        }
    }
    if let (Some(lq), Some(mq)) = (&usage.limited_user_quotas, &usage.monthly_quotas) {
        if let (Some(remaining), Some(limit)) = (lq.chat, mq.chat) {
            println!("Chat (Free): {} / {} left", remaining, limit);
        }
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━");
}
