
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModSegment {
    #[serde(default)]
    pub text: String,

    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub bold: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModSection {
    #[serde(default)]
    pub heading: String,
    #[serde(default)]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModMode {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModData {
    #[serde(default)]
    pub segments: Vec<ModSegment>,
    #[serde(default)]
    pub sections: Vec<ModSection>,

    #[serde(default)]
    pub mode: Option<ModMode>,

    #[serde(default)]
    pub model: String,

    #[serde(default, alias = "modelId")]
    pub model_id: String,

    #[serde(default)]
    pub effort: String,

    #[serde(default)]
    pub modals: Vec<ModModal>,

    #[serde(default)]
    pub turns: Vec<ModTurn>,

    #[serde(default)]
    pub workspace: String,

    #[serde(default, alias = "contextUsage")]
    pub context_usage: Option<ModContextUsage>,

    #[serde(default, alias = "updatedAt")]
    pub updated_at: Option<u64>,

    #[serde(default)]
    pub seq: Option<u64>,

    #[serde(default, alias = "modId")]
    pub mod_id: String,

    #[serde(default, alias = "unknownPricing")]
    pub unknown_pricing: bool,

    #[serde(default, alias = "liveBlocks")]
    pub live_blocks: Vec<ModLiveBlock>,

    #[serde(default)]
    pub panels: Vec<ModPanel>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModLiveBlock {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub agents: Vec<ModLiveAgent>,
    #[serde(default)]
    pub terminal: usize,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub aborted: bool,
    #[serde(default)]
    pub stalled: bool,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default, alias = "openPath")]
    pub open_path: Option<String>,
    #[serde(default, alias = "copyText")]
    pub copy_text: Option<String>,
    #[serde(default, alias = "resumeCommand")]
    pub resume_command: Option<String>,

    #[serde(default, alias = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModLiveAgent {
    #[serde(default)]
    pub label: String,

    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModContextUsage {
    #[serde(default)]
    pub used: u64,
    #[serde(default)]
    pub max: u64,

    #[serde(default)]
    pub pct: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModTurn {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default, alias = "cacheRead")]
    pub cache_read: u64,
    #[serde(default)]
    pub cost: f64,

    #[serde(default, alias = "cacheHitPct")]
    pub cache_hit_pct: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModModal {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub pending: bool,
    #[serde(default)]
    pub items: Vec<ModModalItem>,

    #[serde(default)]
    pub actions: Vec<ModModalAction>,

    #[serde(default)]
    pub progress: Option<ModModalProgress>,

    #[serde(default)]
    pub readonly: bool,

    #[serde(default)]
    pub confirm: Option<String>,

    #[serde(default, alias = "pickupCommand")]
    pub pickup_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModModalProgress {

    #[serde(default)]
    pub label: String,

    #[serde(default)]
    pub current: usize,

    #[serde(default)]
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModModalItem {
    #[serde(default)]
    pub label: String,

    #[serde(default)]
    pub value: String,

    #[serde(default)]
    pub detail: String,

    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModModalAction {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct ModLiveData {
    pub id: String,
    pub data: ModData,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelTab {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelColumn {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub width: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelSpan {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub align: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelRow {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub cells: Vec<String>,
    #[serde(default, alias = "cellColors")]
    pub cell_colors: Vec<String>,
    #[serde(default)]
    pub spans: Vec<ModPanelSpan>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelDetailLine {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelDetail {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub lines: Vec<ModPanelDetailLine>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelFooterHint {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default, alias = "isAction")]
    pub is_action: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanelSummary {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModPanel {
    #[serde(default)]
    pub id: String,
    #[serde(default, alias = "modId")]
    pub mod_id: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub tabs: Vec<ModPanelTab>,
    #[serde(default, alias = "activeTab")]
    pub active_tab: String,
    #[serde(default)]
    pub columns: Vec<ModPanelColumn>,
    #[serde(default)]
    pub rows: Vec<ModPanelRow>,
    #[serde(default, alias = "tabRows")]
    pub tab_rows: std::collections::HashMap<String, Vec<ModPanelRow>>,
    #[serde(default)]
    pub detail: Option<ModPanelDetail>,
    #[serde(default)]
    pub footer: Vec<ModPanelFooterHint>,
    #[serde(default, alias = "defaultValue")]
    pub default_value: String,
    #[serde(default)]
    pub summary: Option<ModPanelSummary>,
    #[serde(default)]
    pub wants_context: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ModsData {
    pub mods: Vec<ModLiveData>,

    pub known_mod_ids: std::collections::HashSet<String>,
}

impl ModData {

    pub fn active_model(&self) -> Option<&str> {
        let m = self.model.trim();
        if m.is_empty() || m.eq_ignore_ascii_case("unknown") {
            None
        } else {
            Some(m)
        }
    }

    pub fn effort(&self) -> Option<String> {
        let e = self.effort.trim();
        if e.is_empty() {
            None
        } else {
            Some(e.to_string())
        }
    }

    pub fn context_usage(&self) -> Option<&ModContextUsage> {
        self.context_usage.as_ref()
    }
}

