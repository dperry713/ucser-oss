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
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    // Restore terminal
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
            for item in audit_data.iter().rev().take(15) { // Show last 15 items
                let (task, status) = item;
                rows.push(Row::new(vec![task.clone(), status.clone()]));
            }

            let table = Table::new(rows, &[Constraint::Percentage(50), Constraint::Percentage(50)])
                .header(
                    Row::new(vec!["Task ID", "Event Status"])
                        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                )
                .block(Block::default().borders(Borders::ALL).title("Recent Execution Trace"));
            
            f.render_widget(table, chunks[1]);

            let footer = Paragraph::new("Press 'q' to exit | Type `ucser-cli upgrade` to explore Enterprise Security.")
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

fn read_audit_data() -> Vec<(String, String)> {
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
                    let task = json.get("task_id").and_then(|v| v.as_str()).unwrap_or("unknown_task").to_string();
                    let event = json.get("event").and_then(|v| v.as_str()).unwrap_or("unknown_event").to_string();
                    data.push((task, event));
                }
            }
        }
    }
    data
}
