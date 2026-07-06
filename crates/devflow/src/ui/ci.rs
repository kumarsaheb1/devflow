use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use super::{sparkline, Theme};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(area);

    // Top: CI pass rate sparkline summary per repo
    if let Some(m) = &app.metrics {
        let rate_style = theme.level_style(&m.ci_pass_rate.level);
        let overall_pct = format!("{:.1}%", m.ci_pass_rate.rate * 100.0);
        let spark = sparkline::render(
            &m.ci_pass_rate.history,
            chunks[0].width.saturating_sub(40) as usize,
            rate_style,
        );

        let summary = Block::default()
            .borders(Borders::ALL)
            .title(" CI Pass Rate ")
            .border_style(theme.border);

        let inner = summary.inner(chunks[0]);
        f.render_widget(summary, chunks[0]);

        let info = ratatui::widgets::Paragraph::new(vec![
            Line::from(vec![
                Span::raw(" Overall: "),
                Span::styled(overall_pct, rate_style.add_modifier(ratatui::style::Modifier::BOLD)),
                Span::styled("  past 90 days", theme.dim),
            ]),
            spark,
        ]);
        f.render_widget(info, inner);
    }

    // Bottom: CI run table
    let ds = match &app.dataset { Some(d) => d, None => return };

    let mut runs: Vec<_> = ds.ci_runs.iter().collect();
    runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let header = Row::new(vec![
        Cell::from("Repo").style(theme.header),
        Cell::from("Workflow").style(theme.header),
        Cell::from("Branch").style(theme.header),
        Cell::from("Result").style(theme.header),
        Cell::from("Duration").style(theme.header),
        Cell::from("Date").style(theme.header),
    ]);

    let rows: Vec<Row> = runs.iter().enumerate().map(|(i, r)| {
        let (sym, style) = match r.conclusion.as_deref() {
            Some("success")  => ("✓ success",  theme.success),
            Some("failure")  => ("✗ failure",  theme.failure),
            Some("timed_out")=> ("⏱ timed_out",theme.warning),
            Some("cancelled")=> ("⊘ cancelled",theme.dim),
            Some(other)      => (other,         theme.dim),
            None             => ("… running",   theme.warning),
        };
        let duration = r.duration_secs
            .map(|s| format!("{}m {:02}s", s / 60, s % 60))
            .unwrap_or_else(|| "—".into());
        let date = r.created_at.format("%m/%d %H:%M").to_string();
        let row_style = if i == app.selected { theme.selected } else { theme.normal };

        Row::new(vec![
            Cell::from(r.repo.clone()),
            Cell::from(r.name.clone()),
            Cell::from(r.branch.clone()),
            Cell::from(sym).style(style),
            Cell::from(duration),
            Cell::from(date),
        ]).style(row_style)
    }).collect();

    let widths = [
        Constraint::Min(16),
        Constraint::Min(20),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(13),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL)
            .title(" CI Runs (recent first) ")
            .border_style(theme.border))
        .row_highlight_style(theme.selected);

    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(table, chunks[1], &mut state);
}
