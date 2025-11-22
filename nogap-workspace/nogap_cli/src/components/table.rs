/// Reusable table component with sort and filter capabilities
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Row, Table, Widget},
};

#[derive(Clone, Debug)]
pub struct TableRow {
    pub cells: Vec<String>,
    pub style: Style,
}

pub struct TableWidget<'a> {
    headers: Vec<&'a str>,
    rows: Vec<TableRow>,
    widths: Vec<u16>,
    selected: usize,
    total_rows: usize,
    title: &'a str,
    high_contrast: bool,
    _scroll_offset: usize,
}

impl<'a> TableWidget<'a> {
    pub fn new(title: &'a str, headers: Vec<&'a str>, widths: Vec<u16>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            widths,
            selected: 0,
            total_rows: 0,
            title,
            high_contrast: false,
            _scroll_offset: 0,
        }
    }

    pub fn rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index.min(self.rows.len().saturating_sub(1));
        self
    }

    pub fn total_rows(mut self, total: usize) -> Self {
        self.total_rows = total;
        self
    }

    pub fn high_contrast(mut self, enabled: bool) -> Self {
        self.high_contrast = enabled;
        self
    }

    fn get_accent_color(&self) -> Color {
        if self.high_contrast {
            Color::White
        } else {
            Color::Rgb(45, 212, 191) // Teal #2DD4BF
        }
    }
}

impl<'a> Widget for TableWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let accent = self.get_accent_color();
        let border_style = if self.high_contrast {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                self.title,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

        let header_cells = self
            .headers
            .iter()
            .map(|h| Span::styled(*h, Style::default().fg(accent).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        // Calculate viewport: how many rows can fit on screen
        let viewport_height = area.height.saturating_sub(3).max(1) as usize; // Minus borders and header
        
        // Calculate scroll offset to keep selected item visible
        let offset = if self.rows.is_empty() || viewport_height == 0 {
            0
        } else if self.selected < viewport_height {
            // Selection is in first page, show from beginning
            0
        } else {
            // Scroll just enough to keep selection visible at bottom
            (self.selected + 1).saturating_sub(viewport_height)
        };

        // Only render visible rows, marking the selected one
        let visible_rows: Vec<Row> = self
            .rows
            .iter()
            .enumerate()
            .skip(offset)
            .take(viewport_height)
            .map(|(absolute_idx, row)| {
                // Check if this absolute index matches the selected index
                let style = if absolute_idx == self.selected {
                    Style::default()
                        .bg(accent)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    row.style
                };
                Row::new(row.cells.iter().map(|c| c.as_str())).style(style)
            })
            .collect();

        let table = Table::new(visible_rows, self.widths.iter().copied())
            .header(header)
            .block(block);

        Widget::render(table, area, buf);

        // Render scrollbar if needed
        if self.total_rows > 0 && area.height > 4 {
            self.render_scrollbar(area, buf);
        }
    }
}

impl<'a> TableWidget<'a> {
    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        if self.total_rows == 0 {
            return;
        }

        let viewport_height = (area.height.saturating_sub(3)) as usize; // Minus borders and header
        if self.total_rows <= viewport_height {
            return; // No need for scrollbar
        }

        let scrollbar_x = area.right().saturating_sub(1);
        let scrollbar_start = area.top() + 2; // After top border and header
        let scrollbar_end = area.bottom().saturating_sub(1); // Before bottom border
        let scrollbar_height = scrollbar_end.saturating_sub(scrollbar_start);

        if scrollbar_height == 0 {
            return;
        }

        // Calculate thumb position based on selected item and total rows
        // This ensures the thumb moves proportionally through the entire list
        let scroll_ratio = self.selected as f32 / self.total_rows.saturating_sub(1).max(1) as f32;
        let scroll_position = (scroll_ratio * scrollbar_height.saturating_sub(1) as f32) as u16;
        let thumb_y = scrollbar_start + scroll_position;

        let scrollbar_color = if self.high_contrast {
            Color::White
        } else {
            Color::Rgb(45, 212, 191)
        };

        // Draw scrollbar track
        for y in scrollbar_start..scrollbar_end {
            if y == thumb_y {
                buf.cell_mut((scrollbar_x, y))
                    .expect("valid scrollbar position")
                    .set_char('█')
                    .set_fg(scrollbar_color);
            } else {
                buf.cell_mut((scrollbar_x, y))
                    .expect("valid scrollbar position")
                    .set_char('│')
                    .set_fg(Color::DarkGray);
            }
        }
    }
}

/// Helper to create styled rows
pub fn create_row(cells: Vec<String>, color: Color) -> TableRow {
    TableRow {
        cells,
        style: Style::default().fg(color),
    }
}
