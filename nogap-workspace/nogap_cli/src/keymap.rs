/// Centralized keybindings and help text for the NoGap TUI

use crossterm::event::{KeyCode, KeyModifiers};

pub struct KeyMap;

impl KeyMap {
    /// Get help text for all keybindings
    pub fn help_text() -> Vec<(&'static str, &'static str)> {
        vec![
            ("j/↓", "Move down"),
            ("k/↑", "Move up"),
            ("/", "Search policies"),
            ("f", "Open filter modal"),
            ("o", "Cycle sort mode"),
            ("s", "Browse snapshots"),
            ("a", "Run audit"),
            ("r", "Remediate"),
            ("d", "View diff"),
            ("t", "Toggle high-contrast"),
            ("?", "Show help"),
            ("q/Esc", "Quit/Close"),
        ]
    }

    /// Check if key is quit
    pub fn is_quit(code: KeyCode, modifiers: KeyModifiers) -> bool {
        matches!(code, KeyCode::Char('q') | KeyCode::Esc)
            || (matches!(code, KeyCode::Char('c')) && modifiers.contains(KeyModifiers::CONTROL))
    }

    /// Check if key is help
    pub fn is_help(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('?'))
    }

    /// Check if key is down
    pub fn is_down(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('j') | KeyCode::Down)
    }

    /// Check if key is up
    pub fn is_up(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('k') | KeyCode::Up)
    }

    /// Check if key is audit
    pub fn is_audit(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('a'))
    }

    /// Check if key is remediate
    pub fn is_remediate(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('r'))
    }

    /// Check if key is diff
    pub fn is_diff(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('d'))
    }

    /// Check if key is search
    pub fn is_search(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('/'))
    }

    /// Check if key is toggle filters
    pub fn is_toggle_filter(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('f'))
    }

    /// Check if key is sort
    pub fn is_sort(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('o'))
    }

    /// Check if key is snapshots
    pub fn is_snapshot(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('s'))
    }

    /// Check if key is toggle theme
    pub fn is_toggle_theme(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char('t'))
    }

    /// Check if key is confirm (Enter/y)
    pub fn is_confirm(code: KeyCode) -> bool {
        matches!(code, KeyCode::Enter | KeyCode::Char('y'))
    }

    /// Check if key is space (for toggling checkboxes)
    pub fn is_space(code: KeyCode) -> bool {
        matches!(code, KeyCode::Char(' '))
    }
}
