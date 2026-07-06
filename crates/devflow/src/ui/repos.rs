use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use super::{overview::format_hours, Theme};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let header = Row::new(vec![
        Cell::from("Repo").style(theme.header),
        Cell::from("PRs").style(theme.header),
        Cell::from("Merged").style(theme.header),
        Cell::from("Avg Cycle").style(theme.header),
        Cell::from("Avg Review").style(theme.header),
        Cell::from("CI Pass").style(theme.header),
        Cell::from("Deploys").style(theme.header),
    ]);

    let rows: Vec<Row> = app.repo_stats.iter().enumerate().map(|(i, r)| {
        let ci_pct   = format!("{:.0}%", r.ci_pass_rate * 100.0);
        let ci_style = if r.ci_pass_rate >= 0.95 { theme.success }
                       else if r.ci_pass_rate >= 0.80 { theme.warning }
                       else { theme.failure };
        let style = if i == app.selected { theme.selected } else { theme.normal };

        Row::new(vec![
            Cell::from(r.repo.clone()),
            Cell::from(r.pr_count.to_string()),
            Cell::from(r.merged_count.to_string()),
            Cell::from(format_hours(r.avg_cycle_hours)),
            Cell::from(format_hours(r.avg_review_hours)),
            Cell::from(ci_pct).style(ci_style),
            Cell::from(r.deploy_count.to_string()),
        ]).style(style)
    }).collect();

    let widths = [
        ratatui::layout::Constraint::Min(20),
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(13),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Length(9),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL)
            .title(" Repos — j/k to scroll ")
            .border_style(theme.border))
        .row_highlight_style(theme.selected);

    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}
