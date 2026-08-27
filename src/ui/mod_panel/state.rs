
use crate::ui::mod_bridge::contract::ModPanel;

#[derive(Debug, Clone, Default)]
pub struct PanelState {

    pub selected: usize,

    pub scroll: usize,

    pub active_tab: usize,

    pub active_action: usize,

    last_selected_id: String,
}

#[derive(Debug, Default, Clone)]
pub struct PanelView {

    pub row_y: Vec<(u16, usize)>,

    pub tab_y: Vec<(u16, usize)>,

    pub tab_x: Vec<(u16, u16, u16, usize)>,

    pub action_x: Vec<(u16, u16, u16, usize)>,

    pub carousel_arrows: Vec<(u16, u16, u16, bool)>,
}

impl PanelView {
    pub fn row_at_y(&self, y: u16) -> Option<usize> {
        self.row_y.iter().find(|(ry, _)| *ry == y).map(|(_, i)| *i)
    }

    pub fn tab_at_y(&self, y: u16) -> Option<usize> {
        self.tab_y.iter().find(|(ty, _)| *ty == y).map(|(_, i)| *i)
    }

    pub fn tab_at(&self, x: u16, y: u16) -> Option<usize> {
        self.tab_x
            .iter()
            .find(|(x0, x1, ty, _)| y == *ty && x >= *x0 && x < *x1)
            .map(|(_, _, _, i)| *i)
    }

    pub fn action_at(&self, x: u16, y: u16) -> Option<usize> {
        self.action_x
            .iter()
            .find(|(x0, x1, ay, _)| y == *ay && x >= *x0 && x < *x1)
            .map(|(_, _, _, i)| *i)
    }

    pub fn arrow_at(&self, x: u16, y: u16) -> Option<bool> {
        self.carousel_arrows
            .iter()
            .find(|(x0, x1, ay, _)| y == *ay && x >= *x0 && x < *x1)
            .map(|(_, _, _, next)| *next)
    }
}

impl PanelState {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn visible_rows(height: u16) -> usize {
        height.saturating_sub(3).max(1) as usize
    }

    pub fn reconcile(&mut self, panel: &ModPanel) {
        let len = panel.rows.len();
        if len == 0 {
            self.selected = 0;
            self.scroll = 0;
            return;
        }

        if !self.last_selected_id.is_empty() {
            if let Some(idx) = panel.rows.iter().position(|r| r.id == self.last_selected_id) {
                self.selected = idx;
            } else {
                self.selected = self.selected.min(len - 1);
            }
        } else {
            self.selected = self.selected.min(len - 1);
        }
        self.clamp_scroll(len);
    }

    pub fn select(&mut self, idx: usize, panel: &ModPanel) {
        if !panel.rows.is_empty() {
            self.selected = idx.min(panel.rows.len() - 1);
            self.last_selected_id = panel.rows[self.selected].id.clone();
        }
        self.clamp_scroll(panel.rows.len());
    }

    pub fn next(&mut self, panel: &ModPanel) {
        if !panel.rows.is_empty() {
            self.select((self.selected + 1) % panel.rows.len(), panel);
        }
    }

    pub fn prev(&mut self, panel: &ModPanel) {
        if !panel.rows.is_empty() {
            self.select(if self.selected == 0 {
                panel.rows.len() - 1
            } else {
                self.selected - 1
            }, panel);
        }
    }

    pub fn set_active_tab(&mut self, idx: usize, panel: &ModPanel) {
        if !panel.tabs.is_empty() {
            self.active_tab = idx.min(panel.tabs.len() - 1);
        }
        self.selected = 0;
        self.last_selected_id = String::new();
        self.scroll = 0;
    }

    fn clamp_scroll(&mut self, len: usize) {
        let visible = 20;
        if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if len > 0 && self.scroll >= len {
            self.scroll = len.saturating_sub(1);
        }
    }

    pub fn set_visible(&mut self, visible: usize, len: usize) {
        if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        let max_scroll = len.saturating_sub(visible);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(rows: &[&str]) -> ModPanel {
        ModPanel {
            id: "p".into(),
            rows: rows
                .iter()
                .map(|r| crate::ui::mod_bridge::contract::ModPanelRow {
                    id: r.to_string(),
                    cells: vec![r.to_string()],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn selection_restores_by_stable_id() {
        let mut st = PanelState::new();
        let p = panel(&["a", "b", "c"]);
        st.reconcile(&p);
        st.select(1, &p);

        st.reconcile(&p);
        assert_eq!(st.selected, 1);
        assert_eq!(st.last_selected_id, "b");
    }

    #[test]
    fn selection_clamps_when_row_removed() {
        let mut st = PanelState::new();
        let p = panel(&["a", "b", "c"]);
        st.reconcile(&p);
        st.select(2, &p);
        let p2 = panel(&["a", "b"]);
        st.reconcile(&p2);
        assert_eq!(st.selected, 1);
    }

    #[test]
    fn next_prev_cycle() {
        let mut st = PanelState::new();
        let p = panel(&["a", "b", "c"]);
        st.reconcile(&p);
        st.next(&p);
        assert_eq!(st.selected, 1);
        st.next(&p);
        assert_eq!(st.selected, 2);
        st.next(&p);
        assert_eq!(st.selected, 0);
        st.prev(&p);
        assert_eq!(st.selected, 2);
    }

    #[test]
    fn empty_panel_never_panics() {
        let mut st = PanelState::new();
        let p = panel(&[]);
        st.reconcile(&p);
        st.next(&p);
        st.prev(&p);
        assert_eq!(st.selected, 0);
    }

    #[test]
    fn set_visible_clamps_scroll() {
        let mut st = PanelState::new();
        let p = panel(&["a", "b", "c", "d", "e"]);
        st.reconcile(&p);
        st.select(4, &p);
        st.set_visible(3, 5);
        assert_eq!(st.selected, 4);
        assert_eq!(st.scroll, 2);
    }
}

