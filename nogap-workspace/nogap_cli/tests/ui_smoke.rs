/// Smoke tests for UI rendering using ratatui buffer snapshots

use nogap_cli::screens::{Dashboard, DashboardState, PolicyFilter, PolicyStatus, SortMode};
use nogap_cli::components::MultiSelectState;
use nogap_core::types::Policy;
use ratatui::{backend::TestBackend, Terminal};

fn create_test_policies() -> Vec<Policy> {
    vec![
        Policy {
            id: "SSH-H-001".to_string(),
            title: Some("SSH Root Login".to_string()),
            description: Some("Disable SSH root login".to_string()),
            platform: "Linux".to_string(),
            severity: Some("high".to_string()),
            check_type: "file".to_string(),
            ..Default::default()
        },
        Policy {
            id: "FW-M-002".to_string(),
            title: Some("Firewall Configuration".to_string()),
            description: Some("Enable firewall on macOS".to_string()),
            platform: "macOS".to_string(),
            severity: Some("medium".to_string()),
            check_type: "process".to_string(),
            ..Default::default()
        },
        Policy {
            id: "REG-L-003".to_string(),
            title: Some("Registry Protection".to_string()),
            description: Some("Protect Windows registry".to_string()),
            platform: "Windows".to_string(),
            severity: Some("low".to_string()),
            check_type: "registry".to_string(),
            ..Default::default()
        },
        Policy {
            id: "SSH-M-004".to_string(),
            title: Some("SSH Protocol Version".to_string()),
            description: Some("Enforce SSH protocol 2".to_string()),
            platform: "Linux".to_string(),
            severity: Some("medium".to_string()),
            check_type: "file".to_string(),
            ..Default::default()
        },
        Policy {
            id: "UAC-H-005".to_string(),
            title: Some("UAC Elevation".to_string()),
            description: Some("Require UAC elevation".to_string()),
            platform: "Windows".to_string(),
            severity: Some("high".to_string()),
            check_type: "registry".to_string(),
            ..Default::default()
        },
    ]
}

#[test]
fn test_dashboard_state_navigation() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.compute_filtered_indices();

    // Initial state
    assert_eq!(state.selected, 0);
    assert_eq!(state.policies.len(), 5);
    assert_eq!(state.statuses.len(), 5);
    assert!(matches!(state.statuses[0], PolicyStatus::Unknown));

    // Move down
    state.move_down();
    assert_eq!(state.selected, 1);

    state.move_down();
    assert_eq!(state.selected, 2);

    // Move up
    state.move_up();
    assert_eq!(state.selected, 1);

    state.move_up();
    assert_eq!(state.selected, 0);

    // Try to move before start - should stay at first item
    state.move_up();
    assert_eq!(state.selected, 0);
}

#[test]
fn test_dashboard_status_updates() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);

    // Update first policy status to Pass
    state.update_status(0, PolicyStatus::Pass);
    assert!(matches!(state.statuses[0], PolicyStatus::Pass));

    // Update second policy status to Fail
    state.update_status(1, PolicyStatus::Fail);
    assert!(matches!(state.statuses[1], PolicyStatus::Fail));

    // Update third policy status to Warning
    state.update_status(2, PolicyStatus::Warning);
    assert!(matches!(state.statuses[2], PolicyStatus::Warning));
}

#[test]
fn test_dashboard_get_selected_policy() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.compute_filtered_indices();

    // Get first policy (after ID ascending sort: FW-M-002)
    let policy = state.get_selected_policy().unwrap();
    assert_eq!(policy.id, "FW-M-002");
    assert_eq!(policy.title.as_deref(), Some("Firewall Configuration"));

    // Move to second policy (after ID ascending sort: REG-L-003)
    state.move_down();
    let policy = state.get_selected_policy().unwrap();
    assert_eq!(policy.id, "REG-L-003");
    assert_eq!(policy.title.as_deref(), Some("Registry Protection"));
}

#[test]
fn test_dashboard_last_action() {
    let policies = create_test_policies();
    let state = DashboardState::new(policies);

    assert_eq!(state.last_action, "Ready");
}

#[test]
fn test_dashboard_renders_without_panic() {
    let policies = create_test_policies();
    let state = DashboardState::new(policies);
    let multiselect = MultiSelectState::new();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let dashboard = Dashboard::new(&state, &multiselect);
            f.render_widget(dashboard, f.area());
        })
        .unwrap();

    // If we get here without panic, rendering works
    let buffer = terminal.backend().buffer();

    // Check that the buffer contains expected text
    let buffer_str = buffer.content().iter().map(|c| c.symbol()).collect::<String>();
    
    // Should contain policy IDs
    assert!(buffer_str.contains("SSH-H-001"));
    assert!(buffer_str.contains("FW-M-002"));
    
    // Should contain title
    assert!(buffer_str.contains("NoGap Policies"));
}

#[test]
fn test_policy_status_colors() {
    // Test that each status returns appropriate colors
    let pass = PolicyStatus::Pass;
    let fail = PolicyStatus::Fail;
    let warning = PolicyStatus::Warning;
    let unknown = PolicyStatus::Unknown;

    // Normal mode
    assert_eq!(pass.color(false), ratatui::style::Color::Green);
    assert_eq!(fail.color(false), ratatui::style::Color::Red);
    assert_eq!(warning.color(false), ratatui::style::Color::Rgb(251, 146, 60));
    assert_eq!(unknown.color(false), ratatui::style::Color::Gray);

    // High contrast mode
    assert_eq!(pass.color(true), ratatui::style::Color::White);
    assert_eq!(fail.color(true), ratatui::style::Color::White);
    assert_eq!(warning.color(true), ratatui::style::Color::White);
    assert_eq!(unknown.color(true), ratatui::style::Color::Gray);
}

#[test]
fn test_policy_status_symbols() {
    assert_eq!(PolicyStatus::Pass.symbol(), "✓");
    assert_eq!(PolicyStatus::Fail.symbol(), "✗");
    assert_eq!(PolicyStatus::Warning.symbol(), "⚠");
    assert_eq!(PolicyStatus::Unknown.symbol(), "?");
}

// ============================================================================
// STAGE 1 TESTS - Search, Filter, Sort, Scrollbar
// ============================================================================

#[test]
fn test_search_filtering() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // No search - all policies visible
    state.filter = String::new();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 5);
    
    // Search for "ssh" - should match SSH-H-001 and SSH-M-004
    state.filter = "ssh".to_string();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    assert_eq!(state.policies[state.filtered_indices[0]].id, "SSH-H-001");
    assert_eq!(state.policies[state.filtered_indices[1]].id, "SSH-M-004");
    
    // Search for "firewall" - should match FW-M-002
    state.filter = "firewall".to_string();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.policies[state.filtered_indices[0]].id, "FW-M-002");
    
    // Search for nonexistent - should match nothing
    state.filter = "nonexistent".to_string();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 0);
}

#[test]
fn test_severity_filter() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // All severities enabled - all policies visible
    state.policy_filter = PolicyFilter::default();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 5);
    
    // Only high severity - should match SSH-H-001 and UAC-H-005
    state.policy_filter.severity_high = true;
    state.policy_filter.severity_medium = false;
    state.policy_filter.severity_low = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    assert!(state.policies[state.filtered_indices[0]].severity.as_deref() == Some("high"));
    assert!(state.policies[state.filtered_indices[1]].severity.as_deref() == Some("high"));
    
    // Only medium severity - should match FW-M-002 and SSH-M-004
    state.policy_filter.severity_high = false;
    state.policy_filter.severity_medium = true;
    state.policy_filter.severity_low = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    
    // Only low severity - should match REG-L-003
    state.policy_filter.severity_high = false;
    state.policy_filter.severity_medium = false;
    state.policy_filter.severity_low = true;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.policies[state.filtered_indices[0]].id, "REG-L-003");
}

#[test]
fn test_platform_filter() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // All platforms enabled - all policies visible
    state.policy_filter = PolicyFilter::default();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 5);
    
    // Only Linux - should match SSH-H-001 and SSH-M-004
    state.policy_filter.platform_windows = false;
    state.policy_filter.platform_linux = true;
    state.policy_filter.platform_macos = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    assert_eq!(state.policies[state.filtered_indices[0]].platform, "Linux");
    assert_eq!(state.policies[state.filtered_indices[1]].platform, "Linux");
    
    // Only Windows - should match REG-L-003 and UAC-H-005
    state.policy_filter.platform_windows = true;
    state.policy_filter.platform_linux = false;
    state.policy_filter.platform_macos = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    assert_eq!(state.policies[state.filtered_indices[0]].platform, "Windows");
    assert_eq!(state.policies[state.filtered_indices[1]].platform, "Windows");
    
    // Only macOS - should match FW-M-002
    state.policy_filter.platform_windows = false;
    state.policy_filter.platform_linux = false;
    state.policy_filter.platform_macos = true;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.policies[state.filtered_indices[0]].platform, "macOS");
}

#[test]
fn test_combined_filters() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // Search "ssh" + only high severity - should match SSH-H-001
    state.filter = "ssh".to_string();
    state.policy_filter.severity_high = true;
    state.policy_filter.severity_medium = false;
    state.policy_filter.severity_low = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 1);
    assert_eq!(state.policies[state.filtered_indices[0]].id, "SSH-H-001");
    
    // Search "ssh" + only Linux - should match SSH-H-001 and SSH-M-004
    state.filter = "ssh".to_string();
    state.policy_filter = PolicyFilter::default();
    state.policy_filter.platform_windows = false;
    state.policy_filter.platform_linux = true;
    state.policy_filter.platform_macos = false;
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
}

#[test]
fn test_sort_mode_cycling() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.compute_filtered_indices();
    
    // Initial sort mode is IdAscending
    assert!(matches!(state.sort_mode, SortMode::IdAscending));
    
    // First cycle -> SeverityDescending
    state.cycle_sort();
    assert!(matches!(state.sort_mode, SortMode::SeverityDescending));
    
    // Second cycle -> PlatformAscending
    state.cycle_sort();
    assert!(matches!(state.sort_mode, SortMode::PlatformAscending));
    
    // Third cycle -> back to IdAscending
    state.cycle_sort();
    assert!(matches!(state.sort_mode, SortMode::IdAscending));
}

#[test]
fn test_sort_by_id_ascending() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.sort_mode = SortMode::IdAscending;
    state.compute_filtered_indices();
    state.apply_sort();
    
    // Should be sorted: FW-M-002, REG-L-003, SSH-H-001, SSH-M-004, UAC-H-005
    assert_eq!(state.policies[state.filtered_indices[0]].id, "FW-M-002");
    assert_eq!(state.policies[state.filtered_indices[1]].id, "REG-L-003");
    assert_eq!(state.policies[state.filtered_indices[2]].id, "SSH-H-001");
    assert_eq!(state.policies[state.filtered_indices[3]].id, "SSH-M-004");
    assert_eq!(state.policies[state.filtered_indices[4]].id, "UAC-H-005");
}

#[test]
fn test_sort_by_severity_descending() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.sort_mode = SortMode::SeverityDescending;
    state.compute_filtered_indices();
    state.apply_sort();
    
    // Should be sorted: high (SSH-H-001, UAC-H-005), medium (FW-M-002, SSH-M-004), low (REG-L-003)
    let first_two = &state.filtered_indices[0..2];
    let middle_two = &state.filtered_indices[2..4];
    let last_one = state.filtered_indices[4];
    
    // First two should be high severity
    for &idx in first_two {
        assert_eq!(state.policies[idx].severity.as_deref(), Some("high"));
    }
    
    // Middle two should be medium severity
    for &idx in middle_two {
        assert_eq!(state.policies[idx].severity.as_deref(), Some("medium"));
    }
    
    // Last should be low severity
    assert_eq!(state.policies[last_one].severity.as_deref(), Some("low"));
}

#[test]
fn test_sort_by_platform_ascending() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    state.sort_mode = SortMode::PlatformAscending;
    state.compute_filtered_indices();
    state.apply_sort();
    
    // Should be sorted by platform: Linux (2), Windows (2), macOS (1)
    // Linux: SSH-H-001, SSH-M-004
    // Windows: REG-L-003, UAC-H-005
    // macOS: FW-M-002
    
    let platforms: Vec<_> = state.filtered_indices.iter()
        .map(|&idx| state.policies[idx].platform.as_str())
        .collect();
    
    // Check that Linux comes before Windows, Windows before macOS
    let linux_count = platforms.iter().filter(|&&p| p == "Linux").count();
    let windows_start = platforms.iter().position(|&p| p == "Windows").unwrap();
    let macos_start = platforms.iter().position(|&p| p == "macOS").unwrap();
    
    assert_eq!(linux_count, 2);
    assert!(windows_start >= linux_count);
    assert!(macos_start > windows_start);
}

#[test]
fn test_filtered_navigation() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // Filter to only "ssh" - should have 2 results
    state.filter = "ssh".to_string();
    state.compute_filtered_indices();
    assert_eq!(state.filtered_indices.len(), 2);
    
    // Initial selection should be 0
    state.selected = 0;
    assert_eq!(state.selected, 0);
    
    // Move down once
    state.move_down();
    assert_eq!(state.selected, 1);
    
    // Move down again - should clamp at last filtered index
    state.move_down();
    assert_eq!(state.selected, 1); // Stays at 1 (last index in filtered list)
    
    // Move up
    state.move_up();
    assert_eq!(state.selected, 0);
    
    // Move up again - should clamp at 0
    state.move_up();
    assert_eq!(state.selected, 0);
}

#[test]
fn test_get_selected_index_with_filter() {
    let policies = create_test_policies();
    let mut state = DashboardState::new(policies);
    
    // Filter to only "ssh"
    state.filter = "ssh".to_string();
    state.compute_filtered_indices();
    
    // Select first filtered item (should be SSH-H-001 at original index 0)
    state.selected = 0;
    let actual_idx = state.get_selected_index().unwrap();
    assert_eq!(state.policies[actual_idx].id, "SSH-H-001");
    
    // Select second filtered item (should be SSH-M-004 at original index 3)
    state.selected = 1;
    let actual_idx = state.get_selected_index().unwrap();
    assert_eq!(state.policies[actual_idx].id, "SSH-M-004");
}
