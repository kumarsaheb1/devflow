use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use super::{overview::format_hours, Theme};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let header = Row::new(vec![
        Cell::from("Author").style(theme.header),
        Cell::from("PRs").style(theme.header),
        Cell::from("Merged").style(theme.header),
        Cell::from("Merge %").style(theme.header),
        Cell::from("Avg Cycle").style(theme.header),
        Cell::from("Avg PR Size").style(theme.header),
    ]);

    let rows: Vec<Row> = app.author_stats.iter().enumerate().map(|(i, a)| {
        let merge_pct = if a.pr_count == 0 { 0.0 }
                        else { a.merged_count as f64 / a.pr_count as f64 * 100.0 };
        let style = if i == app.selected { theme.selected } else { theme.normal };

        Row::new(vec![
            Cell::from(a.author.clone()),
            Cell::from(a.pr_count.to_string()),
            Cell::from(a.merged_count.to_string()),
            Cell::from(format!("{merge_pct:.0}%")),
            Cell::from(format_hours(a.avg_cycle_hours)),
            Cell::from(format!("{:.0} lines", a.avg_pr_size)),
        ]).style(style)
    }).collect();

    let widths = [
        ratatui::layout::Constraint::Min(16),
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Length(9),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(13),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL)
            .title(" Authors — j/k to scroll ")
            .border_style(theme.border))
        .row_highlight_style(theme.selected);

    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}
