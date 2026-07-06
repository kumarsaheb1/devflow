mod authors;
mod ci;
mod overview;
mod repos;
mod sparkline;
mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, LoadState, Tab};
pub use theme::Theme;

pub fn draw(f: &mut Frame, app: &App) {
    let theme = Theme::default();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(1),    // content
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    // ── Tab bar ───────────────────────────────────────────────────────────────
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.title())).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" devflow — DORA Metrics "))
        .select(app.tab)
        .style(theme.normal)
        .highlight_style(theme.tab_active);
    f.render_widget(tabs, outer[0]);

    // ── Content ───────────────────────────────────────────────────────────────
    match &app.load_state {
        LoadState::Loading => {
            let msg = Paragraph::new("Fetching data from GitHub…  (this may take ~30s for large orgs)")
                .style(theme.dim)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(msg, outer[1]);
        }
        LoadState::Error(e) => {
            let msg = Paragraph::new(format!("Error: {e}\n\nCheck your GITHUB_TOKEN and GITHUB_OWNER."))
                .style(theme.failure)
                .block(Block::default().borders(Borders::ALL).title(" Error "));
            f.render_widget(msg, outer[1]);
        }
        LoadState::Loaded => {
            match Tab::ALL.get(app.tab) {
                Some(Tab::Overview) => overview::draw(f, app, outer[1], &theme),
                Some(Tab::Repos)    => repos::draw(f, app, outer[1], &theme),
                Some(Tab::Authors)  => authors::draw(f, app, outer[1], &theme),
                Some(Tab::CI)       => ci::draw(f, app, outer[1], &theme),
                _ => {}
            }
        }
    }

    // ── Status bar ────────────────────────────────────────────────────────────
    let demo_badge = if app.demo { " [DEMO] " } else { "" };
    let lookback   = format!("{}d window", app.lookback_days);
    let keys       = "Tab/h/l: switch  j/k: scroll  r: refresh  q: quit";
    let bar        = format!(" devflow{demo_badge}  {lookback}  │  {keys}");
    f.render_widget(
        Paragraph::new(bar).style(theme.status_bar),
        outer[2],
    );
}
