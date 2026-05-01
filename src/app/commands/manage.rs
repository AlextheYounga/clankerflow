mod query;
mod state;

use std::io::{self, Stdout};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use sea_orm::DatabaseConnection;

use crate::app::commands::manage::query::{load_events, load_runs};
use crate::app::commands::manage::state::{RunRow, State};
use crate::core::codebase_id;
use crate::core::project::require_project_root;
use crate::core::tmux;
use crate::db::connection::connect;
struct App<'a> {
    db: &'a DatabaseConnection,
    project_key: &'a str,
    project_session_name: &'a str,
    state: State,
}

enum LoopOutcome {
    Exit,
    Attach { session_name: String, window_name: String },
}

/// # Errors
/// Returns an error if the project is not initialized, terminal setup fails, or
/// database/tmux reads fail while the TUI is running.
pub async fn run() -> Result<()> {
    let project_root = require_project_root()?;
    let project_key = codebase_id::derive(&project_root);
    let db = connect(&project_root).await?;
    let project_session_name = tmux::project_session_name(&project_key);

    let mut app = App {
        db: &db,
        project_key: &project_key,
        project_session_name: &project_session_name,
        state: State::default(),
    };
    refresh(&mut app).await?;

    let mut terminal = init_terminal()?;
    let run_result = run_loop(&mut terminal, &mut app).await;
    let restore_result = restore_terminal(&mut terminal);

    restore_result?;
    match run_result? {
        LoopOutcome::Exit => Ok(()),
        LoopOutcome::Attach { session_name, window_name } => tmux::attach_window(&session_name, &window_name),
    }
}

async fn run_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App<'_>) -> Result<LoopOutcome> {
    let refresh_interval = Duration::from_secs(2);
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|frame| render(frame, &app.state))?;

        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key_event) = event::read()?
            && key_event.kind == KeyEventKind::Press
        {
            match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(LoopOutcome::Exit),
                KeyCode::Up | KeyCode::Char('k') => app.state.move_up(),
                KeyCode::Down | KeyCode::Char('j') => app.state.move_down(),
                KeyCode::Char('r') => refresh(app).await?,
                KeyCode::Char('c') => cancel_selected(&mut app.state)?,
                KeyCode::Enter | KeyCode::Char('a') => {
                    if let Some(window_name) = selected_window_name(&mut app.state) {
                        return Ok(LoopOutcome::Attach {
                            session_name: app.project_session_name.to_string(),
                            window_name,
                        });
                    }
                }
                _ => {}
            }

            load_selected_events(app).await?;
        }

        if last_refresh.elapsed() >= refresh_interval {
            refresh(app).await?;
            last_refresh = Instant::now();
        }
    }
}

async fn refresh(app: &mut App<'_>) -> Result<()> {
    let runs = load_runs(app.db, app.project_key).await?;
    app.state.set_runs(runs);
    load_selected_events(app).await?;

    if app.state.runs.is_empty() {
        app.state.set_status("No clankerflow runs found yet", false);
    } else {
        app.state.set_status(format!("Loaded {} runs", app.state.runs.len()), false);
    }

    Ok(())
}

async fn load_selected_events(app: &mut App<'_>) -> Result<()> {
    let Some(run) = app.state.selected_run() else {
        app.state.events.clear();
        return Ok(());
    };

    app.state.events = load_events(app.db, run.run_id).await?;
    Ok(())
}

fn selected_window_name(state: &mut State) -> Option<String> {
    let run = state.selected_run()?;

    let Some(window_name) = preferred_window(run) else {
        state.set_status("No tmux window available for selected run", true);
        return None;
    };

    Some(window_name.to_string())
}

fn cancel_selected(state: &mut State) -> Result<()> {
    let Some(run) = state.selected_run().cloned() else {
        return Ok(());
    };

    if matches!(run.status.as_str(), "Completed" | "Failed" | "Cancelled") {
        state.set_status("Selected run is already finished", true);
        return Ok(());
    }

    let Some(pid) = run.pid else {
        state.set_status("Selected run has no worker PID", true);
        return Ok(());
    };

    send_sigint(pid)?;
    state.set_status(format!("Sent cancel signal to run {} (pid {pid})", run.run_id), false);
    Ok(())
}

fn preferred_window(run: &RunRow) -> Option<&str> {
    run.tmux_windows
        .iter()
        .find(|window| window.starts_with("opencode__"))
        .map(String::as_str)
        .or_else(|| run.tmux_windows.first().map(String::as_str))
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

fn send_sigint(pid: i64) -> Result<()> {
    let status = Command::new("kill").args(["-INT", &pid.to_string()]).status()?;

    if !status.success() {
        anyhow::bail!("failed to send SIGINT to pid {pid}");
    }

    Ok(())
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn render(frame: &mut ratatui::Frame, state: &State) {
    let regions = split(frame.area());
    render_runs(frame, regions[0], state);
    render_details(frame, regions[1], state);
    render_events(frame, regions[2], state);
    render_help(frame, regions[3], state);
}

fn split(area: Rect) -> [Rect; 4] {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
        .margin(1)
        .split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(34), Constraint::Percentage(34)])
        .split(outer[0]);

    [top[0], top[1], top[2], outer[1]]
}

fn render_runs(frame: &mut ratatui::Frame, area: Rect, state: &State) {
    let items = if state.runs.is_empty() {
        vec![ListItem::new("No runs yet")]
    } else {
        state
            .runs
            .iter()
            .enumerate()
            .map(|(index, run)| {
                let marker = if index == state.selected_index { ">" } else { " " };
                let status_style = status_style(&run.status);
                let line = Line::from(vec![
                    Span::styled(format!("{marker} #{} ", run.run_id), Style::default().fg(Color::DarkGray)),
                    Span::styled(run.workflow_name.clone(), Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(run.status.clone(), status_style),
                ]);
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items).block(panel("Runs"));
    frame.render_widget(list, area);
}

fn render_details(frame: &mut ratatui::Frame, area: Rect, state: &State) {
    let body = if let Some(run) = state.selected_run() {
        let mut lines = vec![
            Line::from(format!("Workflow: {}", run.workflow_name)),
            Line::from(format!("Run ID: {}", run.run_id)),
            Line::from(format!("Status: {}", run.status)),
            Line::from(format!("Env: {}", run.env)),
            Line::from(format!("PID: {}", run.pid.map_or_else(|| "-".to_string(), |pid| pid.to_string()))),
            Line::from(format!("Updated: {}", run.updated_at.format("%Y-%m-%d %H:%M:%S UTC"))),
            Line::from(""),
            Line::from("OpenCode Sessions:"),
        ];

        if run.session_ids.is_empty() {
            lines.push(Line::from("  waiting for session"));
        } else {
            lines.extend(run.session_ids.iter().map(|session_id| Line::from(format!("  {session_id}"))));
        }

        lines.push(Line::from(""));
        lines.push(Line::from("Tmux Windows:"));
        if run.tmux_windows.is_empty() {
            lines.push(Line::from("  missing"));
        } else {
            lines.extend(run.tmux_windows.iter().map(|window| Line::from(format!("  {window}"))));
        }

        lines
    } else {
        vec![Line::from("No run selected")]
    };

    let paragraph = Paragraph::new(body).wrap(Wrap { trim: false }).block(panel("Details"));
    frame.render_widget(paragraph, area);
}

fn render_events(frame: &mut ratatui::Frame, area: Rect, state: &State) {
    let items = if state.events.is_empty() {
        vec![ListItem::new("No events yet")]
    } else {
        state
            .events
            .iter()
            .map(|event| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", event.created_at.format("%H:%M:%S")),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(event.event_type.clone(), Style::default().fg(Color::LightYellow)),
                    Span::raw(" "),
                    Span::raw(event.summary.clone()),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(panel("Events"));
    frame.render_widget(list, area);
}

fn render_help(frame: &mut ratatui::Frame, area: Rect, state: &State) {
    let style = if state.status_is_error {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(Color::Rgb(255, 180, 80))
    };
    let text = Line::from(vec![
        Span::styled("Enter/a", Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(255, 140, 0))),
        Span::raw(" attach  "),
        Span::styled("c", Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(255, 140, 0))),
        Span::raw(" cancel  "),
        Span::styled("r", Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(255, 140, 0))),
        Span::raw(" refresh  "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD).fg(Color::Rgb(255, 140, 0))),
        Span::raw(" quit  "),
        Span::styled(state.status_line.clone(), style),
    ]);

    let paragraph = Paragraph::new(text).block(panel("Clankerflow"));
    frame.render_widget(paragraph, area);
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(255, 140, 0)))
        .title(title)
        .title_style(Style::default().fg(Color::Rgb(255, 190, 110)).add_modifier(Modifier::BOLD))
}

fn status_style(status: &str) -> Style {
    match status {
        "Running" => Style::default().fg(Color::Rgb(255, 170, 0)).add_modifier(Modifier::BOLD),
        "Completed" => Style::default().fg(Color::Gray),
        "Failed" => Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
        "Cancelled" => Style::default().fg(Color::Yellow),
        _ => Style::default().fg(Color::DarkGray),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_window_uses_opencode_window_first() {
        let run = RunRow {
            run_id: 1,
            workflow_name: "duos".to_string(),
            status: "Running".to_string(),
            env: "Host".to_string(),
            pid: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            session_ids: vec!["sess_123".to_string()],
            tmux_windows: vec!["run__1__duos".to_string(), "opencode__1__sess_123".to_string()],
        };

        assert_eq!(preferred_window(&run), Some("opencode__1__sess_123"));
    }

    #[test]
    fn cancel_selected_rejects_finished_runs() {
        let mut state = State {
            runs: vec![RunRow {
                run_id: 1,
                workflow_name: "duos".to_string(),
                status: "Completed".to_string(),
                env: "Host".to_string(),
                pid: Some(123),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                session_ids: Vec::new(),
                tmux_windows: Vec::new(),
            }],
            ..State::default()
        };

        let result = cancel_selected(&mut state);

        assert!(result.is_ok());

        assert!(state.status_is_error);
        assert_eq!(state.status_line, "Selected run is already finished");
    }
}
