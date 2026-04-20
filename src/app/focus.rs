/// Grid-based UI focus model for cursor key navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusPosition {
    pub row: usize,
    pub col: usize,
}

impl FocusPosition {
    /// Create a new focus position at (row, col)
    pub fn new(row: usize, col: usize) -> Self {
        FocusPosition { row, col }
    }

    /// Move focus up one row, clamping to 0
    pub fn move_up(&self) -> Self {
        FocusPosition {
            row: self.row.saturating_sub(1),
            col: self.col,
        }
    }

    /// Move focus down one row, clamping to max_row
    pub fn move_down(&self, max_row: usize) -> Self {
        FocusPosition {
            row: (self.row + 1).min(max_row),
            col: self.col,
        }
    }

    /// Move focus left one column, clamping to 0
    pub fn move_left(&self) -> Self {
        FocusPosition {
            row: self.row,
            col: self.col.saturating_sub(1),
        }
    }

    /// Move focus right one column, clamping to max_col for this row
    pub fn move_right(&self, max_col: usize) -> Self {
        FocusPosition {
            row: self.row,
            col: (self.col + 1).min(max_col),
        }
    }

    /// Check if focus is on a slider row (row 0 or 1)
    pub fn is_on_slider(&self) -> bool {
        self.row == 0 || self.row == 1
    }

    /// Check if focus is on waveform buttons (row 2)
    pub fn is_on_waveform(&self) -> bool {
        self.row == 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_focus_position() {
        let pos = FocusPosition::new(1, 2);
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 2);
    }

    #[test]
    fn test_move_up() {
        let pos = FocusPosition::new(2, 1);
        let new_pos = pos.move_up();
        assert_eq!(new_pos.row, 1);
        assert_eq!(new_pos.col, 1);
    }

    #[test]
    fn test_move_up_clamps_to_zero() {
        let pos = FocusPosition::new(0, 0);
        let new_pos = pos.move_up();
        assert_eq!(new_pos.row, 0);
        assert_eq!(new_pos.col, 0);
    }

    #[test]
    fn test_move_down() {
        let pos = FocusPosition::new(0, 1);
        let new_pos = pos.move_down(2);
        assert_eq!(new_pos.row, 1);
        assert_eq!(new_pos.col, 1);
    }

    #[test]
    fn test_move_down_clamps_to_max() {
        let pos = FocusPosition::new(2, 0);
        let new_pos = pos.move_down(2);
        assert_eq!(new_pos.row, 2);
        assert_eq!(new_pos.col, 0);
    }

    #[test]
    fn test_move_left() {
        let pos = FocusPosition::new(1, 2);
        let new_pos = pos.move_left();
        assert_eq!(new_pos.row, 1);
        assert_eq!(new_pos.col, 1);
    }

    #[test]
    fn test_move_left_clamps_to_zero() {
        let pos = FocusPosition::new(0, 0);
        let new_pos = pos.move_left();
        assert_eq!(new_pos.row, 0);
        assert_eq!(new_pos.col, 0);
    }

    #[test]
    fn test_move_right() {
        let pos = FocusPosition::new(2, 0);
        let new_pos = pos.move_right(3);
        assert_eq!(new_pos.row, 2);
        assert_eq!(new_pos.col, 1);
    }

    #[test]
    fn test_move_right_clamps_to_max() {
        let pos = FocusPosition::new(2, 3);
        let new_pos = pos.move_right(3);
        assert_eq!(new_pos.row, 2);
        assert_eq!(new_pos.col, 3);
    }

    #[test]
    fn test_is_on_slider_frequency() {
        let pos = FocusPosition::new(0, 0);
        assert!(pos.is_on_slider());
    }

    #[test]
    fn test_is_on_slider_volume() {
        let pos = FocusPosition::new(1, 0);
        assert!(pos.is_on_slider());
    }

    #[test]
    fn test_is_on_waveform() {
        let pos = FocusPosition::new(2, 1);
        assert!(pos.is_on_waveform());
    }

    #[test]
    fn test_is_on_slider_returns_false_for_waveform() {
        let pos = FocusPosition::new(2, 0);
        assert!(!pos.is_on_slider());
    }

    #[test]
    fn test_is_on_waveform_returns_false_for_sliders() {
        let pos = FocusPosition::new(0, 0);
        assert!(!pos.is_on_waveform());
    }
}
