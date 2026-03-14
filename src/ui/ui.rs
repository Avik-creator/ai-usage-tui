use crate::app::{App, UsageLine};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(plan_title(app));

    let inner_area = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    if app.is_loading {
        let loading = Paragraph::new("Loading usage data...");
        frame.render_widget(loading, inner_area);
        return;
    }

    if let Some(ref err) = app.error {
        let error_text = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
        frame.render_widget(error_text, inner_area);
        return;
    }

    // one row per usage line + 1 for the quit hint
    let row_count = (app.usage_lines.len() + 1).max(1);
    let constraints: Vec<Constraint> = (0..row_count).map(|_| Constraint::Length(3)).collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (i, line) in app.usage_lines.iter().enumerate() {
        match line {
            UsageLine::Progress {
                label,
                used,
                total,
                resets_at,
            } => {
                let ratio = (used / total).clamp(0.0, 1.0);

                let color = match ratio {
                    r if r >= 0.9 => Color::Red,
                    r if r >= 0.7 => Color::Yellow,
                    _ => Color::Green,
                };

                let title = match resets_at {
                    Some(t) => format!(" {} (resets {}) ", label, t),
                    None => format!(" {} ", label),
                };

                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(title))
                    .gauge_style(Style::default().fg(color))
                    .percent((*used as u16).min(100));

                frame.render_widget(gauge, chunks[i]);
            }

            UsageLine::Text { label, value } => {
                let text = Line::from(vec![
                    Span::styled(
                        format!("{}: ", label),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(value.as_str()),
                ]);
                let para = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
                frame.render_widget(para, chunks[i]);
            }

            UsageLine::Badge { label, value } => {
                let text = Line::from(vec![
                    Span::styled(
                        format!("{}: ", label),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        value.as_str(),
                        Style::default().fg(Color::Black).bg(Color::Gray),
                    ),
                ]);
                let para = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
                frame.render_widget(para, chunks[i]);
            }
        }
    }

    // quit hint
    let last = chunks[app.usage_lines.len()];
    let hint = Paragraph::new(" Press q to quit").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, last);
}

fn plan_title(app: &App) -> String {
    match &app.plan {
        Some(p) => format!(" Claude Usage — {} ", p),
        None => " Claude Usage ".to_string(),
    }
}
