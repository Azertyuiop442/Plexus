
pub fn data_dir() -> String {
    crate::ipc::data_dir_str()
}

pub const SESSIONS_SHOWN: usize = 3;
#[allow(dead_code)]
pub const SIDEBAR_W: u16 = 25;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[allow(dead_code)]
pub struct SessionEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "lastAt", default)]
    pub last_at: i64,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub age: String,
    #[serde(rename = "ageShort", default)]
    pub age_short: String,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct SessionsFile {
    #[serde(default)]
    pub project: String,

    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub sessions: Vec<SessionEntry>,
}

impl SessionsFile {
    pub fn load() -> Self {
        Self::load_checked().unwrap_or_default()
    }

    pub fn load_checked() -> Option<Self> {
        let raw = std::fs::read_to_string(std::path::Path::new(&data_dir()).join("sessions.json"))
            .ok()?;
        serde_json::from_str::<SessionsFile>(&raw).ok()
    }
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
pub struct ModItem {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct ModMenuFile {
    #[serde(default)]
    items: Vec<ModItem>,
}

pub fn load_mods(config: Option<&serde_json::Value>) -> Vec<ModItem> {
    let mut mods: Vec<ModItem> = std::fs::read_to_string(std::path::Path::new(&data_dir()).join("mods.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<ModMenuFile>(&raw).ok())
        .map(|f| f.items)
        .filter(|items| !items.is_empty())
        .unwrap_or_default();
    if let Some(cfg) = config {
        for item in &mut mods {
            if let Some(enabled) = cfg.get(&item.id).and_then(|v| v.as_bool()) {
                item.enabled = enabled;
            }
        }
    }
    mods
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsSubMenu {
    Main,
    Preferences,
    ModConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SidebarPanelItem {
    pub icon: String,
    pub title: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarRow {
    UsageCarousel,
    UsagePrev,
    UsageNext,

    NewSession,
    Session(usize),
    MoreSessions,

    NavPreferences,
    NavModConfig,
    NavAIPrefs,
    Reload,
    BugReport,
    Update,
    Twitter,

    NavBack,

    PrefFullConfig,
    PrefAutoRetry,
    PrefSkills,
    PrefSkillInjection,
    PrefYolo,
    PrefShowUsage,
    PrefSounds,
    PrefWebhook,

    ModConfig(usize),
    LiveBlockOpen(usize),
    LiveBlockDismiss(usize),
    LiveBlockResume(usize),
    LiveBlockCopy(usize),

    RightSidebar(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct ClickZone {
    pub y: u16,
    pub x_start: u16,
    pub x_end: u16,
    pub row: SidebarRow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::HOME_LOCK;

    struct EnvGuard {
        old: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set_sidebar_dir(dir: &std::path::Path) -> Self {
            let old = std::env::var_os("CC_SIDEBAR_DIR");
            std::env::set_var("CC_SIDEBAR_DIR", dir);
            Self { old }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.old {
                Some(v) => std::env::set_var("CC_SIDEBAR_DIR", v),
                None => std::env::remove_var("CC_SIDEBAR_DIR"),
            }
        }
    }

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-sidebar-model-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_checked_is_none_when_sessions_file_is_absent() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp_dir("absent");
        let _env = EnvGuard::set_sidebar_dir(&dir);
        assert!(SessionsFile::load_checked().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_checked_is_none_on_partial_payload() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp_dir("partial");
        let _env = EnvGuard::set_sidebar_dir(&dir);
        std::fs::write(
            dir.join("sessions.json"),
            r#"{ "project": "p", "sessions": [ { "id": "a""#,
        )
        .unwrap();
        assert!(SessionsFile::load_checked().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_checked_parses_a_valid_payload() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp_dir("valid");
        let _env = EnvGuard::set_sidebar_dir(&dir);
        std::fs::write(
            dir.join("sessions.json"),
            r#"{ "project": "p", "cwd": "/tmp/p", "sessions": [ { "id": "a", "title": "A", "lastAt": 5 } ] }"#,
        )
        .unwrap();
        let file = SessionsFile::load_checked().expect("valid payload parses");
        assert_eq!(file.project, "p");
        assert_eq!(file.cwd, "/tmp/p");
        assert_eq!(file.sessions.len(), 1);
        assert_eq!(file.sessions[0].id, "a");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[derive(Debug, Default, Clone)]
pub struct SidebarView {
    pub row_y: Vec<(u16, SidebarRow)>,

    pub zones: Vec<ClickZone>,
}

impl SidebarView {

    pub fn row_at_y(&self, y: u16) -> Option<SidebarRow> {
        self.row_y
            .iter()
            .find(|(ry, _)| *ry == y)
            .map(|(_, row)| *row)
    }

    pub fn zone_at(&self, x: u16, y: u16) -> Option<SidebarRow> {
        self.zones
            .iter()
            .find(|z| z.y == y && x >= z.x_start && x < z.x_end)
            .map(|z| z.row)
    }
}

