mod app;
mod demo;
mod ui;

use std::time::Duration;
use anyhow::Context;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, LoadState};

/// Restore the user's terminal (raw mode / alternate screen) before letting
/// a panic proceed, so a crash doesn't leave their shell stuck in a broken
/// state - important once this runs on machines we don't control.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_panic_hook();
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let demo_mode  = std::env::args().any(|a| a == "--demo");
    let debug_mode = std::env::args().any(|a| a == "--debug");

    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = App::new(demo_mode);

    // Kick off background data fetch
    let fetch_tx = app.fetch_tx.clone();
    if demo_mode {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(800)).await;
            let ds = demo::make_dataset();
            let _ = fetch_tx.send(Ok(ds));
        });
    } else {
        let cfg = devflow_core::types::Config::from_env()?;
        if debug_mode {
            eprintln!("[devflow debug] GITHUB_OWNER = {}", cfg.owner);
            eprintln!("[devflow debug] GITHUB_REPOS = {:?}", cfg.repos);
            eprintln!("[devflow debug] lookback     = {} days", cfg.lookback_days);
            eprintln!("[devflow debug] token prefix = {}...", &cfg.token.chars().take(12).collect::<String>());
        }
        tokio::spawn(async move {
            let result = match devflow_core::github::GithubClient::new(&cfg) {
                Ok(client) => client.fetch_all(&cfg).await,
                Err(e)     => Err(e),
            };
            let _ = fetch_tx.send(result);
        });
    }

    let result = run_loop(&mut term, &mut app).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    result
}

async fn run_loop(
    term: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app:  &mut App,
) -> anyhow::Result<()> {
    loop {
        // Poll for completed fetch
        if let Ok(result) = app.fetch_rx.try_recv() {
            match result {
                Ok(ds) => app.on_data_loaded(ds),
                Err(e) => app.load_state = LoadState::Error(e.to_string()),
            }
        }

        term.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match (key.modifiers, key.code) {
                    (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(()),
                    (_, KeyCode::Tab) | (_, KeyCode::Char('l'))  => app.next_tab(),
                    (_, KeyCode::BackTab) | (_, KeyCode::Char('h')) => app.prev_tab(),
                    (_, KeyCode::Down) | (_, KeyCode::Char('j')) => app.move_down(),
                    (_, KeyCode::Up)   | (_, KeyCode::Char('k')) => app.move_up(),
                    (_, KeyCode::Char('r'))                       => app.refresh(),
                    _ => {}
                }
            }
        }
    }
}
