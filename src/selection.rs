
use crate::ui::widget::ScrollMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Anchored,
    Dragging,
    Done,
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub anchor: (u32, u16),
    pub cursor: (u32, u16),
    pub viewport_click: (u16, u16),
    pub phase: Phase,
}

impl Selection {
    pub fn anchor(viewport_row: u16, col: u16, metrics: ScrollMetrics) -> Self {
        let abs_row = absolute_row(viewport_row, metrics);
        Self {
            anchor: (abs_row, col),
            cursor: (abs_row, col),
            viewport_click: (viewport_row, col),
            phase: Phase::Anchored,
        }
    }

    pub fn drag(&mut self, viewport_row: u16, col: u16, metrics: ScrollMetrics) {
        let abs_row = absolute_row(viewport_row, metrics);
        self.cursor = (abs_row, col);
        if self.cursor != self.anchor {
            self.phase = Phase::Dragging;
        }
    }

    pub fn finish(&mut self) -> bool {
        if self.phase == Phase::Dragging {
            self.phase = Phase::Done;
            true
        } else {
            false
        }
    }

    pub fn is_visible(&self) -> bool {
        matches!(self.phase, Phase::Dragging | Phase::Done)
    }

    pub fn was_just_click(&self) -> bool {
        self.phase == Phase::Anchored
    }

    pub fn ordered(&self) -> ((u32, u16), (u32, u16)) {
        let (ar, ac) = self.anchor;
        let (cr, cc) = self.cursor;
        if ar < cr || (ar == cr && ac <= cc) {
            ((ar, ac), (cr, cc))
        } else {
            ((cr, cc), (ar, ac))
        }
    }

    pub fn contains(&self, viewport_row: u16, col: u16, metrics: ScrollMetrics) -> bool {
        if !self.is_visible() {
            return false;
        }
        let row = absolute_row(viewport_row, metrics);
        let ((sr, sc), (er, ec)) = self.ordered();
        if row < sr || row > er {
            return false;
        }
        if sr == er {
            col >= sc && col <= ec
        } else if row == sr {
            col >= sc
        } else if row == er {
            col <= ec
        } else {
            true
        }
    }
}

pub fn absolute_row(viewport_row: u16, metrics: ScrollMetrics) -> u32 {
    let top_row = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom) as u32;
    top_row + viewport_row as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_lifecycle_and_contains() {
        let metrics = ScrollMetrics {
            max_offset_from_bottom: 100,
            offset_from_bottom: 20,
            viewport_rows: 24,
        };

        let mut sel = Selection::anchor(2, 5, metrics);
        assert!(!sel.is_visible());
        assert!(sel.was_just_click());

        sel.drag(4, 15, metrics);
        assert!(sel.is_visible());
        assert!(!sel.was_just_click());

        assert!(sel.contains(2, 5, metrics));
        assert!(sel.contains(2, 20, metrics));

        assert!(sel.contains(3, 0, metrics));
        assert!(sel.contains(3, 50, metrics));

        assert!(sel.contains(4, 15, metrics));
        assert!(!sel.contains(4, 16, metrics));

        assert!(sel.finish());
    }
}

