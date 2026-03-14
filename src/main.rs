mod api;
mod app;
mod auth;
mod ui;

use crate::api::api::UsageResponse;
use app::{App, UsageLine};
use ratatui::crossterm::event::{self, Event, KeyCode};
use std::env;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let headless = env::args().any(|arg| arg == "--headless" || arg == "-h");

    let mut app = App::new();

    let mut creds = match auth::load_credentials() {
        Ok(c) => c,
        Err(e) => {
            if headless {
                eprintln!("Error loading credentials: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return run_tui(app);
        }
    };

    if auth::is_token_expired(&creds.claude_ai_oauth) {
        if let Err(e) = auth::refresh_token(&mut creds.claude_ai_oauth) {
            if headless {
                eprintln!("Error refreshing token: {}", e);
                std::process::exit(1);
            }
            app.error = Some(e);
            app.is_loading = false;
            return run_tui(app);
        }
    }

    match api::fetch_usage(&creds.claude_ai_oauth) {
        Ok(usage) => {
            if headless {
                print_usage_headless(&usage, creds.claude_ai_oauth.subscription_type.as_deref());
                return Ok(());
            }
            app.is_loading = false;
            app.plan = creds.claude_ai_oauth.subscription_type;

            if let Some(session) = usage.five_hour {
                app.usage_lines.push(UsageLine::Progress {
                    label: "Session".to_string(),
                    used: session.utilization,
                    total: 100.0,
                    resets_at: session.resets_at,
                });
            }
            if let Some(weekly) = usage.seven_day {
                app.usage_lines.push(UsageLine::Progress {
                    label: "Weekly".to_string(),
                    used: weekly.utilization,
                    total: 100.0,
                    resets_at: weekly.resets_at,
                });
            }
            if let Some(sonnet) = usage.seven_day_sonnet {
                app.usage_lines.push(UsageLine::Progress {
                    label: "Sonnet".to_string(),
                    used: sonnet.utilization,
                    total: 100.0,
                    resets_at: sonnet.resets_at,
                });
            }
        }
        Err(e) => {
            app.error = Some(e.to_string());
            app.is_loading = false;
        }
    }

    run_tui(app)?;
    Ok(())
}

fn run_tui(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    app.should_quit = true;
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

fn print_usage_headless(usage: &UsageResponse, plan: Option<&str>) {
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
}
