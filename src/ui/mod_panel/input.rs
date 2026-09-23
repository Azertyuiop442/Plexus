
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ui::mod_bridge::contract::ModPanel;
use crate::ui::mod_panel::state::PanelState;

#[allow(dead_code)]
pub enum PanelAction {

    Handled,

    Pickup { key: String, row_value: String, panel_id: String },

    Close,

    Maximize,

    Ignored,
}

pub fn handle_key(key: KeyEvent, panel: &ModPanel, st: &mut PanelState) -> PanelAction {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('m') => PanelAction::Maximize,
            _ => PanelAction::Ignored,
        };
    }
    match key.code {
        KeyCode::Up => {
            st.prev(panel);
            PanelAction::Handled
        }
        KeyCode::Down => {
            st.next(panel);
            PanelAction::Handled
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                st.prev(panel);
            }
            PanelAction::Handled
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                st.next(panel);
            }
            PanelAction::Handled
        }
        KeyCode::Home => {
            if !panel.rows.is_empty() {
                st.select(0, panel);
            }
            PanelAction::Handled
        }
        KeyCode::End => {
            if !panel.rows.is_empty() {
                st.select(panel.rows.len() - 1, panel);
            }
            PanelAction::Handled
        }
        KeyCode::Tab => {

            if !panel.tabs.is_empty() {
                let next = (st.active_tab + 1) % panel.tabs.len();
                st.set_active_tab(next, panel);
            }
            PanelAction::Handled
        }
        KeyCode::Right => {

            if !panel.footer.is_empty() {
                st.active_action = (st.active_action + 1) % panel.footer.len();
            }
            PanelAction::Handled
        }
        KeyCode::Left => {

            if !panel.footer.is_empty() {
                st.active_action = if st.active_action == 0 {
                    panel.footer.len() - 1
                } else {
                    st.active_action - 1
                };
            }
            PanelAction::Handled
        }
        KeyCode::Esc => PanelAction::Close,
        KeyCode::Enter => {

            let value = panel
                .rows
                .get(st.selected)
                .map(|r| r.value.clone())
                .unwrap_or_default();
            PanelAction::Pickup {
                key: "enter".into(),
                row_value: value,
                panel_id: panel.id.clone(),
            }
        }
        KeyCode::Char(c) => {

            let k = c.to_string();
            if panel.footer.iter().any(|h| h.key == k) {
                let value = panel
                    .rows
                    .get(st.selected)
                    .map(|r| r.value.clone())
                    .unwrap_or_default();
                PanelAction::Pickup {
                    key: k,
                    row_value: value,
                    panel_id: panel.id.clone(),
                }
            } else {
                PanelAction::Ignored
            }
        }
        _ => PanelAction::Ignored,
    }
}

