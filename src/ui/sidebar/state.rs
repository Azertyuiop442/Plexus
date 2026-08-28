
use std::fs;
use std::path::Path;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::models::{data_dir as bridge_data_dir,
    load_mods, SettingsSubMenu, SidebarRow, SessionEntry, SessionsFile, SESSIONS_SHOWN,
};

pub fn session_title(title: &str, max: usize) -> String {
    let trimmed = title.trim();
    if UnicodeWidthStr::width(trimmed) <= max {
        return trimmed.to_string();
    }
    let mut out = String::new();
    let mut width = 0;
    for ch in trimmed.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + w + 1 > max {
            break;
        }
        out.push(ch);
        width += w;
    }
    out.push('…');
    out
}

pub fn sort_sessions_by_activity(sessions: &mut [SessionEntry]) {
    sessions.sort_by(|a, b| b.last_at.cmp(&a.last_at));
}

pub fn session_resumable(session_id: &str, project: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    let home = crate::ipc::home_dir();
    if home.as_os_str().is_empty() {
        return false;
    }
    let path = home
        .join(".commandcode/projects")
        .join(project)
        .join(format!("{}.jsonl", session_id));
    match fs::metadata(&path) {
        Ok(md) => md.len() > 0,
        Err(_) => false,
    }
}

#[derive(Debug, Clone)]
pub struct LiveBlock {

    pub id: String,

    pub label: String,

    #[allow(dead_code)]
    pub phase: String,

    pub agents: Vec<LiveAgent>,

    pub terminal: usize,

    pub done: bool,

    pub aborted: bool,

    pub stalled: bool,

    pub hint: Option<String>,

    pub open_path: Option<String>,

    pub copy_text: Option<String>,

    pub resume_command: Option<String>,

    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct LiveAgent {
    pub label: String,

    pub status: String,
}

impl LiveBlock {

    pub fn is_dismissed(id: &str) -> bool {
        fs::read_to_string(Path::new(&crate::ui::sidebar::models::data_dir()).join("live-dismissed.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|j| {
                j.get("ids")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().any(|r| r.as_str() == Some(id)))
            })
            .unwrap_or(false)
    }

    pub fn dismiss(id: &str) {
        let path = Path::new(&crate::ui::sidebar::models::data_dir()).join("live-dismissed.json");
        let mut ids: Vec<String> = fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|j| {
                j.get("ids").and_then(|v| v.as_array()).map(|a| {
                    a.iter()
                        .filter_map(|r| r.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();
        if !ids.iter().any(|r| r == id) {
            ids.push(id.to_string());
        }
        let _ = crate::ipc::atomic_write(&path, &serde_json::json!({ "ids": ids }).to_string());
    }
}

pub struct Sidebar {
    pub project: String,

    pub project_cwd: String,
    pub sessions: Vec<SessionEntry>,
    pub mods: Vec<crate::ui::sidebar::models::ModItem>,
    pub rows: Vec<SidebarRow>,
    pub selected: usize,
    pub expanded: bool,
    pub scroll: usize,
    pub view_lines: usize,
    pub active_tab: usize,
    pub settings_menu: SettingsSubMenu,
    pub yolo_mode: bool,
    pub taste_learning: bool,
    pub ide_context: bool,
    pub show_cost_bar: bool,
    pub show_context_btn: bool,
    pub show_usage: bool,

    pub live_blocks: Vec<LiveBlock>,
    pub available_update: Option<String>,
    pub usage: Option<crate::usage::UsageData>,
    pub usage_tab: usize,
}

impl Sidebar {

    pub fn next_usage_tab(&mut self) {
        self.usage_tab = (self.usage_tab + 1) % 3;
    }

    pub fn prev_usage_tab(&mut self) {
        self.usage_tab = if self.usage_tab == 0 { 2 } else { self.usage_tab - 1 };
    }

    pub fn remove_session_by_id(&mut self, id: &str) {
        let before = self.sessions.len();
        self.sessions.retain(|s| s.id != id);
        if self.sessions.len() != before {
            self.save_sessions();
            self.rebuild_rows();
        }
    }

    pub fn selection_index(&self, clicked: SidebarRow) -> usize {
        self.rows
            .iter()
            .position(|r| *r == clicked)
            .unwrap_or(self.selected)
    }

    pub fn load() -> Self {
        let file = SessionsFile::load();
        let mut valid_sessions: Vec<SessionEntry> = file
            .sessions
            .into_iter()
            .filter(|s| {
                let t = s.title.trim();
                !t.is_empty() && t != "Untitled" && t != "commandcode"
            })
            .collect();
        sort_sessions_by_activity(&mut valid_sessions);
        let mut s = Sidebar {
            project: file.project.clone(),
            project_cwd: file.cwd.clone(),
            sessions: valid_sessions,
            mods: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            expanded: false,
            scroll: 0,
            view_lines: 10,
            active_tab: 0,
            settings_menu: SettingsSubMenu::Main,
            yolo_mode: false,
            taste_learning: true,
            ide_context: true,
            show_cost_bar: true,
            show_context_btn: true,
            show_usage: true,
            live_blocks: Vec::new(),
            available_update: None,
            usage: crate::usage::load_cached_usage(),
            usage_tab: 0,
        };

        let prefs = crate::prefs::Prefs::load();
        s.yolo_mode = prefs.yolo_mode;
        s.taste_learning = prefs.taste_learning;
        s.ide_context = prefs.ide_context;
        s.show_cost_bar = prefs.show_cost_bar;
        s.show_context_btn = prefs.show_context_btn;
        s.show_usage = prefs.show_usage;
        let config = fs::read_to_string(Path::new(&bridge_data_dir()).join("config.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
        s.mods = load_mods(config.as_ref());
        s.rebuild_rows();
        s
    }

    pub fn refresh(&mut self) {
        let file = SessionsFile::load();
        self.project = file.project.clone();
        self.project_cwd = file.cwd.clone();
        self.sessions = file
            .sessions
            .into_iter()
            .filter(|s| {
                let t = s.title.trim();
                !t.is_empty()
                    && t != "Untitled"
                    && t != "commandcode"
                    && !t.contains("<command-")
                    && !t.contains("</command-")
            })
            .collect();
        sort_sessions_by_activity(&mut self.sessions);
        let config = fs::read_to_string(Path::new(&bridge_data_dir()).join("config.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
        self.mods = load_mods(config.as_ref());
        self.rebuild_rows();
    }

    pub fn set_active_tab(&mut self, tab: usize) {
        self.active_tab = tab;
        self.settings_menu = SettingsSubMenu::Main;
        self.selected = 0;
        self.scroll = 0;
        self.rebuild_rows();
    }

    pub fn open_submenu(&mut self, menu: SettingsSubMenu) {
        self.settings_menu = menu;
        self.selected = 0;
        self.scroll = 0;
        self.rebuild_rows();
    }

    pub fn rebuild_rows(&mut self) {
        self.rows.clear();
        match self.settings_menu {
            SettingsSubMenu::Main => {
                self.rows.push(SidebarRow::NavPreferences);
                self.rows.push(SidebarRow::NavModConfig);
                self.rows.push(SidebarRow::NavAIPrefs);
                self.rows.push(SidebarRow::Reload);

                self.rows.push(SidebarRow::NewSession);
                if !self.sessions.is_empty() {
                    let shown = if self.expanded {
                        self.sessions.len()
                    } else {
                        self.sessions.len().min(SESSIONS_SHOWN)
                    };
                    for i in 0..shown {
                        self.rows.push(SidebarRow::Session(i));
                    }
                    self.rows.push(SidebarRow::MoreSessions);
                }

                if self.usage.is_some() && self.show_usage {
                    self.rows.push(SidebarRow::UsageCarousel);
                }
            }
            SettingsSubMenu::Preferences => {
                self.rows.push(SidebarRow::NavBack);
                self.rows.push(SidebarRow::PrefFullConfig);
                self.rows.push(SidebarRow::PrefYolo);
                self.rows.push(SidebarRow::PrefShowUsage);
            }
            SettingsSubMenu::ModConfig => {
                self.rows.push(SidebarRow::NavBack);
                for i in 0..self.mods.len() {
                    self.rows.push(SidebarRow::ModConfig(i));
                }
            }
        }
        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
    }

    fn clamp_scroll(&mut self) {
        let visible = self.view_lines.max(1);
        if self.selected >= self.scroll + visible {
            self.scroll = self.selected + 1 - visible;
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
    }

    pub fn next(&mut self) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + 1) % self.rows.len();
            self.clamp_scroll();
        }
    }

    pub fn prev(&mut self) {
        if !self.rows.is_empty() {
            self.selected = if self.selected == 0 {
                self.rows.len() - 1
            } else {
                self.selected - 1
            };
            self.clamp_scroll();
        }
    }

    pub fn selected_row(&self) -> Option<SidebarRow> {
        self.rows.get(self.selected).copied()
    }

    pub fn delete_session(&mut self, idx: usize) {
        if idx < self.sessions.len() {
            let session = self.sessions.remove(idx);
            self.rebuild_rows();
            self.save_sessions();

            let home = crate::ipc::home_dir();
            if home.as_os_str().is_empty() {
                return;
            }
            let proj_dir = home.join(".commandcode/projects").join(&self.project);
            for ext in [".jsonl", ".meta.json", ".checkpoints.jsonl"] {
                let sess_file = proj_dir.join(format!("{}{}", session.id, ext));
                let _ = fs::remove_file(sess_file);
            }
        }
    }

    pub fn save_sessions(&self) {
        let mut sessions = self.sessions.clone();
        sort_sessions_by_activity(&mut sessions);
        let file = serde_json::json!({
            "project": self.project,
            "sessions": sessions,
        });
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = crate::ipc::atomic_write(&std::path::Path::new(&bridge_data_dir()).join("sessions.json"), &json);
        }
    }

    pub fn clean_all_sessions(&mut self, open_session_ids: &[String]) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|s| open_session_ids.contains(&s.id));
        let mut count = before.saturating_sub(self.sessions.len());
        self.rebuild_rows();
        self.save_sessions();

        let home = crate::ipc::home_dir();
        if !home.as_os_str().is_empty() {
            let proj_dir = home.join(".commandcode/projects").join(&self.project);
            if let Ok(entries) = fs::read_dir(&proj_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_open = path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|stem| open_session_ids.iter().any(|id| stem.starts_with(id)))
                        .unwrap_or(false);
                    if !is_open {
                        if fs::remove_file(path).is_ok() {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    pub fn clean_old_hours(&mut self, hours: i64, open_session_ids: &[String]) -> usize {
        let cutoff_ms = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0))
            - hours * 3600 * 1000;

        let before = self.sessions.len();
        self.sessions.retain(|s| s.last_at >= cutoff_ms || open_session_ids.contains(&s.id));
        let mut count = before.saturating_sub(self.sessions.len());
        self.rebuild_rows();
        self.save_sessions();

        let cutoff_time = std::time::SystemTime::now() - std::time::Duration::from_secs((hours * 3600) as u64);
        let home = crate::ipc::home_dir();
        if !home.as_os_str().is_empty() {
            let proj_dir = home.join(".commandcode/projects").join(&self.project);
            if let Ok(entries) = fs::read_dir(&proj_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let is_open = path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|stem| open_session_ids.iter().any(|id| stem.starts_with(id)))
                        .unwrap_or(false);
                    if !is_open {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(mtime) = meta.modified() {
                                if mtime < cutoff_time {
                                    if fs::remove_file(path).is_ok() {
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        count
    }

    pub fn clean_old_sessions(&mut self, days: i64, open_session_ids: &[String]) -> usize {
        self.clean_old_hours(days * 24, open_session_ids)
    }
}

