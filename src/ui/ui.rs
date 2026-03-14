use crate::app::{App, Service, UsageLine};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Padding, Paragraph},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 20;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(40)])
        .split(area);

    render_sidebar(frame, chunks[0], app);
    render_main(frame, chunks[1], app);
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Services ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let services = Service::all();
    let item_height = 3;
    let constraints: Vec<Constraint> = services
        .iter()
        .map(|_| Constraint::Length(item_height))
        .collect();

    let service_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, service) in services.iter().enumerate() {
        let is_active = *service == app.active_service;
        render_service_item(frame, service_chunks[i], service, is_active);
    }

    let hint_area = Rect {
        x: inner.x,
        y: inner.y + (item_height * 4) as u16 + 1,
        width: inner.width,
        height: inner.height - (item_height * 4) as u16 - 1,
    };

    let hints = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "←/→ Switch",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled("q Quit", Style::default().fg(Color::DarkGray))),
    ]);
    frame.render_widget(hints, hint_area);
}

fn render_service_item(frame: &mut Frame, area: Rect, service: &Service, is_active: bool) {
    let (name, color) = match service {
        Service::Claude => ("Claude AI", Color::Magenta),
        Service::Codex => ("OpenAI Codex", Color::Green),
        Service::Copilot => ("GitHub Copilot", Color::White),
        Service::OpenCode => ("OpenCode", Color::Blue),
    };

    let border_style = if is_active {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let prefix = if is_active { "●" } else { "○" };
    let text = Line::from(vec![
        Span::styled(
            format!(" {} ", prefix),
            Style::default().fg(if is_active { color } else { Color::DarkGray }),
        ),
        Span::styled(
            name,
            Style::default()
                .fg(if is_active { color } else { Color::Gray })
                .add_modifier(if is_active {
                    Modifier::BOLD
                } else {
                    Modifier::ITALIC
                }),
        ),
    ]);

    let para = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(block);
    frame.render_widget(para, area);
}

fn render_main(frame: &mut Frame, area: Rect, app: &App) {
    let (service_name, service_color) = match app.active_service {
        Service::Claude => ("Claude AI", Color::Magenta),
        Service::Codex => ("OpenAI Codex", Color::Green),
        Service::Copilot => ("GitHub Copilot", Color::White),
        Service::OpenCode => ("OpenCode", Color::Blue),
    };

    let plan_str = app.plan.as_deref().unwrap_or("Free");
    let title = format!(" {} Usage — {} ", service_name, plan_str);

    let main_block = Block::default()
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
        ));

    let inner = main_block.inner(area);
    frame.render_widget(main_block, area);

    if app.is_loading {
        let loading = Paragraph::new("Loading usage data...")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(loading, inner);
        return;
    }

    if let Some(ref err) = app.error {
        let error_block = Block::default()
            .title(" Error ")
            .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red))
            .padding(Padding::new(1, 1, 1, 1));

        let error_text = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::LightRed))
            .block(error_block);
        frame.render_widget(error_text, inner);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(inner);

    render_usage_details(frame, main_chunks[0], app, service_color);
    render_graphs(frame, main_chunks[1], app, service_color);
}

fn render_usage_details(frame: &mut Frame, area: Rect, app: &App, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(" Usage Details ")
        .title_style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let usage_items: Vec<_> = app
        .usage_lines
        .iter()
        .filter(|u| !matches!(u, UsageLine::Graph { .. }))
        .collect();

    if usage_items.is_empty() {
        let para = Paragraph::new("No usage data available")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    let item_height = 3;
    let constraints: Vec<Constraint> = usage_items
        .iter()
        .map(|_| Constraint::Length(item_height))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, line) in usage_items.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        render_usage_line(frame, chunks[i], line, color);
    }
}

fn render_usage_line(frame: &mut Frame, area: Rect, line: &UsageLine, color: Color) {
    match line {
        UsageLine::Progress {
            label,
            used,
            total,
            resets_at: _,
        } => {
            let ratio = (used / total).clamp(0.0, 1.0);
            let (bar_color, _) = get_color_for_percentage(ratio * 100.0);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(bar_color))
                .padding(Padding::horizontal(1));

            let gauge = Gauge::default()
                .block(block)
                .gauge_style(Style::default().fg(bar_color).bg(Color::DarkGray))
                .label(format!(" {}: {:.1}% ", label, used))
                .percent((*used as u16).min(100));

            frame.render_widget(gauge, area);
        }

        UsageLine::Text { label, value } => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(color))
                .padding(Padding::horizontal(1));

            let text = Line::from(vec![
                Span::styled(
                    format!("{}: ", label),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(value.as_str(), Style::default().fg(Color::White)),
            ]);

            let para = Paragraph::new(text)
                .alignment(ratatui::layout::Alignment::Center)
                .block(block);
            frame.render_widget(para, area);
        }

        UsageLine::Badge {
            label,
            value,
            color: badge_color,
        } => {
            let badge_color = badge_color
                .and_then(|c| parse_hex_color(c))
                .unwrap_or(color);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(badge_color))
                .padding(Padding::horizontal(1));

            let text = Line::from(vec![
                Span::styled(
                    format!("{}: ", label),
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
                .block(block);
            frame.render_widget(para, area);
        }

        UsageLine::Graph { .. } => {}
    }
}

fn render_graphs(frame: &mut Frame, area: Rect, app: &App, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .title(" Visual Graphs ")
        .title_style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let graph_items: Vec<_> = app
        .usage_lines
        .iter()
        .filter_map(|u| match u {
            UsageLine::Graph { label, percentage } => Some((label.clone(), *percentage)),
            _ => None,
        })
        .collect();

    if graph_items.is_empty() {
        let para = Paragraph::new("No graph data available")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(para, inner);
        return;
    }

    let graph_width = 30;
    let graph_height = 3;
    let available_width = inner.width.saturating_sub(4);
    let available_height = inner.height.saturating_sub(2);

    let cols = (available_width / (graph_width + 2)).max(1) as usize;
    let rows =
        ((graph_items.len() + cols - 1) / cols).min((available_height / graph_height) as usize);

    let col_constraints: Vec<Constraint> = (0..cols)
        .map(|_| Constraint::Length(graph_width + 2))
        .collect();

    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Length(graph_height))
        .collect();

    let graph_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    for (i, (label, percentage)) in graph_items.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;

        if row >= graph_chunks.len() {
            break;
        }

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints.clone())
            .split(graph_chunks[row]);

        render_bar_graph(frame, col_chunks[col], label, *percentage, color);
    }
}

fn render_bar_graph(frame: &mut Frame, area: Rect, label: &str, percentage: f64, _color: Color) {
    let block = Block::default()
        .borders(Borders::NONE)
        .padding(Padding::vertical(0));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let graph_width = (inner.width as f64 * 0.8) as u16;
    let filled = ((percentage / 100.0) * graph_width as f64) as u16;

    let (bar_color, _) = get_color_for_percentage(percentage);

    let label_text = format!("{}: {:.1}%", label, percentage);
    let label_len = label_text.len() as u16;

    let label_area = Rect {
        x: inner.x,
        y: inner.y,
        width: label_len.min(inner.width),
        height: 1,
    };

    let label_para = Paragraph::new(label_text.clone()).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(label_para, label_area);

    let bar_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: graph_width,
        height: 1,
    };

    let filled_str = "█".repeat(filled as usize);
    let empty_str = "░".repeat((graph_width - filled) as usize);

    let bar_text = Line::from(vec![
        Span::styled(filled_str, Style::default().fg(bar_color)),
        Span::styled(empty_str, Style::default().fg(Color::DarkGray)),
    ]);

    let bar_para = Paragraph::new(bar_text);
    frame.render_widget(bar_para, bar_area);
}

fn get_color_for_percentage(pct: f64) -> (Color, Color) {
    match pct {
        p if p >= 90.0 => (Color::LightRed, Color::DarkGray),
        p if p >= 70.0 => (Color::Yellow, Color::DarkGray),
        p if p >= 50.0 => (Color::LightGreen, Color::DarkGray),
        _ => (Color::Green, Color::DarkGray),
    }
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
