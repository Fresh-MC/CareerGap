/// Snapshot Browser and Snapshot Preview screens

use crate::components::{create_row, TableWidget};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Clone)]
pub struct SnapshotMetadata {
    pub id: i64,
    pub timestamp: i64,
    pub description: String,
}

#[derive(Clone)]
pub struct SnapshotBrowserState {
    pub snapshots: Vec<SnapshotMetadata>,
    pub selected: usize,
    pub high_contrast: bool,
}

impl SnapshotBrowserState {
    pub fn new(snapshots: Vec<SnapshotMetadata>) -> Self {
        Self {
            snapshots,
            selected: 0,
            high_contrast: false,
        }
    }

    pub fn move_down(&mut self) {
        if !self.snapshots.is_empty() && self.selected < self.snapshots.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn get_selected(&self) -> Option<&SnapshotMetadata> {
        self.snapshots.get(self.selected)
    }
}

pub struct SnapshotBrowser<'a> {
    state: &'a SnapshotBrowserState,
}

impl<'a> SnapshotBrowser<'a> {
    pub fn new(state: &'a SnapshotBrowserState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for SnapshotBrowser<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        self.render_table(chunks[0], buf);
        self.render_footer(chunks[1], buf);
    }
}

impl<'a> SnapshotBrowser<'a> {
    fn render_table(&self, area: Rect, buf: &mut Buffer) {
        let rows: Vec<_> = self
            .state
            .snapshots
            .iter()
            .map(|snapshot| {
                let timestamp_str = format!("{}", snapshot.timestamp);
                let desc = snapshot.description.clone();
                
                create_row(
                    vec![
                        snapshot.id.to_string(),
                        timestamp_str,
                        desc,
                    ],
                    Color::White,
                )
            })
            .collect();

        let table = TableWidget::new(
            " Snapshots ",
            vec!["ID", "Timestamp", "Description"],
            vec![8, 20, 50],
        )
        .rows(rows)
        .selected(self.state.selected)
        .high_contrast(self.state.high_contrast)
        .total_rows(self.state.snapshots.len());

        Widget::render(table, area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
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
            .border_style(border_style);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let text = vec![Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                format!("{} snapshots", self.state.snapshots.len()),
                Style::default().fg(accent),
            ),
            Span::raw("  "),
            Span::styled("[Enter] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Preview  "),
            Span::styled("[Esc] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Back"),
        ])];

        let paragraph = Paragraph::new(text);
        Widget::render(paragraph, inner, buf);
    }
}

/// Snapshot Preview screen showing BEFORE and AFTER states
#[derive(Clone)]
pub struct SnapshotPreviewState {
    pub snapshot_id: i64,
    pub before_content: String,
    pub after_content: String,
    pub scroll_offset: usize,
    pub high_contrast: bool,
}

impl SnapshotPreviewState {
    pub fn new(snapshot_id: i64, before: String, after: String) -> Self {
        Self {
            snapshot_id,
            before_content: before,
            after_content: after,
            scroll_offset: 0,
            high_contrast: false,
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }
}

pub struct SnapshotPreview<'a> {
    state: &'a SnapshotPreviewState,
}

impl<'a> SnapshotPreview<'a> {
    pub fn new(state: &'a SnapshotPreviewState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for SnapshotPreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

        self.render_header(chunks[0], buf);
        self.render_content(chunks[1], buf);
        self.render_footer(chunks[2], buf);
    }
}

impl<'a> SnapshotPreview<'a> {
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
                format!(" Snapshot #{} ", self.state.snapshot_id),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let text = vec![Line::from(vec![
            Span::styled("BEFORE ", Style::default().fg(Color::Yellow)),
            Span::raw("and "),
            Span::styled("AFTER ", Style::default().fg(Color::Green)),
            Span::raw("states"),
        ])];

        let paragraph = Paragraph::new(text);
        Widget::render(paragraph, inner, buf);
    }

    fn render_content(&self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        self.render_panel(chunks[0], "BEFORE", &self.state.before_content, buf);
        self.render_panel(chunks[1], "AFTER", &self.state.after_content, buf);
    }

    fn render_panel(&self, area: Rect, title: &str, content: &str, buf: &mut Buffer) {
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
            .skip(self.state.scroll_offset)
            .take((inner.height as usize).saturating_sub(1))
            .map(|line| Line::from(line.to_string()))
            .collect();

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        Widget::render(paragraph, inner, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = vec![Line::from(vec![
            Span::styled("[d] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Diff  "),
            Span::styled("[j/k] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Scroll  "),
            Span::styled("[Esc] ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("Back"),
        ])];

        let paragraph = Paragraph::new(text);
        Widget::render(paragraph, area, buf);
    }
}
