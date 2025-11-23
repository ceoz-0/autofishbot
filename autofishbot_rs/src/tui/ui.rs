use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Span, Line},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph, Tabs, Gauge},
    Frame,
};
use crate::tui::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Header/Tabs
                Constraint::Min(0),    // Main Content
                Constraint::Length(1), // Status Bar
            ]
            .as_ref(),
        )
        .split(f.area());

    draw_header(f, app, chunks[0]);

    // Main Content
    match app.tab_index {
        0 => draw_dashboard(f, app, chunks[1]),
        1 => draw_profile(f, app, chunks[1]),
        2 => draw_logs(f, app, chunks[1]),
        3 => draw_config(f, app, chunks[1]),
        _ => {},
    }

    draw_status_bar(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app.tabs.iter().enumerate().map(|(i, t)| {
        let style = if i == app.tab_index {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Line::from(Span::styled(t, style))
    }).collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Autofishbot RS "))
        .select(app.tab_index)
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_style = if app.is_running {
        Style::default().fg(Color::Black).bg(Color::Green)
    } else {
        Style::default().fg(Color::White).bg(Color::Red)
    };

    let status_text = format!(" STATUS: {} | Q: Quit | TAB: Switch Tab | S: Start/Stop ", app.status);
    let status_bar = Paragraph::new(status_text)
        .style(status_style);
    f.render_widget(status_bar, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[0]);

    // Stats
    let stats_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Statistics ")
        .style(Style::default().fg(Color::Magenta));

    let stats_text = vec![
        Line::from(vec![Span::styled("Fish Caught: ", Style::default().fg(Color::Blue)), Span::raw(app.stats.fish_caught.to_string())]),
        Line::from(vec![Span::styled("Money Earned: ", Style::default().fg(Color::Yellow)), Span::raw(format!("${}", app.stats.money_earned))]),
        Line::from(vec![Span::styled("Captchas:    ", Style::default().fg(Color::Red)), Span::raw(app.stats.captchas_solved.to_string())]),
        Line::from(vec![Span::styled("Runtime:     ", Style::default().fg(Color::Green)), Span::raw(&app.stats.runtime)]),
    ];
    let stats_p = Paragraph::new(stats_text).block(stats_block).style(Style::default().fg(Color::White));
    f.render_widget(stats_p, left_chunks[0]);

    // Last Message
    let msg_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Last Message ");
    let msg_p = Paragraph::new(app.last_message.clone())
        .block(msg_block)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .style(Style::default().fg(Color::Gray));
    f.render_widget(msg_p, left_chunks[1]);

    // Recent Logs (Right side)
    let logs_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Recent Activity ");
    let logs: Vec<ListItem> = app.logs.iter().rev().take(20).map(|l| {
        ListItem::new(Line::from(vec![
            Span::styled(">> ", Style::default().fg(Color::Blue)),
            Span::raw(l.clone()),
        ]))
    }).collect();
    let logs_list = List::new(logs).block(logs_block);
    f.render_widget(logs_list, chunks[1]);
}

fn draw_profile(f: &mut Frame, app: &App, area: Rect) {
     let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

     // Left Side: General Info & Inventory
     let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
        .split(chunks[0]);

     let info_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Player Info ");
     let info_text = vec![
         Line::from(vec![Span::styled("Balance: ", Style::default().fg(Color::Yellow)), Span::raw(&app.profile.balance)]),
         Line::from(vec![Span::styled("Level:   ", Style::default().fg(Color::Cyan)), Span::raw(&app.profile.level)]),
         Line::from(vec![Span::styled("Biome:   ", Style::default().fg(Color::Green)), Span::raw(&app.profile.biome)]),
         Line::from(vec![Span::styled("Rod:     ", Style::default().fg(Color::Magenta)), Span::raw(&app.profile.rod)]),
         Line::from(vec![Span::styled("Pet:     ", Style::default().fg(Color::Blue)), Span::raw(&app.profile.pet)]),
         Line::from(vec![Span::styled("Bait:    ", Style::default().fg(Color::Red)), Span::raw(&app.profile.bait)]),
     ];
     f.render_widget(Paragraph::new(info_text).block(info_block), left_chunks[0]);

     let inv_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Inventory ");
     let inv_items: Vec<ListItem> = app.profile.inventory.iter().map(|(amt, name)| {
         ListItem::new(format!("{} x {}", amt, name))
     }).collect();
     f.render_widget(List::new(inv_items).block(inv_block), left_chunks[1]);

     // Right Side: Quests & Stats
     let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

     let quest_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Quests ");
     let quest_items: Vec<ListItem> = app.profile.quests.iter().map(|q| {
         let style = if q.is_completed { Style::default().fg(Color::Green) } else { Style::default() };
         ListItem::new(Line::from(vec![
             Span::styled(format!("[{}] ", if q.is_completed { "X" } else { " " }), style),
             Span::raw(format!("{} - {}", q.category, q.progress)),
         ]))
     }).collect();
     f.render_widget(List::new(quest_items).block(quest_block), right_chunks[0]);

     let charms_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Charms ");
     let charms_text = vec![
         Line::from(format!("Marketing: {}", app.profile.charms.marketing)),
         Line::from(format!("Endurance: {}", app.profile.charms.endurance)),
         Line::from(format!("Haste:     {}", app.profile.charms.haste)),
         Line::from(format!("Treasure:  {}", app.profile.charms.treasure)),
     ];
     f.render_widget(Paragraph::new(charms_text).block(charms_block), right_chunks[1]);
}

fn draw_logs(f: &mut Frame, app: &App, area: Rect) {
    let logs_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Logs ");
    let logs: Vec<ListItem> = app.logs.iter().map(|l| ListItem::new(l.clone())).collect();
    let logs_list = List::new(logs).block(logs_block);
    f.render_widget(logs_list, area);
}

fn draw_config(f: &mut Frame, app: &App, area: Rect) {
     let config_block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(" Configuration ");

     let token_masked = if app.config.system.user_token.len() > 5 {
         format!("{}...", &app.config.system.user_token[0..5])
     } else {
         "Not Set".to_string()
     };

     let text = vec![
         Line::from(vec![Span::styled("User Token: ", Style::default().fg(Color::Yellow)), Span::raw(token_masked)]),
         Line::from(vec![Span::styled("Channel ID: ", Style::default().fg(Color::Yellow)), Span::raw(app.config.system.channel_id.to_string())]),
         Line::from(vec![Span::styled("More Fish:  ", Style::default().fg(Color::Green)), Span::raw(app.config.automation.more_fish.to_string())]),
         Line::from(vec![Span::styled("Auto Daily: ", Style::default().fg(Color::Green)), Span::raw(app.config.automation.auto_daily.to_string())]),
     ];
     let p = Paragraph::new(text).block(config_block);
     f.render_widget(p, area);
}
