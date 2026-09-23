use super::pane_ops::{active_pane_size, spawn_pane};
use crate::state::AppState;
use crate::ui::context_menu::ContextMenuAction;
use crate::ui::modal::{Modal, ModalRow};

pub fn execute_context_menu_action(
    state: &mut AppState,
    action: ContextMenuAction,
    new_tab_cmd: &str,
) {
    match action {
        ContextMenuAction::TabRename(tab_idx) => {
            if let Some(pane) = state.panes.get(tab_idx) {
                let current_title = pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
                let mut m = Modal::new("rename_tab", format!("Rename Terminal #{}", tab_idx + 1));
                m.rows.push(ModalRow::TextInput {
                    key: "title".into(),
                    label: "Title".into(),
                    value: if current_title == "commandcode" { String::new() } else { current_title },
                });
                m.rows.push(ModalRow::Info("Type new name, press ENTER to save or ESC to cancel".into()));
                state.active_modal = Some(m);
            }
        }
        ContextMenuAction::TabDuplicate(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let mut tab_cmd = new_tab_cmd.to_string();
            if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                tab_cmd.push_str(" --yolo");
            }
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::TabClose(tab_idx) => {
            if state.panes.len() > 1 {
                state.close_pane(tab_idx);
            }
        }
        ContextMenuAction::TabSplitRight(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let tab_cmd = new_tab_cmd.to_string();
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::TabSplitDown(_tab_idx) => {
            let (cols, rows) = active_pane_size(state);
            let tab_cmd = new_tab_cmd.to_string();
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::SessionOpen(id) => {
            let (cols, rows) = active_pane_size(state);
            let cmd = format!("{} --resume {}", new_tab_cmd, id);
            if spawn_pane(state, &cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.refresh_mods_now();
            }
        }
        ContextMenuAction::SessionDelete(id) => {
            if let Some(pos) = state.sidebar.sessions.iter().position(|s| s.id == id) {
                state.sidebar.delete_session(pos);
            }
            state.dirty = true;
        }
        ContextMenuAction::SessionCopyId(id) => {
            super::nav::copy_to_clipboard(&id);
        }
        ContextMenuAction::PaneInspect(pane_idx) => {
            super::input::open_cmd_inspect(state, pane_idx);
        }
        ContextMenuAction::PaneClear(pane_idx) => {
            if let Some(pane) = state.panes.get(pane_idx) {
                if let Ok(mut p) = pane.lock() {
                    p.write_input(b"\x0c");
                }
            }
        }
        ContextMenuAction::PaneScrollback(pane_idx) => {
            if let Some(pane) = state.panes.get(pane_idx) {
                if let Ok(mut p) = pane.lock() {
                    p.scroll_display(10);
                }
            }
        }
    }
}
