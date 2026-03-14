mod api;
mod app;
mod auth;
mod ui;

use app::{App, UsageLine};
use ratatui::crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let mut app = App::new();

    // step 1: load credentials
    let mut creds = match auth::load_credentials() {
        Ok(c) => c,
        Err(e) => {
            app.error = Some(e);
            app.is_loading = false;
            return run_tui(app);
        }
    };

    // step 2: refresh token if expired
    if auth::is_token_expired(&creds.claude_ai_oauth) {
        if let Err(e) = auth::refresh_token(&mut creds.claude_ai_oauth) {
            app.error = Some(e);
            app.is_loading = false;
            return run_tui(app);
        }
    }

    // step 3:
    // step 3: fetch usage
    match api::fetch_usage(&creds.claude_ai_oauth) {
        Ok(usage) => {
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

    // ALWAYS restore or your terminal breaks
    ratatui::restore();
    Ok(())
}
