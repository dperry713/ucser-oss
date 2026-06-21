use std::{io, time::Duration, fs, path::Path};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};

pub fn run_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let title = Paragraph::new("🚀 UCSER Live Dashboard (OSS Edition)")
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(title, chunks[0]);

            let audit_data = read_audit_data();
            let mut rows = Vec::new();
            for item in audit_data.iter().rev().take(15) {
                let (task, event, result, latency) = item;
                
                let status_style = match result.as_str() {
                    "success" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    "failure" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    "blocked" => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    _ => match event.as_str() {
                        "started" => Style::default().fg(Color::Yellow),
                        "routed" => Style::default().fg(Color::Blue),
                        _ => Style::default().fg(Color::White),
                    }
                };

                let display_status = if result.is_empty() { 
                    event.clone() 
                } else { 
                    format!("{} ({})", event, result) 
                };

                let latency_display = if latency == "0" && result == "blocked" {
                    "N/A".to_string()
                } else {
                    format!("{} ms", latency)
                };
                
                rows.push(Row::new(vec![
                    ratatui::text::Span::raw(task.clone()),
                    ratatui::text::Span::styled(display_status, status_style),
                    ratatui::text::Span::raw(latency_display),
                ]));
            }

            let table = Table::new(rows, &[
                Constraint::Percentage(40), 
                Constraint::Percentage(40), 
                Constraint::Percentage(20)
            ])
            .header(
                Row::new(vec!["Task ID", "Status / Event", "Latency"])
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().borders(Borders::ALL).title("Recent Execution Trace"));
            
            f.render_widget(table, chunks[1]);

            let footer = Paragraph::new("Press 'q' to exit | Type `ucser-cli replay` to verify or `ucser-cli export` to compliance export.")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
    }
}

fn read_audit_data() -> Vec<(String, String, String, String)> {
    use std::io::BufRead;
    let mut data = Vec::new();
    let file_path = Path::new("audit.ndjson");
    let fallback_path = Path::new("logs/audit.ndjson");
    
    let file = fs::File::open(file_path).or_else(|_| fs::File::open(fallback_path));
    
    if let Ok(f) = file {
        let reader = std::io::BufReader::new(f);
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&l) {
                    let task = json.get("task_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if task.is_empty() {
                        continue;
                    }
                    let event = json.get("event").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let result = json.get("result").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let latency = json.get("latency_ms").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
                    data.push((task, event, result, latency));
                }
            }
        }
    }
    data
}
