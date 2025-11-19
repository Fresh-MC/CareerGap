use crate::components::{DiffViewer, DiffViewerState, MultiSelectState};
/// Top-level TUI event loop and input handler
use crate::keymap::KeyMap;
use crate::screens::{
    Dashboard, DashboardState, DetailState, DetailsScreen, PolicyFilter, PolicyStatus,
    SnapshotBrowser, SnapshotBrowserState, SnapshotMetadata, SnapshotPreview, SnapshotPreviewState,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nogap_core::{engine, policy_parser, snapshot};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    Details,
    Help,
    ConfirmRemediate,
    FilterModal,
    Snapshots,
    SnapshotPreview,
    SnapshotDiff,
    BatchMenu,
}

#[derive(Debug, Clone)]
struct Modal {
    title: String,
    message: String,
}

impl Modal {
    fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct FilterModalState {
    selected: usize,
    filter: PolicyFilter,
}

impl FilterModalState {
    fn new(current_filter: PolicyFilter) -> Self {
        Self {
            selected: 0,
            filter: current_filter,
        }
    }

    fn move_down(&mut self) {
        if self.selected < 5 {
            self.selected += 1;
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn toggle_selected(&mut self) {
        match self.selected {
            0 => self.filter.severity_high = !self.filter.severity_high,
            1 => self.filter.severity_medium = !self.filter.severity_medium,
            2 => self.filter.severity_low = !self.filter.severity_low,
            3 => self.filter.platform_windows = !self.filter.platform_windows,
            4 => self.filter.platform_linux = !self.filter.platform_linux,
            5 => self.filter.platform_macos = !self.filter.platform_macos,
            _ => {}
        }
    }
}

pub struct AppState {
    screen: Screen,
    dashboard: DashboardState,
    detail: Option<DetailState>,
    snapshot_browser: Option<SnapshotBrowserState>,
    snapshot_preview: Option<SnapshotPreviewState>,
    snapshot_diff: Option<DiffViewerState>,
    diff_previous_screen: Option<Screen>,
    multiselect: MultiSelectState,
    batch_menu_selected: usize,
    modal_message: Option<String>,
    modal_stack: Vec<Modal>,
    search_mode: bool,
    search_query: String,
    filter_modal: Option<FilterModalState>,
    should_quit: bool,
}

impl AppState {
    fn new(policies_path: &str) -> Result<Self> {
        let policies = policy_parser::load_policy(policies_path)?;
        let mut dashboard = DashboardState::new(policies);
        dashboard.compute_filtered_indices();

        Ok(Self {
            screen: Screen::Dashboard,
            dashboard,
            detail: None,
            snapshot_browser: None,
            snapshot_preview: None,
            snapshot_diff: None,
            diff_previous_screen: None,
            multiselect: MultiSelectState::new(),
            batch_menu_selected: 0,
            modal_message: None,
            modal_stack: Vec::new(),
            search_mode: false,
            search_query: String::new(),
            filter_modal: None,
            should_quit: false,
        })
    }

    fn push_modal(&mut self, modal: Modal) {
        self.modal_stack.push(modal);
    }

    fn pop_modal(&mut self) {
        self.modal_stack.pop();
    }

    fn current_modal(&self) -> Option<&Modal> {
        self.modal_stack.last()
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: crossterm::event::KeyModifiers) {
        // Handle search mode separately
        if self.search_mode {
            self.handle_search_key(code);
            return;
        }

        // Global keys
        if KeyMap::is_quit(code, modifiers) {
            if self.screen != Screen::Dashboard {
                self.screen = Screen::Dashboard;
                self.modal_message = None;
                self.filter_modal = None;
            } else {
                self.should_quit = true;
            }
            return;
        }

        if KeyMap::is_help(code) {
            self.screen = if self.screen == Screen::Help {
                Screen::Dashboard
            } else {
                Screen::Help
            };
            return;
        }

        match self.screen {
            Screen::Dashboard => self.handle_dashboard_key(code),
            Screen::Details => self.handle_details_key(code),
            Screen::Help => {
                // Any key closes help
                if matches!(code, KeyCode::Char(_) | KeyCode::Enter | KeyCode::Esc) {
                    self.screen = Screen::Dashboard;
                }
            }
            Screen::ConfirmRemediate => self.handle_confirm_key(code),
            Screen::FilterModal => self.handle_filter_modal_key(code),
            Screen::Snapshots => self.handle_snapshots_key(code),
            Screen::SnapshotPreview => self.handle_snapshot_preview_key(code),
            Screen::SnapshotDiff => self.handle_snapshot_diff_key(code),
            Screen::BatchMenu => self.handle_batch_menu_key(code),
        }
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.dashboard.filter.clear();
                self.dashboard.compute_filtered_indices();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.dashboard.filter = self.search_query.clone();
                self.dashboard.compute_filtered_indices();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.dashboard.filter = self.search_query.clone();
                self.dashboard.compute_filtered_indices();
            }
            _ => {}
        }
    }

    fn handle_filter_modal_key(&mut self, code: KeyCode) {
        if let Some(filter_modal) = &mut self.filter_modal {
            if KeyMap::is_down(code) {
                filter_modal.move_down();
            } else if KeyMap::is_up(code) {
                filter_modal.move_up();
            } else if KeyMap::is_space(code) {
                filter_modal.toggle_selected();
            } else if KeyMap::is_confirm(code) {
                // Apply filter
                self.dashboard.policy_filter = filter_modal.filter.clone();
                self.dashboard.compute_filtered_indices();
                self.filter_modal = None;
                self.screen = Screen::Dashboard;
            } else if matches!(code, KeyCode::Esc) {
                // Cancel filter
                self.filter_modal = None;
                self.screen = Screen::Dashboard;
            }
        }
    }

    fn handle_dashboard_key(&mut self, code: KeyCode) {
        if KeyMap::is_down(code) {
            self.dashboard.move_down();
        } else if KeyMap::is_up(code) {
            self.dashboard.move_up();
        } else if KeyMap::is_search(code) {
            self.search_mode = true;
            self.search_query.clear();
        } else if KeyMap::is_toggle_filter(code) {
            self.filter_modal = Some(FilterModalState::new(self.dashboard.policy_filter.clone()));
            self.screen = Screen::FilterModal;
        } else if KeyMap::is_sort(code) {
            self.dashboard.cycle_sort();
            self.dashboard
                .set_last_action(format!("Sort: {}", self.dashboard.sort_mode.label()));
        } else if KeyMap::is_toggle_theme(code) {
            self.dashboard.high_contrast = !self.dashboard.high_contrast;
        } else if KeyMap::is_remediate(code) {
            self.screen = Screen::ConfirmRemediate;
        } else if KeyMap::is_diff(code) {
            self.open_diff_viewer();
        } else if matches!(code, KeyCode::Char('m')) {
            // Toggle multi-select mode
            if self.multiselect.active {
                self.multiselect.exit_mode();
                self.dashboard
                    .set_last_action("Multi-select mode OFF".to_string());
            } else {
                self.multiselect.enter_mode();
                self.dashboard.set_last_action(
                    "Multi-select mode ON (Space=toggle, A=all, N=clear)".to_string(),
                );
            }
        } else if matches!(code, KeyCode::Char('b')) {
            // Open batch menu if multi-select active with selections
            if self.multiselect.active && self.multiselect.selected_count() > 0 {
                self.batch_menu_selected = 0;
                self.screen = Screen::BatchMenu;
            } else if !self.multiselect.active {
                self.dashboard
                    .set_last_action("Enable multi-select mode first (press 'm')".to_string());
            } else {
                self.dashboard
                    .set_last_action("No policies selected".to_string());
            }
        } else if matches!(code, KeyCode::Char('S')) {
            // Open snapshot browser
            self.load_snapshots();
            if self.snapshot_browser.is_some() {
                self.screen = Screen::Snapshots;
            }
        } else if matches!(code, KeyCode::Char(' ')) && self.multiselect.active {
            // Toggle current item selection
            if let Some(idx) = self.dashboard.get_selected_index() {
                self.multiselect.toggle(idx);
                self.dashboard
                    .set_last_action(format!("Selected: {}", self.multiselect.selected_count()));
            }
        } else if matches!(code, KeyCode::Char('A')) && self.multiselect.active {
            // Select all filtered items
            let indices: Vec<usize> = (0..self.dashboard.filtered_indices.len()).collect();
            self.multiselect.select_all(&indices);
            self.dashboard.set_last_action(format!(
                "Selected all: {}",
                self.multiselect.selected_count()
            ));
        } else if matches!(code, KeyCode::Char('N')) && self.multiselect.active {
            // Clear all selections
            self.multiselect.clear_all();
            self.dashboard
                .set_last_action("Selections cleared".to_string());
        }
        // Note: Audit key is handled in main loop to support blocking modal flow
    }

    fn handle_details_key(&mut self, code: KeyCode) {
        if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
            self.screen = Screen::Dashboard;
        } else if code == KeyCode::Char('d') {
            // Open diff viewer from detail screen snapshots
            if let Some(ref detail) = self.detail {
                let before = detail.before_snapshot.clone();
                let after = detail.after_snapshot.clone();

                let state = DiffViewerState::new(&before, &after);
                self.snapshot_diff = Some(state);
                self.diff_previous_screen = Some(Screen::Details);
                self.screen = Screen::SnapshotDiff;
            }
        }
    }

    fn handle_confirm_key(&mut self, code: KeyCode) {
        if KeyMap::is_confirm(code) {
            // Mark that we need to run remediate, but don't do it here
            // The main loop will handle it
            self.modal_message = Some("remediate_confirmed".to_string());
            self.screen = Screen::Dashboard;
        } else if matches!(code, KeyCode::Esc | KeyCode::Char('n')) {
            self.screen = Screen::Dashboard;
            self.dashboard
                .set_last_action("Remediation cancelled".to_string());
        }
    }

    fn handle_snapshots_key(&mut self, code: KeyCode) {
        if let Some(ref mut state) = self.snapshot_browser {
            if KeyMap::is_down(code) {
                state.move_down();
            } else if KeyMap::is_up(code) {
                state.move_up();
            } else if matches!(code, KeyCode::Enter) {
                // Load snapshot preview - get ID first to avoid borrow issues
                let snapshot_id = state.get_selected().map(|s| s.id);
                if let Some(id) = snapshot_id {
                    self.load_snapshot_preview(id);
                    if self.snapshot_preview.is_some() {
                        self.screen = Screen::SnapshotPreview;
                    }
                }
            } else if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                self.snapshot_browser = None;
                self.screen = Screen::Dashboard;
            }
        }
    }

    fn handle_snapshot_preview_key(&mut self, code: KeyCode) {
        if let Some(ref mut state) = self.snapshot_preview {
            if KeyMap::is_down(code) || matches!(code, KeyCode::Char('j')) {
                state.scroll_down(1);
            } else if KeyMap::is_up(code) || matches!(code, KeyCode::Char('k')) {
                state.scroll_up(1);
            } else if matches!(code, KeyCode::Char('d')) {
                // Open diff viewer
                self.open_snapshot_diff();
                if self.snapshot_diff.is_some() {
                    self.diff_previous_screen = Some(Screen::SnapshotPreview);
                    self.screen = Screen::SnapshotDiff;
                }
            } else if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                self.snapshot_preview = None;
                self.screen = Screen::Snapshots;
            }
        }
    }

    fn handle_snapshot_diff_key(&mut self, code: KeyCode) {
        if let Some(ref mut state) = self.snapshot_diff {
            if KeyMap::is_down(code) || matches!(code, KeyCode::Char('j')) {
                state.scroll_down(1);
            } else if KeyMap::is_up(code) || matches!(code, KeyCode::Char('k')) {
                state.scroll_up(1);
            } else if matches!(code, KeyCode::PageDown) {
                state.page_down(20);
            } else if matches!(code, KeyCode::PageUp) {
                state.page_up(20);
            } else if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                self.snapshot_diff = None;
                // Go back to the screen we came from
                self.screen = self
                    .diff_previous_screen
                    .take()
                    .unwrap_or(Screen::SnapshotPreview);
            }
        }
    }

    fn handle_batch_menu_key(&mut self, code: KeyCode) {
        if KeyMap::is_down(code) || matches!(code, KeyCode::Char('j')) {
            self.batch_menu_selected = (self.batch_menu_selected + 1) % 2;
        } else if KeyMap::is_up(code) || matches!(code, KeyCode::Char('k')) {
            self.batch_menu_selected = if self.batch_menu_selected == 0 { 1 } else { 0 };
        } else if matches!(code, KeyCode::Enter) {
            // Execute selected batch operation
            if self.batch_menu_selected == 0 {
                self.batch_audit();
            } else {
                self.batch_remediate();
            }
            self.screen = Screen::Dashboard;
        } else if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
            self.screen = Screen::Dashboard;
        }
    }

    fn run_audit(&mut self) {
        // Get the selected policy
        if let Some(policy) = self.dashboard.get_selected_policy().cloned() {
            let policies = vec![policy];

            // Call the engine synchronously - blocking call
            match engine::audit(&policies) {
                Ok(audit_results) => {
                    // Apply results to dashboard state
                    for result in audit_results {
                        let selected = self.dashboard.selected;
                        let status = if result.passed {
                            PolicyStatus::Pass
                        } else {
                            PolicyStatus::Fail
                        };
                        self.dashboard.update_status(selected, status);
                    }

                    let title = policies[0].title.as_deref().unwrap_or("Untitled");
                    self.dashboard
                        .set_last_action(format!("Audited: {}", title));
                }
                Err(e) => {
                    self.dashboard
                        .set_last_action(format!("Audit failed: {}", e));
                }
            }
        }
    }

    fn run_remediate(&mut self) {
        // Get the selected policy
        if let Some(policy) = self.dashboard.get_selected_policy().cloned() {
            let policies = vec![policy];

            // Call the engine synchronously - blocking call
            let snapshot_provider = engine::RealSnapshotProvider;
            match engine::remediate(&policies, &snapshot_provider) {
                Ok(remediate_results) => {
                    // Apply results to dashboard state
                    for result in remediate_results {
                        let selected = self.dashboard.selected;
                        let status = match result {
                            engine::RemediateResult::Success { .. } => PolicyStatus::Pass,
                            engine::RemediateResult::Failed { .. } => PolicyStatus::Fail,
                        };
                        self.dashboard.update_status(selected, status);
                    }

                    let title = policies[0].title.as_deref().unwrap_or("Untitled");
                    self.dashboard
                        .set_last_action(format!("Remediated: {}", title));
                }
                Err(e) => {
                    self.dashboard
                        .set_last_action(format!("Remediate failed: {}", e));
                }
            }
        }
    }

    fn open_diff_viewer(&mut self) {
        if let Some(policy) = self.dashboard.get_selected_policy().cloned() {
            let detail = DetailState::new(policy).with_snapshots(
                "# BEFORE snapshot\nSystem state before remediation\n\nExample content..."
                    .to_string(),
                "# AFTER snapshot\nSystem state after remediation\n\nExample content..."
                    .to_string(),
            );

            self.detail = Some(detail);
            self.screen = Screen::Details;
        }
    }

    fn load_snapshots(&mut self) {
        // Initialize snapshot database and load snapshot list
        match snapshot::init_db() {
            Ok(conn) => match snapshot::list_snapshots(&conn) {
                Ok(snapshots) => {
                    let snapshot_list: Vec<SnapshotMetadata> = snapshots
                        .iter()
                        .map(|s| SnapshotMetadata {
                            id: s.0,
                            timestamp: s.1,
                            description: s.2.clone(),
                        })
                        .collect();

                    self.snapshot_browser = Some(SnapshotBrowserState::new(snapshot_list));
                }
                Err(e) => {
                    self.dashboard
                        .set_last_action(format!("Error loading snapshots: {}", e));
                }
            },
            Err(e) => {
                self.dashboard
                    .set_last_action(format!("Error initializing snapshot DB: {}", e));
            }
        }
    }

    fn load_snapshot_preview(&mut self, snapshot_id: i64) {
        // Load snapshot details
        match snapshot::init_db() {
            Ok(conn) => match snapshot::get_snapshot(&conn, snapshot_id) {
                Ok(snap) => {
                    let before_content = snap.2.clone();
                    let after_content = snap.3.clone();

                    self.snapshot_preview = Some(SnapshotPreviewState::new(
                        snapshot_id,
                        before_content,
                        after_content,
                    ));
                }
                Err(e) => {
                    self.dashboard
                        .set_last_action(format!("Error loading snapshot: {}", e));
                }
            },
            Err(e) => {
                self.dashboard
                    .set_last_action(format!("Error initializing snapshot DB: {}", e));
            }
        }
    }

    fn open_snapshot_diff(&mut self) {
        if let Some(ref preview) = self.snapshot_preview {
            let before = preview.before_content.clone();
            let after = preview.after_content.clone();

            let state = DiffViewerState::new(&before, &after);
            self.snapshot_diff = Some(state);
        }
    }

    fn batch_audit(&mut self) {
        let selected_indices = self.multiselect.get_selected_indices();
        let count = selected_indices.len();

        if count == 0 {
            self.dashboard
                .set_last_action("No policies selected".to_string());
            return;
        }

        let start_time = Instant::now();
        let mut processed = 0;
        let mut failed = 0;

        for idx in selected_indices {
            // Get the policy at this index
            if let Some(policy) = self.dashboard.policies.get(idx).cloned() {
                processed += 1;

                // Audit synchronously (blocking)
                match engine::audit(std::slice::from_ref(&policy)) {
                    Ok(audit_results) => {
                        if let Some(result) = audit_results.first() {
                            let status = if result.passed {
                                PolicyStatus::Pass
                            } else {
                                PolicyStatus::Fail
                            };
                            self.dashboard.update_status(idx, status);
                        }
                    }
                    Err(_e) => {
                        failed += 1;
                    }
                }
            }
        }

        let elapsed = start_time.elapsed().as_millis();
        self.dashboard.set_last_action(format!(
            "Batch Audit: {} processed, {} failed, {} ms",
            processed, failed, elapsed
        ));

        // Exit multi-select mode
        self.multiselect.exit_mode();
    }

    fn batch_remediate(&mut self) {
        let selected_indices = self.multiselect.get_selected_indices();
        let count = selected_indices.len();

        if count == 0 {
            self.dashboard
                .set_last_action("No policies selected".to_string());
            return;
        }

        let start_time = Instant::now();
        let mut processed = 0;
        let mut failed = 0;
        let snapshot_provider = engine::RealSnapshotProvider;

        for idx in selected_indices {
            // Get the policy at this index
            if let Some(policy) = self.dashboard.policies.get(idx).cloned() {
                processed += 1;

                // Remediate synchronously (blocking)
                match engine::remediate(std::slice::from_ref(&policy), &snapshot_provider) {
                    Ok(_) => {
                        // Update status to Pass
                        self.dashboard.update_status(idx, PolicyStatus::Pass);
                    }
                    Err(_e) => {
                        failed += 1;
                    }
                }
            }
        }

        let elapsed = start_time.elapsed().as_millis();
        self.dashboard.set_last_action(format!(
            "Batch Remediate: {} processed, {} failed, {} ms",
            processed, failed, elapsed
        ));

        // Exit multi-select mode
        self.multiselect.exit_mode();
    }
}

pub fn run_tui(policies_path: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = AppState::new(policies_path)?;
    let mut pending_audit = false;
    let mut pending_remediate = false;

    // Main loop
    while !app.should_quit {
        terminal.draw(|f| {
            let size = f.area();

            match app.screen {
                Screen::Dashboard => {
                    let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                    f.render_widget(dashboard, size);

                    // Render search box overlay if in search mode
                    if app.search_mode {
                        render_search_box(f, size, &app.search_query, app.dashboard.high_contrast);
                    }

                    // Render quick actions bar at bottom
                    render_quick_actions_bar(f, size, &app.dashboard, app.dashboard.high_contrast);
                }
                Screen::Details => {
                    if let Some(ref detail) = app.detail {
                        let mut detail_state = detail.clone();
                        detail_state.high_contrast = app.dashboard.high_contrast;
                        let details = DetailsScreen::new(&detail_state);
                        f.render_widget(details, size);
                    }
                }
                Screen::Help => {
                    render_help(f, size, app.dashboard.high_contrast);
                }
                Screen::ConfirmRemediate => {
                    let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                    f.render_widget(dashboard, size);

                    if let Some(policy) = app.dashboard.get_selected_policy() {
                        let title = policy.title.as_deref().unwrap_or("Untitled");
                        let message = format!(
                            "Remediate policy: {} [{}]?\n\nThis will create a snapshot and apply changes.\n\n[Enter/y] Confirm  [Esc/n] Cancel",
                            title, policy.id
                        );
                        render_modal(f, size, "Confirm Remediation", &message, app.dashboard.high_contrast);
                    }
                }
                Screen::FilterModal => {
                    let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                    f.render_widget(dashboard, size);

                    if let Some(ref filter_modal) = app.filter_modal {
                        render_filter_modal(f, size, filter_modal, app.dashboard.high_contrast);
                    }
                }
                Screen::Snapshots => {
                    if let Some(ref snapshot_browser) = app.snapshot_browser {
                        let browser = SnapshotBrowser::new(snapshot_browser);
                        f.render_widget(browser, size);
                    }
                }
                Screen::SnapshotPreview => {
                    if let Some(ref snapshot_preview) = app.snapshot_preview {
                        let preview = SnapshotPreview::new(snapshot_preview);
                        f.render_widget(preview, size);
                    }
                }
                Screen::SnapshotDiff => {
                    if let Some(ref snapshot_diff) = app.snapshot_diff {
                        let diff = DiffViewer::new(snapshot_diff);
                        f.render_widget(diff, size);
                    }
                }
                Screen::BatchMenu => {
                    // Render dashboard in background
                    let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                    f.render_widget(dashboard, size);

                    // Render batch menu modal
                    let count = app.multiselect.selected_count();
                    let options = ["Batch Audit", "Batch Remediate"];
                    let selected_marker = if app.batch_menu_selected == 0 { ">" } else { " " };
                    let other_marker = if app.batch_menu_selected == 1 { ">" } else { " " };

                    let message = format!(
                        "Selected {} policies\n\n{} {}\n{} {}\n\n[j/k] Navigate  [Enter] Confirm  [Esc] Cancel",
                        count,
                        selected_marker, options[0],
                        other_marker, options[1]
                    );
                    render_modal(f, size, "Batch Operations", &message, app.dashboard.high_contrast);
                }
            }

            // Render modal stack (blocking operations modal)
            if let Some(modal) = app.current_modal() {
                render_modal(f, size, &modal.title, &modal.message, app.dashboard.high_contrast);
            }
        })?;

        // Handle pending operations that need modal flow
        if pending_audit {
            pending_audit = false;

            // 1. Push modal
            app.push_modal(Modal::new(
                "AUDITING",
                "Please wait. This operation is blocking and will complete soon.",
            ));

            // 2. Render modal immediately
            terminal.draw(|f| {
                let size = f.area();
                let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                f.render_widget(dashboard, size);

                if let Some(modal) = app.current_modal() {
                    render_modal(
                        f,
                        size,
                        &modal.title,
                        &modal.message,
                        app.dashboard.high_contrast,
                    );
                }
            })?;

            // 3. Execute blocking operation
            app.run_audit();

            // 4. Pop modal
            app.pop_modal();
        }

        if pending_remediate {
            pending_remediate = false;

            // 1. Push modal
            app.push_modal(Modal::new(
                "REMEDIATING",
                "Please wait. This operation is blocking and will complete soon.",
            ));

            // 2. Render modal immediately
            terminal.draw(|f| {
                let size = f.area();
                let dashboard = Dashboard::new(&app.dashboard, &app.multiselect);
                f.render_widget(dashboard, size);

                if let Some(modal) = app.current_modal() {
                    render_modal(
                        f,
                        size,
                        &modal.title,
                        &modal.message,
                        app.dashboard.high_contrast,
                    );
                }
            })?;

            // 3. Execute blocking operation
            app.run_remediate();

            // 4. Pop modal
            app.pop_modal();
        }

        // Check for remediate confirmation from modal_message
        if let Some(ref msg) = app.modal_message {
            if msg == "remediate_confirmed" {
                app.modal_message = None;
                pending_remediate = true;
                continue; // Skip input handling this iteration
            }
        }

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Check for audit key before general handler
                if app.screen == Screen::Dashboard && KeyMap::is_audit(key.code) {
                    pending_audit = true;
                    continue;
                }

                app.handle_key(key.code, key.modifiers);
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_help(f: &mut ratatui::Frame, area: Rect, high_contrast: bool) {
    let accent = if high_contrast {
        Color::White
    } else {
        Color::Rgb(45, 212, 191)
    };

    let border_style = if high_contrast {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " Help - Keybindings ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let help_items = KeyMap::help_text();
    let mut lines = vec![
        Line::from(Span::styled(
            "NoGap Operator Cockpit",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (key, desc) in help_items {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:10}", key),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw(desc),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(paragraph, inner);
}

fn render_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    message: &str,
    high_contrast: bool,
) {
    let accent = if high_contrast {
        Color::White
    } else {
        Color::Rgb(45, 212, 191)
    };

    // Center the modal
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vertical[1]);

    let modal_area = horizontal[1];

    // Clear the area
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let text = Paragraph::new(message)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    f.render_widget(text, inner);
}

fn render_search_box(f: &mut ratatui::Frame, area: Rect, query: &str, high_contrast: bool) {
    let accent = if high_contrast {
        Color::White
    } else {
        Color::Rgb(45, 212, 191)
    };

    // Position search box at top right
    let search_width = 40.min(area.width.saturating_sub(4));
    let search_x = area.right().saturating_sub(search_width + 2);
    let search_area = Rect::new(search_x, area.top() + 1, search_width, 3);

    f.render_widget(Clear, search_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            " Search ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(search_area);
    f.render_widget(block, search_area);

    let text = Paragraph::new(format!("/{}", query)).style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}

fn render_quick_actions_bar(
    f: &mut ratatui::Frame,
    area: Rect,
    dashboard: &DashboardState,
    _high_contrast: bool,
) {
    let bar_y = area.bottom().saturating_sub(1);
    let bar_area = Rect::new(area.left(), bar_y, area.width, 1);

    let status_color = if dashboard.get_selected_policy().is_some() {
        let idx = dashboard.get_selected_index().unwrap_or(0);
        match dashboard.statuses.get(idx) {
            Some(PolicyStatus::Pass) => Color::Green,
            Some(PolicyStatus::Fail) => Color::Red,
            Some(PolicyStatus::Warning) => Color::Yellow,
            _ => Color::Gray,
        }
    } else {
        Color::Gray
    };

    let actions = [
        "[a] Audit".to_string(),
        "[r] Remediate".to_string(),
        "[d] Diff".to_string(),
        "[f] Filter".to_string(),
        format!("[o] Sort: {}", dashboard.sort_mode.label()),
        "[/] Search".to_string(),
    ];

    let text = Line::from(
        actions
            .iter()
            .map(|a| Span::styled(format!(" {} ", a), Style::default().fg(status_color)))
            .collect::<Vec<_>>(),
    );

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::Black))
        .alignment(Alignment::Left);

    f.render_widget(paragraph, bar_area);
}

fn render_filter_modal(
    f: &mut ratatui::Frame,
    area: Rect,
    filter_modal: &FilterModalState,
    high_contrast: bool,
) {
    let accent = if high_contrast {
        Color::White
    } else {
        Color::Rgb(45, 212, 191)
    };

    // Center the modal
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(vertical[1]);

    let modal_area = horizontal[1];

    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            " Policy Filters ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let checkboxes = vec![
        (0, "Severity: High", filter_modal.filter.severity_high),
        (1, "Severity: Medium", filter_modal.filter.severity_medium),
        (2, "Severity: Low", filter_modal.filter.severity_low),
        (3, "Platform: Windows", filter_modal.filter.platform_windows),
        (4, "Platform: Linux", filter_modal.filter.platform_linux),
        (5, "Platform: macOS", filter_modal.filter.platform_macos),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            "Use j/k to navigate, Space to toggle, Enter to apply",
            Style::default().add_modifier(Modifier::DIM),
        )),
        Line::from(""),
    ];

    for (idx, label, checked) in checkboxes {
        let checkbox = if checked { "[✓]" } else { "[ ]" };
        let style = if idx == filter_modal.selected {
            Style::default()
                .fg(accent)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", checkbox), style),
            Span::styled(label, style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "[Enter] Apply  [Esc] Cancel",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    f.render_widget(paragraph, inner);
}
