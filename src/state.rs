
use std::sync::{Arc, Mutex};

use crate::mux_events::{event_bus, MuxEventReceiver, MuxEventSender};
use crate::ui::mod_bridge::ModsData;
use crate::ui::modal::{Modal, ModelPicker};
use crate::ui::pane::MuxPane;
use crate::ui::sidebar::{Sidebar, SidebarView};

pub struct PickerState {

    pub row_idx: usize,
    pub picker: ModelPicker,
}

pub struct AppState {
    pub panes: Vec<Arc<Mutex<MuxPane>>>,
    pub active: usize,
    pub sidebar: Sidebar,

    pub sidebar_view: SidebarView,
    pub sidebar_open: bool,
    pub sidebar_focus: bool,
    pub sidebar_w: u16,
    pub active_modal: Option<Modal>,
    pub picker: Option<PickerState>,
    pub context_menu: Option<crate::ui::context_menu::ContextMenu>,
    pub cmd_inspect: Option<crate::ui::cmdinfo::CmdInspectState>,
    pub switcher: Option<crate::ui::switcher::SwitcherState>,
    pub hover_divider: Option<crate::ui::borders::HoverDivider>,
    pub last_click_tab: Option<usize>,
    pub last_click_time: Option<std::time::Instant>,

    pub confirm_close_idx: Option<usize>,

    pub finder: Option<crate::ui::search::FinderState>,

    pub mods_data: ModsData,
    pub last_mods_refresh: std::time::Instant,

    pub started_at_ms: u64,

    pub(crate) last_model_check: std::time::Instant,
    cached_model: Option<String>,
    cached_effort: Option<String>,

    pub available_update: Option<String>,

    pub dirty: bool,

    pub events: MuxEventSender,

    event_rx: Option<MuxEventReceiver>,

    pub dictation: Option<(std::time::Instant, String)>,

    pub scroll_physics: crate::scroll_physics::ScrollState,

    pub next_pane_gen: u64,

    pub last_pickup_seq: Option<u64>,
    pub panel_state: crate::ui::mod_panel::PanelState,

    pub panel_view: crate::ui::mod_panel::PanelView,

    pub panel_sidebar_open: bool,

    pub panel_maximized: bool,

    pub active_right_sidebar: usize,

    pub panel_focused: bool,

    pub panel_sidebar_w: u16,

    pub resizing_panel: bool,

    pub resizing_sidebar: bool,
}

pub const PANEL_SIDEBAR_MAX: u16 = 60;

pub const PANEL_SIDEBAR_MIN: u16 = 20;

const MODEL_CHECK_MS: u64 = 500;

const REFRESH_MODS_MIN_MS: u64 = 200;

impl AppState {
    pub fn new(sidebar: Sidebar) -> Self {
        let (events, event_rx) = event_bus();
        Self {
            panes: Vec::new(),
            active: 0,
            sidebar,
            sidebar_view: SidebarView::default(),
            sidebar_open: true,
            sidebar_focus: false,
            sidebar_w: 25,
            active_modal: None,
            picker: None,
            context_menu: None,
            cmd_inspect: None,
            switcher: None,
            hover_divider: None,
            last_click_tab: None,
            last_click_time: None,
            confirm_close_idx: None,
            finder: None,
            mods_data: ModsData::default(),
            last_pickup_seq: None,
            last_mods_refresh: std::time::Instant::now(),
            started_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            last_model_check: std::time::Instant::now()
                - std::time::Duration::from_millis(MODEL_CHECK_MS + 1),
            cached_model: None,
            cached_effort: None,
            available_update: None,
            dirty: true,
            events,
            event_rx: Some(event_rx),
            dictation: None,
            scroll_physics: crate::scroll_physics::ScrollState::new(),
            next_pane_gen: 0,
            panel_state: crate::ui::mod_panel::PanelState::new(),
            panel_view: crate::ui::mod_panel::PanelView::default(),
            panel_sidebar_open: false,
            panel_maximized: false,
            active_right_sidebar: 0,
            panel_focused: false,
            panel_sidebar_w: 25,
            resizing_panel: false,
            resizing_sidebar: false,
        }
    }

    pub fn take_events(&mut self) -> Option<MuxEventReceiver> {
        self.event_rx.take()
    }

    pub fn refresh_model_cache(&mut self) {
        if self.last_model_check.elapsed() < std::time::Duration::from_millis(MODEL_CHECK_MS) {
            return;
        }
        self.last_model_check = std::time::Instant::now();
        let model = self.mods_data.active_model();
        let effort = model.as_deref().and_then(|m| self.mods_data.active_effort(m));
        self.cached_model = model;
        self.cached_effort = effort;
    }

    pub fn model_info(&self) -> (Option<String>, Option<String>) {
        (self.cached_model.clone(), self.cached_effort.clone())
    }

    pub fn close_pane(&mut self, idx: usize) {
        if idx < self.panes.len() {
            if let Some(pane) = self.panes.get(idx) {
                pane.lock().unwrap_or_else(|e| e.into_inner()).kill();
            }
            self.panes.remove(idx);
            self.active = self.active.min(self.panes.len().saturating_sub(1));
            self.sync_pane_count();
            self.refresh_mods_now();
        }
    }

    pub fn sync_pane_count(&mut self) {
        let n = self.panes.len();
        for p in &self.panes {
            if let Ok(mut g) = p.lock() {
                g.state.pane_count = n;
            }
        }
    }

    pub fn refresh_mods_now(&mut self) {
        if self.last_mods_refresh.elapsed() < std::time::Duration::from_millis(REFRESH_MODS_MIN_MS)
        {
            return;
        }
        let tty = self
            .panes
            .get(self.active)
            .and_then(|p| p.lock().ok())
            .map(|p| p.state.tty_name.clone())
            .filter(|t| !t.is_empty());
        self.mods_data = crate::ui::mod_bridge::ModsData::load_with_tty(tty.as_deref());
        self.last_mods_refresh = std::time::Instant::now();
    }

    pub fn clamp_active(&mut self) {
        self.active = self.active.min(self.panes.len().saturating_sub(1));
    }

    pub fn session_is_open(&self, session_id: &str) -> bool {
        let in_panes = self.panes.iter().any(|p| {
            p.lock()
                .map(|g| {
                    g.state.launch_cmd.contains(&format!("--session {}", session_id))
                        || g.state.launch_cmd.contains(&format!("--resume {}", session_id))
                        || g.state.session_id.as_deref() == Some(session_id)
                })
                .unwrap_or(false)
        });
        if in_panes {
            return true;
        }

        let safe: String = session_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = crate::ipc::ipc_path(&format!("agent_status-{safe}.json"));
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return false;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return false;
        };
        let Some(updated_at) = json.get("updatedAt").and_then(|v| v.as_u64()) else {
            return false;
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms.saturating_sub(updated_at) <= 30_000
    }

    pub fn focus_tab(&mut self, idx: usize) {
        if idx >= self.panes.len() {
            return;
        }
        if self.active != idx {
            self.active = idx;
            self.refresh_mods_now();
            self.dirty = true;
        }
    }
}

