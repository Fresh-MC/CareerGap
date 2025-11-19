/// Policy details screen with BEFORE/AFTER diff viewer
use nogap_core::types::Policy;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Clone)]
pub struct DetailState {
    pub policy: Policy,
    pub before_snapshot: String,
    pub after_snapshot: String,
    pub high_contrast: bool,
}

impl DetailState {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            before_snapshot: "No snapshot available".to_string(),
            after_snapshot: "No snapshot available".to_string(),
            high_contrast: false,
        }
    }

    pub fn with_snapshots(mut self, before: String, after: String) -> Self {
        self.before_snapshot = before;
        self.after_snapshot = after;
        self
    }
}

pub struct DetailsScreen<'a> {
    state: &'a DetailState,
}

impl<'a> DetailsScreen<'a> {
    pub fn new(state: &'a DetailState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for DetailsScreen<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_header(chunks[0], buf);
        self.render_diff(chunks[1], buf);
        self.render_footer(chunks[2], buf);
    }
}

impl<'a> DetailsScreen<'a> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let accent = if self.state.high_contrast {
            Color::White
        } else {
            Color::Rgb(45, 212, 191)
        };

        let border_style = if self.state.high_contrast {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                " Policy Details ",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let title = self.state.policy.title.as_deref().unwrap_or("Untitled");
        let desc = self
            .state
            .policy
            .description
            .as_deref()
            .unwrap_or("No description");

        let text = vec![
            Line::from(Span::styled(
                title,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(&self.state.policy.id),
            ]),
            Line::from(desc),
        ];

        let paragraph = Paragraph::new(text);
        Widget::render(paragraph, inner, buf);
    }

    fn render_diff(&self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_snapshot_panel(chunks[0], "BEFORE", &self.state.before_snapshot, buf);
        self.render_snapshot_panel(chunks[1], "AFTER", &self.state.after_snapshot, buf);
    }

    fn render_snapshot_panel(&self, area: Rect, title: &str, content: &str, buf: &mut Buffer) {
        let accent = if self.state.high_contrast {
            Color::White
        } else {
            Color::Rgb(45, 212, 191)
        };

        let border_style = if self.state.high_contrast {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                format!(" {} ", title),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let lines: Vec<Line> = content
            .lines()
            .take((inner.height as usize).saturating_sub(1))
            .map(|line| Line::from(line.to_string()))
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        Widget::render(paragraph, inner, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = vec![Line::from(vec![
            Span::styled("[Esc] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Back  "),
            Span::styled("[d] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Diff  "),
            Span::styled("[q] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Quit"),
        ])];

        let paragraph = Paragraph::new(text);
        Widget::render(paragraph, area, buf);
    }
}
