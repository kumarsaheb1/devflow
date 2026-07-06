use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use devflow_core::types::DoraMetrics;

use super::{sparkline, Theme};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let Some(m) = &app.metrics else { return; };

    // 2×2 grid of metric cards
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    draw_deploy_freq(f, m, top[0], theme);
    draw_lead_time(f, m, top[1], theme);
    draw_change_failure(f, m, bottom[0], theme);
    draw_mttr(f, m, bottom[1], theme);
}

fn draw_deploy_freq(f: &mut Frame, m: &DoraMetrics, area: Rect, theme: &Theme) {
    use devflow_core::types::DoraLevel;
    let df = &m.deployment_frequency;
    let level_style = theme.level_style(&df.level);

    let freq_label = if df.level == DoraLevel::NoData {
        "No data".into()
    } else if df.per_day >= 1.0 {
        format!("{:.1} / day", df.per_day)
    } else if df.per_day * 7.0 >= 1.0 {
        format!("{:.1} / week", df.per_day * 7.0)
    } else {
        format!("{:.1} / month", df.per_day * 30.0)
    };

    let spark = sparkline::render(&df.history, area.width.saturating_sub(4) as usize, level_style);

    let content = vec![
        Line::from(vec![
            Span::styled("  ", theme.normal),
            Span::styled(freq_label, level_style.add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(""),
        spark,
        Line::from(""),
        Line::from(vec![
            Span::raw("  Level: "),
            Span::styled(df.level.label(), level_style.add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Elite ≥1/day  High ≥1/week  Med ≥1/mo", theme.dim),
        ]),
        Line::from(vec![
            Span::styled("  (using merged PRs to main as proxy)", theme.dim),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🚀 Deployment Frequency ")
        .border_style(theme.border);
    f.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_lead_time(f: &mut Frame, m: &DoraMetrics, area: Rect, theme: &Theme) {
    let lt = &m.lead_time;
    let level_style = theme.level_style(&lt.level);
    let hours_label = format_hours(lt.median_hours);
    let spark = sparkline::render(&lt.history, area.width.saturating_sub(4) as usize, level_style);

    let content = vec![
        Line::from(vec![
            Span::styled("  ", theme.normal),
            Span::styled(hours_label, level_style.add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("  median", theme.dim),
        ]),
        Line::from(""),
        spark,
        Line::from(""),
        Line::from(vec![
            Span::raw("  Level: "),
            Span::styled(lt.level.label(), level_style.add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Elite <1d  High <1w  Med <1mo", theme.dim),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ⏱  Lead Time for Changes ")
        .border_style(theme.border);
    f.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_change_failure(f: &mut Frame, m: &DoraMetrics, area: Rect, theme: &Theme) {
    let cfr = &m.change_failure_rate;
    let level_style = theme.level_style(&cfr.level);
    let pct = format!("{:.1}%", cfr.rate * 100.0);
    let spark = sparkline::render(&cfr.history, area.width.saturating_sub(4) as usize, level_style);

    let content = vec![
        Line::from(vec![
            Span::styled("  ", theme.normal),
            Span::styled(pct, level_style.add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("  of deploys failed", theme.dim),
        ]),
        Line::from(""),
        spark,
        Line::from(""),
        Line::from(vec![
            Span::raw("  Level: "),
            Span::styled(cfr.level.label(), level_style.add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("  Elite <5%  High <10%  Med <15%", theme.dim),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🔥 Change Failure Rate ")
        .border_style(theme.border);
    f.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_mttr(f: &mut Frame, m: &DoraMetrics, area: Rect, theme: &Theme) {
    let mt = &m.mean_time_to_recovery;
    let level_style = theme.level_style(&mt.level);
    let label = if mt.median_hours == 0.0 {
        "No failures".into()
    } else {
        format_hours(mt.median_hours)
    };
    let spark = sparkline::render(&mt.history, area.width.saturating_sub(4) as usize, level_style);

    // Bonus metrics in same card
    let ci_pct   = format!("{:.0}%", m.ci_pass_rate.rate * 100.0);
    let pr_cycle = format_hours(m.pr_cycle_time.median_hours);
    let review   = format_hours(m.review_turnaround.median_hours);
    let pr_size  = format!("{:.0} lines", m.median_pr_size);

    let content = vec![
        Line::from(vec![
            Span::styled("  ", theme.normal),
            Span::styled(label, level_style.add_modifier(ratatui::style::Modifier::BOLD)),
            Span::styled("  to restore", theme.dim),
        ]),
        Line::from(""),
        spark,
        Line::from(""),
        Line::from(vec![
            Span::raw("  Level: "),
            Span::styled(mt.level.label(), level_style.add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("  ─── Bonus Metrics ───────────────", theme.dim)]),
        Line::from(vec![
            Span::styled("  CI pass rate    ", theme.dim),
            Span::styled(ci_pct, theme.level_style(&m.ci_pass_rate.level)),
        ]),
        Line::from(vec![
            Span::styled("  PR cycle time   ", theme.dim),
            Span::styled(pr_cycle, theme.level_style(&m.pr_cycle_time.level)),
        ]),
        Line::from(vec![
            Span::styled("  Review pickup   ", theme.dim),
            Span::styled(review, theme.level_style(&m.review_turnaround.level)),
        ]),
        Line::from(vec![
            Span::styled("  Median PR size  ", theme.dim),
            Span::raw(pr_size),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 🛠  MTTR  +  Bonus ")
        .border_style(theme.border);
    f.render_widget(Paragraph::new(content).block(block), area);
}

pub fn format_hours(h: f64) -> String {
    if h == 0.0            { return "—".into(); }
    if h < 1.0             { return format!("{:.0}m",  h * 60.0); }
    if h < 24.0            { return format!("{:.1}h",  h); }
    if h < 24.0 * 7.0      { return format!("{:.1}d",  h / 24.0); }
    format!("{:.1}w", h / 168.0)
}
