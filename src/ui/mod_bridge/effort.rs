
use std::fs;

use crate::ui::mod_bridge::contract::ModsData;

fn display_effort(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return None;
    }
    Some(match trimmed {
        "low" => "Low".to_string(),
        "medium" => "Medium".to_string(),
        "high" => "High".to_string(),
        "xhigh" => "X-High".to_string(),
        "max" => "Max".to_string(),
        other => other.to_string(),
    })
}

impl ModsData {

    pub fn active_model(&self) -> Option<String> {
        if let Some(m) = self.mods.iter().rev().find_map(|m| m.data.active_model()) {
            if !m.trim().is_empty() && !m.trim().eq_ignore_ascii_case("unknown") {
                return Some(m.trim().to_string());
            }
        }

        let home = crate::ipc::home_dir();
        if home.as_os_str().is_empty() {
            return None;
        }
        let config_path = home.join(".commandcode/config.json");
        if let Ok(raw) = fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(m) = json
                    .get("model")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                {
                    return Some(m.trim().to_string());
                }
            }
        }

        None
    }

    pub fn active_effort(&self, model_id: &str) -> Option<String> {
        if model_id.trim().is_empty() {
            return None;
        }
        let home = crate::ipc::home_dir();
        if home.as_os_str().is_empty() {
            return None;
        }
        let config_path = home.join(".commandcode/config.json");
        if let Ok(raw) = fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(effort_map) = json.get("reasoningEffort").and_then(|v| v.as_object()) {
                    if let Some(eff) = effort_map.get(model_id).and_then(|v| v.as_str()) {
                        if let Some(d) = display_effort(eff) {
                            return Some(d);
                        }
                    }
                }
            }
        }

        if let Some(eff) = self.mods.iter().rev().find_map(|m| m.data.effort()) {
            return display_effort(&eff);
        }

        None
    }
}

