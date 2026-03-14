use crate::app::{App, Service, UsageLine};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Padding, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title_alignment(ratatui::layout::Alignment::Center);

    let inner_area = main_block.inner(area);
    frame.render_widget(main_block, area);

    let header = create_header(app);
    let header_height = 3;
    let footer_height = 1;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(0),
            Constraint::Length(footer_height),
        ])
        .split(inner_area);

    frame.render_widget(header, chunks[0]);

    if app.is_loading {
        let loading = Paragraph::new("Loading usage data...")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(loading, chunks[1]);
        return;
    }

    if let Some(ref err) = app.error {
        let error_block = Block::default()
            .title(" Error ")
            .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red))
            .padding(Padding::horizontal(1));

        let error_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::LightRed))
            .block(error_block);
        frame.render_widget(error_text, chunks[1]);
        return;
    }

    let row_count = app.usage_lines.len().max(1);
    let constraints: Vec<Constraint> = (0..row_count).map(|_| Constraint::Length(5)).collect();

    let usage_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(chunks[1]);

    for (i, line) in app.usage_lines.iter().enumerate() {
        if i >= usage_chunks.len() {
            break;
        }
        match line {
            UsageLine::Progress {
                label,
                used,
                total,
                resets_at: _,
            } => {
                let ratio = (used / total).clamp(0.0, 1.0);

                let (color, bg_color) = match ratio {
                    r if r >= 0.9 => (Color::LightRed, Color::DarkGray),
                    r if r >= 0.7 => (Color::Yellow, Color::DarkGray),
                    r if r >= 0.5 => (Color::LightGreen, Color::DarkGray),
                    _ => (Color::Green, Color::DarkGray),
                };

                let progress_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(color))
                    .padding(Padding::horizontal(1));

                let percent_text = format!("{:.1}%", used);

                let gauge = Gauge::default()
                    .block(progress_block)
                    .gauge_style(Style::default().fg(color).bg(bg_color))
                    .label(format!(" {} ", label))
                    .percent((*used as u16).min(100));

                frame.render_widget(gauge, usage_chunks[i]);

                let percent_span = Span::styled(
                    percent_text,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
                let percent_width = 6;
                let gauge_end = usage_chunks[i].right() - 2;
                let percent_area = ratatui::layout::Rect {
                    x: gauge_end.saturating_sub(percent_width),
                    y: usage_chunks[i].y + 1,
                    width: percent_width,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(Line::from(percent_span))
                        .alignment(ratatui::layout::Alignment::Center),
                    percent_area,
                );
            }

            UsageLine::Text { label, value } => {
                let text_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue))
                    .padding(Padding::horizontal(1));

                let text = Line::from(vec![
                    Span::styled(
                        format!("{} ", label),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(value.as_str(), Style::default().fg(Color::White)),
                ]);
                let para = Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(text_block);
                frame.render_widget(para, usage_chunks[i]);
            }

            UsageLine::Badge {
                label,
                value,
                color,
            } => {
                let badge_color = color
                    .and_then(|c| parse_hex_color(c))
                    .unwrap_or(Color::LightBlue);

                let badge_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(badge_color))
                    .padding(Padding::horizontal(1));

                let text = Line::from(vec![
                    Span::styled(
                        format!("{} ", label),
                        Style::default()
                            .fg(badge_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        value.as_str(),
                        Style::default().fg(Color::White).bg(badge_color),
                    ),
                ]);
                let para = Paragraph::new(text)
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(badge_block);
                frame.render_widget(para, usage_chunks[i]);
            }
        }
    }

    let hint = Paragraph::new(" Press q to quit · Tab to switch service ")
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[2]);
}

fn create_header(app: &App) -> Block<'static> {
    let (service_name, service_color) = match app.active_service {
        Service::Claude => ("Claude AI", Color::Magenta),
        Service::Codex => ("OpenAI Codex", Color::Green),
    };

    let plan_str = app.plan.as_deref().unwrap_or("Free");
    let title = format!(" {} Usage — {} ", service_name, plan_str);

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(service_color))
        .title_alignment(ratatui::layout::Alignment::Center)
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .bg(service_color)
                .add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::vertical(0))
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if hex.starts_with('#') && hex.len() == 7 {
        let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
        let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
        let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}
