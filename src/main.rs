mod api;
mod app;
mod auth;
mod ui;

use crate::api::api::UsageResponse as ClaudeUsage;
use crate::api::codex_api::CodexUsageResponse as CodexUsage;
use app::{App, Service, UsageLine};
use ratatui::crossterm::event::{self, Event, KeyCode};
use std::env;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let headless = env::args().any(|arg| arg == "--headless" || arg == "-h");
    let service_arg = env::args().find(|a| a.starts_with("--service="));
    let initial_service = if let Some(s) = service_arg {
        match s.trim_start_matches("--service=") {
            "codex" => Service::Codex,
            _ => Service::Claude,
        }
    } else {
        Service::Claude
    };

    let mut app = App::new().with_service(initial_service);

    match app.active_service {
        Service::Claude => run_claude(&mut app, headless)?,
        Service::Codex => run_codex(&mut app, headless)?,
    }

    if headless {
        return Ok(());
    }

    run_tui(app)?;
    Ok(())
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
    let mut codex_auth = match auth::load_auth() {
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

    if auth::needs_refresh(&codex_auth) {
        if let Err(e) = auth::refresh_codex_token(&mut codex_auth) {
            if headless {
                eprintln!("Error refreshing Codex token: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return Ok(());
        }
    }

    match api::fetch_codex_usage(&codex_auth) {
        Ok((usage, headers)) => {
            if headless {
                print_codex_headless(&usage, &headers);
                return Ok(());
            }
            app.add_codex_usage(&usage, &headers, None);
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
                    KeyCode::Tab => {
                        app.active_service = match app.active_service {
                            Service::Claude => Service::Codex,
                            Service::Codex => Service::Claude,
                        };
                        match app.active_service {
                            Service::Claude => run_claude(&mut app, false)?,
                            Service::Codex => run_codex(&mut app, false)?,
                        }
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

fn print_claude_headless(usage: &ClaudeUsage, plan: Option<&str>) {
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

fn print_codex_headless(usage: &CodexUsage, headers: &api::codex_api::HeaderUsage) {
    println!("━━━ OpenAI Codex Usage ━━━");
    if let Some(ref plan) = usage.plan_type {
        println!("Plan: {}", plan);
    }
    if let Some(s) = headers.session.or_else(|| {
        usage
            .rate_limit
            .as_ref()
            .and_then(|r| r.primary_window.as_ref())
            .and_then(|w| w.used_percent)
    }) {
        println!("Session: {:.1}%", s);
    }
    if let Some(w) = headers.weekly.or_else(|| {
        usage
            .rate_limit
            .as_ref()
            .and_then(|r| r.secondary_window.as_ref())
            .and_then(|w| w.used_percent)
    }) {
        println!("Weekly: {:.1}%", w);
    }
    if let Some(c) = headers
        .credits
        .or_else(|| usage.credits.as_ref().and_then(|c| c.balance))
    {
        println!("Credits: {:.0}", c);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━");
}
