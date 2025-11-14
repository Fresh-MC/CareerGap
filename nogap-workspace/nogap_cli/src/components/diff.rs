/// Line-by-line diff viewer component

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    Same(String),
    Added(String),
    Removed(String),
    Changed(String, String), // (old, new)
}

impl DiffLine {
    pub fn to_line(&self) -> Line<'static> {
        match self {
            DiffLine::Same(s) => Line::from(Span::raw(format!("  {}", s))),
            DiffLine::Added(s) => Line::from(Span::styled(
                format!("+ {}", s),
                Style::default().fg(Color::Green),
            )),
            DiffLine::Removed(s) => Line::from(Span::styled(
                format!("- {}", s),
                Style::default().fg(Color::Red),
            )),
            DiffLine::Changed(old, new) => {
                // Show as yellow with before -> after
                Line::from(Span::styled(
                    format!("~ {} -> {}", old, new),
                    Style::default().fg(Color::Yellow),
                ))
            }
        }
    }
}

pub struct DiffViewerState {
    pub lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub high_contrast: bool,
}

impl DiffViewerState {
    pub fn new(before: &str, after: &str) -> Self {
        let lines = Self::compute_diff(before, after);
        Self {
            lines,
            scroll_offset: 0,
            high_contrast: false,
        }
    }

    fn compute_diff(before: &str, after: &str) -> Vec<DiffLine> {
        let before_lines: Vec<&str> = before.lines().collect();
        let after_lines: Vec<&str> = after.lines().collect();

        let mut result = Vec::new();
        let max_len = before_lines.len().max(after_lines.len());

        for i in 0..max_len {
            let before_line = before_lines.get(i);
            let after_line = after_lines.get(i);

            match (before_line, after_line) {
                (Some(b), Some(a)) if b == a => {
                    result.push(DiffLine::Same(b.to_string()));
                }
                (Some(b), Some(a)) => {
                    result.push(DiffLine::Changed(b.to_string(), a.to_string()));
                }
                (Some(b), None) => {
                    result.push(DiffLine::Removed(b.to_string()));
                }
                (None, Some(a)) => {
                    result.push(DiffLine::Added(a.to_string()));
                }
                (None, None) => break,
            }
        }

        result
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }
}

pub struct DiffViewer<'a> {
    state: &'a DiffViewerState,
}

impl<'a> DiffViewer<'a> {
    pub fn new(state: &'a DiffViewerState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for DiffViewer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
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
                " Diff Viewer ",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let visible_lines: Vec<Line> = self
            .state
            .lines
            .iter()
            .skip(self.state.scroll_offset)
            .take(inner.height as usize)
            .map(|diff_line| diff_line.to_line())
            .collect();

        let paragraph = Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false });

        Widget::render(paragraph, inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_same_lines() {
        let before = "line1\nline2\nline3";
        let after = "line1\nline2\nline3";
        let state = DiffViewerState::new(before, after);
        
        assert_eq!(state.lines.len(), 3);
        assert!(matches!(state.lines[0], DiffLine::Same(_)));
    }

    #[test]
    fn test_diff_added_line() {
        let before = "line1\nline2";
        let after = "line1\nline2\nline3";
        let state = DiffViewerState::new(before, after);
        
        assert_eq!(state.lines.len(), 3);
        assert!(matches!(state.lines[2], DiffLine::Added(_)));
    }

    #[test]
    fn test_diff_removed_line() {
        let before = "line1\nline2\nline3";
        let after = "line1\nline2";
        let state = DiffViewerState::new(before, after);
        
        assert_eq!(state.lines.len(), 3);
        assert!(matches!(state.lines[2], DiffLine::Removed(_)));
    }

    #[test]
    fn test_diff_changed_line() {
        let before = "line1\nold content\nline3";
        let after = "line1\nnew content\nline3";
        let state = DiffViewerState::new(before, after);
        
        assert_eq!(state.lines.len(), 3);
        assert!(matches!(state.lines[1], DiffLine::Changed(_, _)));
    }

    #[test]
    fn test_scroll_down() {
        let before = "line1\nline2\nline3\nline4\nline5";
        let after = "line1\nline2\nline3\nline4\nline5";
        let mut state = DiffViewerState::new(before, after);
        
        state.scroll_down(2);
        assert_eq!(state.scroll_offset, 2);
        
        state.scroll_down(10);
        assert_eq!(state.scroll_offset, 4); // max is lines.len() - 1
    }

    #[test]
    fn test_scroll_up() {
        let before = "line1\nline2\nline3";
        let after = "line1\nline2\nline3";
        let mut state = DiffViewerState::new(before, after);
        
        state.scroll_offset = 2;
        state.scroll_up(1);
        assert_eq!(state.scroll_offset, 1);
        
        state.scroll_up(10);
        assert_eq!(state.scroll_offset, 0);
    }
}
