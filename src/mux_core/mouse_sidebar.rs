use std::io;

use super::modals::{
    open_ai_prefs_modal, open_all_sessions_modal, open_full_config_modal,
    open_mod_config_modal, open_update_progress_modal,
};
use super::nav::{reload_mux, send_slash_command};
use super::pane_ops::{active_pane_size, spawn_pane};
use crate::state::AppState;
use crate::ui::sidebar::{session_resumable, LiveBlock, SettingsSubMenu, SidebarRow};

pub fn handle_sidebar_click(
    state: &mut AppState,
    sidebar_row: SidebarRow,
    command: &str,
    new_tab_cmd: &str,
) -> io::Result<()> {
    state.sidebar.selected = state.sidebar.selection_index(sidebar_row);
    match sidebar_row {
        SidebarRow::NewSession => {
            let (cols, rows) = active_pane_size(state);
            let mut tab_cmd = new_tab_cmd.to_string();
            if state.sidebar.yolo_mode && !tab_cmd.contains("--yolo") {
                tab_cmd.push_str(" --yolo");
            }
            if spawn_pane(state, &tab_cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.sidebar_focus = false;
            }
        }
        SidebarRow::Session(i) => {
            let already_open = state.sidebar.sessions.get(i).and_then(|s| {
                if state.session_is_open(&s.id) {
                    return state.panes.iter().position(|p| {
                        p.lock()
                            .map(|g| g.is_session_live(&s.id))
                            .unwrap_or(false)
                    });
                }
                None
            });
            if let Some(pane_idx) = already_open {
                state.focus_tab(pane_idx);
                state.sidebar_focus = false;
                return Ok(());
            }
            let mut cmd = if let Some(session) = state.sidebar.sessions.get(i) {
                let session_id = session.id.clone();
                if !session_resumable(&session_id, &state.sidebar.project) {
                    state.sidebar.remove_session_by_id(&session_id);
                    command.to_string()
                } else {
                    let cwd = state.sidebar.project_cwd.clone();
                    super::nav::session_launch_cmd(&cwd, &session_id)
                        .unwrap_or_else(|| command.to_string())
                }
            } else {
                command.to_string()
            };

            if state.sidebar.yolo_mode && !cmd.contains("--yolo") {
                cmd.push_str(" --yolo");
            }
            let (cols, rows) = active_pane_size(state);
            if spawn_pane(state, &cmd, cols, rows).is_ok() {
                state.active = state.panes.len() - 1;
                state.sidebar_focus = false;
            }
        }
        SidebarRow::UsageCarousel => {
            state.sidebar.next_usage_tab();
        }
        SidebarRow::UsagePrev => {
            state.sidebar.prev_usage_tab();
        }
        SidebarRow::UsageNext => {
            state.sidebar.next_usage_tab();
        }
        SidebarRow::MoreSessions => {
            open_all_sessions_modal(state);
        }
        SidebarRow::NavPreferences => {
            state.sidebar.open_submenu(SettingsSubMenu::Preferences);
        }
        SidebarRow::NavModConfig => {
            state.sidebar.open_submenu(SettingsSubMenu::ModConfig);
        }
        SidebarRow::NavAIPrefs => {
            open_ai_prefs_modal(state);
        }
        SidebarRow::NavBack => {
            state.sidebar.open_submenu(SettingsSubMenu::Main);
        }
        SidebarRow::PrefFullConfig => {
            open_full_config_modal(state);
        }
        SidebarRow::PrefAutoRetry => {
            crate::ui::modal::open_auto_retry_modal(state);
        }
        SidebarRow::PrefSkills => {
            crate::ui::modal::open_skills_modal_fresh(state);
        }
        SidebarRow::PrefSkillInjection => {
            let mut prefs = crate::prefs::Prefs::load();
            prefs.skill_injection = !prefs.skill_injection;
            prefs.skills.injection_enabled = prefs.skill_injection;
            let _ = prefs.save();
            state.sidebar.skill_injection = prefs.skill_injection;
            state.dirty = true;
        }
        SidebarRow::PrefYolo => {
            state.sidebar.yolo_mode = !state.sidebar.yolo_mode;
            crate::prefs::Prefs::set_yolo_mode(state.sidebar.yolo_mode);
            state.sidebar_focus = false;
        }
        SidebarRow::PrefShowUsage => {
            state.sidebar.show_usage = !state.sidebar.show_usage;
            state.sidebar.rebuild_rows();
            state.dirty = true;
        }
        SidebarRow::PrefSounds => {
            crate::ui::modal::open_sounds_modal(state);
        }
        SidebarRow::PrefWebhook => {
            crate::ui::modal::open_webhook_modal(state);
        }
        SidebarRow::ModConfig(idx) => {
            open_mod_config_modal(state, idx);
        }
        SidebarRow::Reload => {
            reload_mux();
        }
        SidebarRow::BugReport => {
            crate::ui::links::open_bug_report_url();
        }
        SidebarRow::Twitter => {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg("https://x.com/astra442")
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg("https://x.com/astra442")
                    .spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "", "https://x.com/astra442"])
                    .spawn();
            }
        }
        SidebarRow::Update => {
            state.sidebar.available_update = None;
            open_update_progress_modal(state, "Starting update...", 5, 100);
            crate::update::perform_update_with_events(state.events.clone());
            state.dirty = true;
        }
        SidebarRow::LiveBlockOpen(_) => {
            let open_path = state
                .sidebar
                .live_blocks
                .iter()
                .find(|b| b.open_path.is_some())
                .and_then(|b| b.open_path.clone());
            if let Some(pth) = open_path {
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").arg(&pth).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &pth])
                        .spawn();
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    let _ = std::process::Command::new("xdg-open").arg(&pth).spawn();
                }
            }
            state.sidebar_focus = false;
        }
        SidebarRow::LiveBlockDismiss(_) => {
            let dismiss_id = state.sidebar.live_blocks.first().map(|b| b.id.clone());
            if let Some(id) = dismiss_id {
                LiveBlock::dismiss(&id);
                state.sidebar.live_blocks.retain(|b| b.id != id);
                state.dirty = true;
            }
            state.sidebar_focus = false;
        }
        SidebarRow::LiveBlockCopy(_) => {
            let copy_text = state
                .sidebar
                .live_blocks
                .iter()
                .find(|b| b.copy_text.is_some())
                .and_then(|b| b.copy_text.clone());
            if let Some(txt) = copy_text {
                super::nav::copy_to_clipboard(&txt);
            }
            state.sidebar_focus = false;
        }
        SidebarRow::LiveBlockResume(_) => {
            let resume = state
                .sidebar
                .live_blocks
                .iter()
                .find(|b| b.resume_command.is_some())
                .and_then(|b| b.resume_command.clone());
            if let Some(cmd) = resume {
                let session_id = state
                    .sidebar
                    .live_blocks
                    .iter()
                    .find(|b| !b.session_id.is_empty())
                    .map(|b| b.session_id.clone())
                    .unwrap_or_default();
                let run_pane = state.panes.iter().position(|p| {
                    p.lock()
                        .map(|g| {
                            g.state.session_id.as_deref() == Some(session_id.as_str())
                                || (!session_id.is_empty()
                                    && (g.state.launch_cmd.contains(
                                        &format!("--session {session_id}"),
                                    ) || g.state.launch_cmd.contains(&format!(
                                        "--resume {session_id}"
                                    ))))
                        })
                        .unwrap_or(false)
                });
                let target_pane = run_pane
                    .or_else(|| (state.active < state.panes.len()).then_some(state.active))
                    .unwrap_or(0);
                if let Some(pane_arc) = state.panes.get(target_pane) {
                    if let Ok(mut p) = pane_arc.lock() {
                        send_slash_command(&mut p, &cmd);
                    }
                }
                state.focus_tab(target_pane);
            }
            state.sidebar_focus = false;
        }
        SidebarRow::RightSidebar(idx) => {
            if state.panel_active && state.active_right_sidebar == idx {
                if state.panel_sidebar_open {
                    state.panel_active = false;
                    state.panel_sidebar_open = false;
                    state.panel_maximized = false;
                    state.panel_focused = false;
                } else {
                    state.panel_sidebar_open = true;
                    state.panel_maximized = false;
                    state.refresh_mods_now();
                }
            } else {
                state.panel_active = true;
                state.active_right_sidebar = idx;
                state.panel_sidebar_open = true;
                state.panel_maximized = false;
                state.refresh_mods_now();
            }
            state.sync_sidebar_panel();
            state.dirty = true;
            state.sidebar_focus = false;
        }
    }
    Ok(())
}
