/// Multi-select component for batch policy operations
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct MultiSelectState {
    pub active: bool,
    pub selected: HashSet<usize>, // Policy indices (actual indices, not filtered)
}

impl MultiSelectState {
    pub fn new() -> Self {
        Self {
            active: false,
            selected: HashSet::new(),
        }
    }

    pub fn enter_mode(&mut self) {
        self.active = true;
    }

    pub fn exit_mode(&mut self) {
        self.active = false;
        self.selected.clear();
    }

    pub fn toggle(&mut self, index: usize) {
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }

    pub fn select_all(&mut self, indices: &[usize]) {
        self.selected.extend(indices.iter().copied());
    }

    pub fn clear_all(&mut self) {
        self.selected.clear();
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.contains(&index)
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn get_selected_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self.selected.iter().copied().collect();
        indices.sort_unstable();
        indices
    }
}

impl Default for MultiSelectState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_multiselect() {
        let state = MultiSelectState::new();
        assert!(!state.active);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_enter_exit_mode() {
        let mut state = MultiSelectState::new();

        state.enter_mode();
        assert!(state.active);

        state.exit_mode();
        assert!(!state.active);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_toggle_selection() {
        let mut state = MultiSelectState::new();
        state.enter_mode();

        state.toggle(0);
        assert!(state.is_selected(0));
        assert_eq!(state.selected_count(), 1);

        state.toggle(0);
        assert!(!state.is_selected(0));
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_select_all() {
        let mut state = MultiSelectState::new();
        state.enter_mode();

        let indices = vec![0, 1, 2, 3];
        state.select_all(&indices);

        assert_eq!(state.selected_count(), 4);
        assert!(state.is_selected(0));
        assert!(state.is_selected(3));
    }

    #[test]
    fn test_clear_all() {
        let mut state = MultiSelectState::new();
        state.enter_mode();

        state.toggle(0);
        state.toggle(1);
        assert_eq!(state.selected_count(), 2);

        state.clear_all();
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_get_selected_indices_sorted() {
        let mut state = MultiSelectState::new();
        state.enter_mode();

        state.toggle(3);
        state.toggle(1);
        state.toggle(5);

        let indices = state.get_selected_indices();
        assert_eq!(indices, vec![1, 3, 5]);
    }
}
