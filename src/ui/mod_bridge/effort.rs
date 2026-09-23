use std::fs;
use std::path::PathBuf;

use crate::ui::mod_bridge::contract::ModsData;

fn display_effort(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("default")
        || trimmed.eq_ignore_ascii_case("n/a")
        || trimmed.eq_ignore_ascii_case("off")
    {
        return None;
    }
    Some(match trimmed.to_ascii_lowercase().as_str() {
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
        let config_paths = [
            home.join(".commandcode/config.json"),
            PathBuf::from(crate::ipc::ipc_path("config.json")),
        ];
        for path in &config_paths {
            if let Ok(raw) = fs::read_to_string(path) {
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
        }

        None
    }

    pub fn active_effort(&self, model_id: &str) -> Option<String> {
        let trimmed_id = model_id.trim();
        if trimmed_id.is_empty() || !crate::ui::modal::is_reasoning_model(trimmed_id) {
            return None;
        }

        let home = crate::ipc::home_dir();
        let config_paths = [
            home.join(".commandcode/config.json"),
            PathBuf::from(crate::ipc::ipc_path("config.json")),
        ];

        for path in &config_paths {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(effort_map) = json.get("reasoningEffort").and_then(|v| v.as_object()) {
                        if let Some(eff) = effort_map.get(trimmed_id).and_then(|v| v.as_str()) {
                            return display_effort(eff);
                        }
                    }
                }
            }
        }

        if let Some(eff) = self.mods.iter().rev().find_map(|m| {
            if m.data.active_model() == Some(trimmed_id) {
                m.data.effort()
            } else {
                None
            }
        }) {
            if let Some(d) = display_effort(&eff) {
                return Some(d);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::mod_bridge::contract::{ModData, ModLiveData};

    #[test]
    fn active_effort_resolves_per_model_and_does_not_cross_override() {
        let mut mods_data = ModsData::default();
        let mut mod_data = ModData::default();
        mod_data.effort = "xhigh".to_string();
        mod_data.model_id = "Qwen/Qwen3.8-Max".to_string();
        mods_data.mods.push(ModLiveData {
            id: "sample_mod".to_string(),
            data: mod_data,
        });

        assert_eq!(display_effort("max"), Some("Max".to_string()));
        assert_eq!(display_effort("xhigh"), Some("X-High".to_string()));
        assert_eq!(display_effort("none"), None);
    }
}
