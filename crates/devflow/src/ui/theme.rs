use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub normal:      Style,
    pub dim:         Style,
    pub bold:        Style,
    pub success:     Style,
    pub failure:     Style,
    pub warning:     Style,
    pub info:        Style,
    pub selected:    Style,
    pub tab_active:  Style,
    pub status_bar:  Style,
    pub elite:       Style,
    pub high:        Style,
    pub medium:      Style,
    pub low:         Style,
    pub border:      Style,
    pub border_dim:  Style,
    pub header:      Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            normal:     Style::default(),
            dim:        Style::default().fg(Color::DarkGray),
            bold:       Style::default().add_modifier(Modifier::BOLD),
            success:    Style::default().fg(Color::Green),
            failure:    Style::default().fg(Color::Red),
            warning:    Style::default().fg(Color::Yellow),
            info:       Style::default().fg(Color::Cyan),
            selected:   Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            tab_active: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            status_bar: Style::default().bg(Color::Blue).fg(Color::White),
            elite:      Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            high:       Style::default().fg(Color::Cyan),
            medium:     Style::default().fg(Color::Yellow),
            low:        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            border:     Style::default().fg(Color::Cyan),
            border_dim: Style::default().fg(Color::DarkGray),
            header:     Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        }
    }
}

impl Theme {
    pub fn level_style(&self, level: &devflow_core::types::DoraLevel) -> Style {
        use devflow_core::types::DoraLevel;
        match level {
            DoraLevel::Elite  => self.elite,
            DoraLevel::High   => self.high,
            DoraLevel::Medium => self.medium,
            DoraLevel::Low    => self.low,
            DoraLevel::NoData => self.dim,
        }
    }
}
