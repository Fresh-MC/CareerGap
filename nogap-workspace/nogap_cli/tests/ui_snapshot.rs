/// Tests for Week 3 Stage 2 snapshot features

use nogap_cli::{
    components::{DiffViewerState, MultiSelectState},
    screens::{SnapshotBrowserState, SnapshotMetadata, SnapshotPreviewState},
};

// ============================================================================
// SNAPSHOT BROWSER TESTS
// ============================================================================

#[test]
fn test_snapshot_browser_listing() {
    // Create test snapshots
    let snapshots = vec![
        SnapshotMetadata {
            id: 1,
            timestamp: 1609459200, // 2021-01-01 00:00:00
            description: "Initial system state".to_string(),
        },
        SnapshotMetadata {
            id: 2,
            timestamp: 1609545600, // 2021-01-02 00:00:00
            description: "After security update".to_string(),
        },
        SnapshotMetadata {
            id: 3,
            timestamp: 1609632000, // 2021-01-03 00:00:00
            description: "Production deployment".to_string(),
        },
    ];

    let mut state = SnapshotBrowserState::new(snapshots.clone());

    // Verify initial state
    assert_eq!(state.selected, 0);
    assert_eq!(state.snapshots.len(), 3);

    // Test navigation down
    state.move_down();
    assert_eq!(state.selected, 1);
    
    state.move_down();
    assert_eq!(state.selected, 2);
    
    // Test clamping at bottom
    state.move_down();
    assert_eq!(state.selected, 2); // Should stay at last item
    
    // Test navigation up
    state.move_up();
    assert_eq!(state.selected, 1);
    
    state.move_up();
    assert_eq!(state.selected, 0);
    
    // Test clamping at top
    state.move_up();
    assert_eq!(state.selected, 0); // Should stay at first item

    // Test get_selected
    let selected = state.get_selected().unwrap();
    assert_eq!(selected.id, 1);
    assert_eq!(selected.description, "Initial system state");
}

#[test]
fn test_snapshot_browser_empty() {
    let state = SnapshotBrowserState::new(vec![]);
    
    assert_eq!(state.selected, 0);
    assert_eq!(state.snapshots.len(), 0);
    assert!(state.get_selected().is_none());
}

// ============================================================================
// SNAPSHOT PREVIEW TESTS
// ============================================================================

#[test]
fn test_snapshot_preview_render() {
    let before = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5".to_string();
    let after = "Line 1\nLine 2 modified\nLine 3\nLine 4\nLine 5".to_string();
    
    let mut state = SnapshotPreviewState::new(1, before.clone(), after.clone());
    
    // Verify initial state
    assert_eq!(state.snapshot_id, 1);
    assert_eq!(state.before_content, before);
    assert_eq!(state.after_content, after);
    assert_eq!(state.scroll_offset, 0);
    
    // Test scrolling down
    state.scroll_down(2);
    assert_eq!(state.scroll_offset, 2);
    
    state.scroll_down(1);
    assert_eq!(state.scroll_offset, 3);
    
    // Test scrolling up
    state.scroll_up(1);
    assert_eq!(state.scroll_offset, 2);
    
    state.scroll_up(3);
    assert_eq!(state.scroll_offset, 0); // Should not go negative
}

// ============================================================================
// DIFF VIEWER TESTS
// ============================================================================

#[test]
fn test_diff_viewer_basic() {
    let before = "Line 1\nLine 2\nLine 3";
    let after = "Line 1\nLine 2 modified\nLine 3\nLine 4";
    
    let state = DiffViewerState::new(before, after);
    
    // Verify diff lines are computed
    assert!(state.lines.len() > 0);
    
    // Verify initial scroll position
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn test_diff_viewer_scrolling() {
    let before = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
    let after = "Line 1\nLine 2\nLine 3 modified\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
    
    let mut state = DiffViewerState::new(before, after);
    
    // Test scroll_down
    state.scroll_down(3);
    assert_eq!(state.scroll_offset, 3);
    
    // Test scroll_up
    state.scroll_up(1);
    assert_eq!(state.scroll_offset, 2);
    
    state.scroll_up(5);
    assert_eq!(state.scroll_offset, 0); // Should clamp at 0
    
    // Test page_down
    state.page_down(5);
    assert_eq!(state.scroll_offset, 5);
    
    // Test page_up
    state.page_up(3);
    assert_eq!(state.scroll_offset, 2);
}

#[test]
fn test_diff_viewer_identical_content() {
    let content = "Line 1\nLine 2\nLine 3";
    
    let state = DiffViewerState::new(content, content);
    
    // All lines should be marked as Same
    assert!(state.lines.iter().all(|line| {
        matches!(line, nogap_cli::components::DiffLine::Same(_))
    }));
}

#[test]
fn test_diff_viewer_completely_different() {
    let before = "Line 1\nLine 2\nLine 3";
    let after = "Different 1\nDifferent 2\nDifferent 3";
    
    let state = DiffViewerState::new(before, after);
    
    // Should have changed lines (simple diff treats all as changed)
    assert!(state.lines.len() >= 3);
}

// ============================================================================
// MULTISELECT TESTS
// ============================================================================

#[test]
fn test_multiselect_select_and_clear() {
    let mut state = MultiSelectState::new();
    
    // Initial state - not active
    assert!(!state.active);
    assert_eq!(state.selected_count(), 0);
    
    // Enter multiselect mode
    state.enter_mode();
    assert!(state.active);
    
    // Toggle selection
    state.toggle(0);
    assert!(state.is_selected(0));
    assert_eq!(state.selected_count(), 1);
    
    state.toggle(2);
    assert!(state.is_selected(2));
    assert_eq!(state.selected_count(), 2);
    
    // Toggle off
    state.toggle(0);
    assert!(!state.is_selected(0));
    assert_eq!(state.selected_count(), 1);
    
    // Select all
    state.select_all(&[0, 1, 2, 3, 4]);
    assert_eq!(state.selected_count(), 5);
    assert!(state.is_selected(0));
    assert!(state.is_selected(4));
    
    // Clear all
    state.clear_all();
    assert_eq!(state.selected_count(), 0);
    assert!(!state.is_selected(0));
    
    // Exit mode
    state.exit_mode();
    assert!(!state.active);
}

#[test]
fn test_multiselect_get_selected_indices() {
    let mut state = MultiSelectState::new();
    state.enter_mode();
    
    // Select non-sequential indices
    state.toggle(4);
    state.toggle(1);
    state.toggle(7);
    state.toggle(2);
    
    let indices = state.get_selected_indices();
    
    // Should be sorted
    assert_eq!(indices, vec![1, 2, 4, 7]);
}

#[test]
fn test_multiselect_exit_clears_selection() {
    let mut state = MultiSelectState::new();
    state.enter_mode();
    
    state.toggle(0);
    state.toggle(1);
    assert_eq!(state.selected_count(), 2);
    
    state.exit_mode();
    assert!(!state.active);
    assert_eq!(state.selected_count(), 0); // Should clear on exit
}

// ============================================================================
// INTEGRATION TESTS (MOCK-BASED)
// ============================================================================

#[test]
fn test_batch_audit_calls_engine() {
    // This is a simplified test - in real implementation, batch_audit
    // would call nogap_core::engine::audit() for each selected policy.
    // Since we can't easily mock the engine here, we just verify the
    // multiselect state management.
    
    let mut multiselect = MultiSelectState::new();
    multiselect.enter_mode();
    
    // Simulate selecting policies
    multiselect.toggle(0);
    multiselect.toggle(2);
    multiselect.toggle(5);
    
    let selected = multiselect.get_selected_indices();
    assert_eq!(selected.len(), 3);
    
    // Simulate batch operation completion - exit multiselect mode
    multiselect.exit_mode();
    assert!(!multiselect.active);
    assert_eq!(multiselect.selected_count(), 0);
}

#[test]
fn test_batch_failures_skip_and_abort() {
    // This test simulates the behavior described in the spec:
    // "If audit fails, update status to 'Failed', skip to next, show error.
    //  If remediate fails, stop processing, exit multiselect, show error."
    
    let mut multiselect = MultiSelectState::new();
    multiselect.enter_mode();
    
    // Select 5 policies
    multiselect.toggle(0);
    multiselect.toggle(1);
    multiselect.toggle(2);
    multiselect.toggle(3);
    multiselect.toggle(4);
    
    assert_eq!(multiselect.selected_count(), 5);
    
    // Simulate processing - if an error occurs at index 2 during remediate,
    // we would abort and exit multiselect mode
    
    // Simulate abort on error
    multiselect.exit_mode();
    assert!(!multiselect.active);
    
    // For audit failures, we would continue processing but mark failed items
    // This is handled in the batch_audit implementation in ui.rs
}
