
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
        std::fs::read_to_string(std::path::Path::new(&data_dir()).join("sessions.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<SessionsFile>(&s).ok())
            .unwrap_or_default()
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

pub const RIGHT_SIDEBARS: &[(&str, &str)] = &[
    ("\u{e702}", "Git"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarRow {
    NewSession,
    Session(usize),
    MoreSessions,

    NavPreferences,
    NavModConfig,
    NavAIPrefs,
    Reload,

    NavBack,

    PrefFullConfig,
    PrefYolo,

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

