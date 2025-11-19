/// Dashboard screen - main policy list view with status
use crate::components::{create_row, MultiSelectState, TableWidget};
use nogap_core::types::Policy;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    IdAscending,
    SeverityDescending,
    PlatformAscending,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::IdAscending => SortMode::SeverityDescending,
            SortMode::SeverityDescending => SortMode::PlatformAscending,
            SortMode::PlatformAscending => SortMode::IdAscending,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortMode::IdAscending => "ID ▲",
            SortMode::SeverityDescending => "Severity ▼",
            SortMode::PlatformAscending => "Platform ▲",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFilter {
    pub severity_high: bool,
    pub severity_medium: bool,
    pub severity_low: bool,
    pub platform_windows: bool,
    pub platform_linux: bool,
    pub platform_macos: bool,
}

impl Default for PolicyFilter {
    fn default() -> Self {
        Self {
            severity_high: true,
            severity_medium: true,
            severity_low: true,
            platform_windows: true,
            platform_linux: true,
            platform_macos: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyStatus {
    Unknown,
    Pass,
    Fail,
    Warning,
}

impl PolicyStatus {
    pub fn color(&self, high_contrast: bool) -> Color {
        if high_contrast {
            match self {
                PolicyStatus::Pass => Color::White,
                PolicyStatus::Fail => Color::White,
                PolicyStatus::Warning => Color::White,
                PolicyStatus::Unknown => Color::Gray,
            }
        } else {
            match self {
                PolicyStatus::Pass => Color::Green,
                PolicyStatus::Fail => Color::Red,
                PolicyStatus::Warning => Color::Rgb(251, 146, 60), // Orange
                PolicyStatus::Unknown => Color::Gray,
            }
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            PolicyStatus::Pass => "✓",
            PolicyStatus::Fail => "✗",
            PolicyStatus::Warning => "⚠",
            PolicyStatus::Unknown => "?",
        }
    }
}

pub struct DashboardState {
    pub policies: Vec<Policy>,
    pub statuses: Vec<PolicyStatus>,
    pub selected: usize,
    pub filter: String,
    pub filtered_indices: Vec<usize>,
    pub sort_mode: SortMode,
    pub policy_filter: PolicyFilter,
    pub high_contrast: bool,
    pub last_action: String,
}

impl DashboardState {
    pub fn new(policies: Vec<Policy>) -> Self {
        let count = policies.len();
        let filtered_indices = (0..count).collect();
        Self {
            policies,
            statuses: vec![PolicyStatus::Unknown; count],
            selected: 0,
            filter: String::new(),
            filtered_indices,
            sort_mode: SortMode::IdAscending,
            policy_filter: PolicyFilter::default(),
            high_contrast: false,
            last_action: "Ready".to_string(),
        }
    }

    pub fn compute_filtered_indices(&mut self) {
        self.filtered_indices = self
            .policies
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                // Text search filter
                let text_match = self.filter.is_empty()
                    || p.id.to_lowercase().contains(&self.filter.to_lowercase())
                    || p.description.as_ref().is_some_and(|d| {
                        d.to_lowercase().contains(&self.filter.to_lowercase())
                    });

                // Severity filter - check the severity field
                let severity_match = if let Some(sev) = &p.severity {
                    let sev_lower = sev.to_lowercase();
                    match sev_lower.as_str() {
                        "high" => self.policy_filter.severity_high,
                        "medium" => self.policy_filter.severity_medium,
                        "low" => self.policy_filter.severity_low,
                        _ => true, // Unknown severity passes through
                    }
                } else {
                    true // No severity specified passes through
                };

                // Platform filter - check the platform field
                let platform_match = {
                    let plat_lower = p.platform.to_lowercase();
                    match plat_lower.as_str() {
                        "windows" => self.policy_filter.platform_windows,
                        "linux" => self.policy_filter.platform_linux,
                        "macos" => self.policy_filter.platform_macos,
                        _ => true, // Unknown platform passes through
                    }
                };

                text_match && severity_match && platform_match
            })
            .map(|(idx, _)| idx)
            .collect();

        self.apply_sort();

        // Clamp selection to valid range
        if !self.filtered_indices.is_empty() && self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn apply_sort(&mut self) {
        match self.sort_mode {
            SortMode::IdAscending => {
                self.filtered_indices
                    .sort_by(|&a, &b| self.policies[a].id.cmp(&self.policies[b].id));
            }
            SortMode::SeverityDescending => {
                self.filtered_indices.sort_by(|&a, &b| {
                    let sev_a = self.policies[a].severity.as_deref().unwrap_or("unknown");
                    let sev_b = self.policies[b].severity.as_deref().unwrap_or("unknown");

                    // Map severity to ordering: high=3, medium=2, low=1, unknown=0
                    let order_a = match sev_a {
                        "high" => 3,
                        "medium" => 2,
                        "low" => 1,
                        _ => 0,
                    };
                    let order_b = match sev_b {
                        "high" => 3,
                        "medium" => 2,
                        "low" => 1,
                        _ => 0,
                    };

                    // Sort descending (high to low)
                    order_b.cmp(&order_a)
                });
            }
            SortMode::PlatformAscending => {
                self.filtered_indices
                    .sort_by(|&a, &b| self.policies[a].platform.cmp(&self.policies[b].platform));
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.apply_sort();
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty()
            && self.selected < self.filtered_indices.len().saturating_sub(1)
        {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn get_selected_policy(&self) -> Option<&Policy> {
        let actual_idx = self.filtered_indices.get(self.selected)?;
        self.policies.get(*actual_idx)
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected).copied()
    }

    pub fn update_status(&mut self, index: usize, status: PolicyStatus) {
        if index < self.statuses.len() {
            self.statuses[index] = status;
        }
    }

    pub fn set_last_action(&mut self, action: String) {
        self.last_action = action;
    }
}

pub struct Dashboard<'a> {
    state: &'a DashboardState,
    multiselect: &'a MultiSelectState,
}

impl<'a> Dashboard<'a> {
    pub fn new(state: &'a DashboardState, multiselect: &'a MultiSelectState) -> Self {
        Self { state, multiselect }
    }
}

impl<'a> Widget for Dashboard<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left panel - policy table
        let table_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(chunks[0]);

        self.render_policy_table(table_area[0], buf);
        self.render_status_bar(table_area[1], buf);

        // Right panel - details
        self.render_details(chunks[1], buf);
    }
}

impl<'a> Dashboard<'a> {
    fn render_policy_table(&self, area: Rect, buf: &mut Buffer) {
        let rows: Vec<_> = self
            .state
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(display_idx, &actual_idx)| {
                let policy = &self.state.policies[actual_idx];
                let status = self.state.statuses[actual_idx];
                let title = policy.title.as_deref().unwrap_or("Untitled");
                let severity = policy.severity.as_deref().unwrap_or("medium");
                let sev_short = match severity {
                    "critical" => "C",
                    "high" => "H",
                    "medium" => "M",
                    "low" => "L",
                    _ => "?",
                };

                // Add checkbox prefix if multiselect is active
                let checkbox = if self.multiselect.active {
                    if self.multiselect.is_selected(display_idx) {
                        "[x]"
                    } else {
                        "[ ]"
                    }
                } else {
                    ""
                };

                let id_with_checkbox = if checkbox.is_empty() {
                    policy.id.clone()
                } else {
                    format!("{} {}", checkbox, policy.id)
                };

                create_row(
                    vec![
                        id_with_checkbox,
                        title.to_string(),
                        sev_short.to_string(),
                        status.symbol().to_string(),
                    ],
                    status.color(self.state.high_contrast),
                )
            })
            .collect();

        let header_cols = vec!["ID", "Title", "S", "✓"];
        let col_widths = if self.multiselect.active {
            vec![16, 36, 3, 3] // Wider ID column for checkbox
        } else {
            vec![12, 40, 3, 3]
        };

        let table = TableWidget::new(" NoGap Policies ", header_cols, col_widths)
            .rows(rows)
            .selected(self.state.selected)
            .high_contrast(self.state.high_contrast)
            .total_rows(self.state.filtered_indices.len());

        Widget::render(table, area, buf);
    }

    fn render_details(&self, area: Rect, buf: &mut Buffer) {
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

        if let Some(policy) = self.state.get_selected_policy() {
            let title = policy.title.as_deref().unwrap_or("Untitled");
            let desc = policy.description.as_deref().unwrap_or("No description");
            let platform = &policy.platform;
            let check_type = &policy.check_type;

            let text = vec![
                Line::from(Span::styled(
                    title,
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "ID: ",
                    Style::default().add_modifier(Modifier::DIM),
                ))
                .spans
                .into_iter()
                .chain([Span::raw(&policy.id)])
                .collect::<Vec<_>>()
                .into(),
                Line::from(vec![
                    Span::styled("Platform: ", Style::default().add_modifier(Modifier::DIM)),
                    Span::raw(platform),
                ]),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().add_modifier(Modifier::DIM)),
                    Span::raw(check_type),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Description:",
                    Style::default().add_modifier(Modifier::DIM),
                )),
                Line::from(desc),
                Line::from(""),
                Line::from(Span::styled(
                    "Actions:",
                    Style::default().add_modifier(Modifier::DIM),
                )),
                Line::from("  [a] Run Audit"),
                Line::from("  [r] Remediate"),
                Line::from("  [d] View Diff"),
            ];

            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
            Widget::render(paragraph, inner, buf);
        }
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
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

        let status_text = vec![Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(&self.state.last_action, Style::default().fg(accent)),
            Span::raw("  "),
            Span::styled("[?] Help", Style::default().add_modifier(Modifier::DIM)),
            Span::raw("  "),
            Span::styled("[q] Quit", Style::default().add_modifier(Modifier::DIM)),
        ])];

        let paragraph = Paragraph::new(status_text);
        Widget::render(paragraph, inner, buf);
    }
}
