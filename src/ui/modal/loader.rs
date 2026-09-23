
use std::path::Path;

use super::model::{Modal, ModalRow};

pub fn load_modal(data_dir: &Path, mod_id: &str) -> Option<Modal> {
    let path = data_dir.join("modals").join(format!("{mod_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[modal] {mod_id}: malformed JSON ({}): {e}", path.display());
            return None;
        }
    };

    let title = value
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or(mod_id)
        .to_string();

    let persist = value
        .get("persist")
        .and_then(|p| p.as_str())
        .map(std::path::PathBuf::from);

    let persist_config = value
        .get("persistConfig")
        .and_then(|p| p.as_str())
        .map(std::path::PathBuf::from);

    let home_cfg = crate::ipc::home_dir().join(".commandcode/config.json");
    let config_json: serde_json::Value = match &persist_config {
        Some(p) => std::fs::read_to_string(p)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(serde_json::json!({})),
        None => std::fs::read_to_string(&home_cfg)
            .ok()
            .or_else(|| std::fs::read_to_string(crate::ipc::ipc_path("config.json")).ok())
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(serde_json::json!({})),
    };

    let mut modal = Modal::new(mod_id.to_string(), title);
    modal.persist = persist;
    modal.persist_config = persist_config;

    if let Some(cmds) = value.get("commands").and_then(|c| c.as_array()) {
        for c in cmds {
            let name = c
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let desc = c
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                modal.commands.push((name, desc));
            }
        }
    }
    if let Some(steps) = value.get("steps").and_then(|s| s.as_array()) {
        for step in steps {
            let step_title = step.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
            let mut step_rows = Vec::new();
            if let Some(rows) = step.get("rows").and_then(|r| r.as_array()) {
                for row in rows {
                    step_rows.push(parse_row(row, &config_json, mod_id));
                }
            }
            modal.add_step(step_title, step_rows);
        }
    } else if let Some(rows) = value.get("rows").and_then(|r| r.as_array()) {
        for row in rows {
            modal.rows.push(parse_row(row, &config_json, mod_id));
        }
    }
    modal.select_first_selectable();

    Some(modal)
}

fn parse_row(row: &serde_json::Value, config_json: &serde_json::Value, mod_id: &str) -> ModalRow {
    let kind = row.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let key = row
        .get("key")
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string();
    let label = row
        .get("label")
        .and_then(|l| l.as_str())
        .unwrap_or("")
        .to_string();
    match kind {
        "toggle" => {
            let mut enabled = row.get("value").and_then(|v| v.as_bool()).unwrap_or(false);
            if let Some(cfg_val) = config_json.get(&key).and_then(|v| v.as_bool()) {
                enabled = cfg_val;
            } else if key == "enabled" {
                if let Some(mod_cfg) = config_json.get(mod_id).and_then(|v| v.as_bool()) {
                    enabled = mod_cfg;
                }
            }
            ModalRow::Toggle {
                key,
                label,
                enabled,
            }
        }
        "choice" => {
            let mut current_val = row
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(cfg_val) = config_json.get(&key).and_then(|v| v.as_str()) {
                current_val = cfg_val.to_string();
            }
            let searchable = row
                .get("searchable")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            let color = row
                .get("color")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let mut options: Vec<(String, String, String)> = Vec::new();
            let mut current = 0usize;
            if let Some(opts) = row.get("options").and_then(|o| o.as_array()) {
                for (i, opt) in opts.iter().enumerate() {
                    if let (Some(l), Some(v)) = (
                        opt.get(0).and_then(|x| x.as_str()),
                        opt.get(1).and_then(|x| x.as_str()),
                    ) {
                        let cat = opt
                            .get(2)
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        if v == current_val {
                            current = i;
                        }
                        options.push((l.to_string(), v.to_string(), cat));
                    }
                }
            }
            ModalRow::Choice {
                key,
                label,
                options,
                current,
                searchable,
                color,
            }
        }
        "text_input" | "input" => {
            let mut value = row
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(cfg_val) = config_json.get(&key).and_then(|v| v.as_str()) {
                value = cfg_val.to_string();
            }
            ModalRow::TextInput { key, label, value }
        }
        "stepper" | "number" => {
            let mut value = row.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            if let Some(cfg_val) = config_json.get(&key).and_then(|v| v.as_i64()) {
                value = cfg_val;
            }
            let min = row.get("min").and_then(|v| v.as_i64()).unwrap_or(0);
            let max = row.get("max").and_then(|v| v.as_i64()).unwrap_or(1000);
            let step = row.get("step").and_then(|v| v.as_i64()).unwrap_or(1).max(1);
            let unit = row
                .get("unit")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ModalRow::Stepper {
                key,
                label,
                value,
                min,
                max,
                step,
                unit,
            }
        }
        "table" => {
            let headers: Vec<String> = row
                .get("headers")
                .and_then(|h| h.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let rows: Vec<Vec<String>> = row
                .get("rows")
                .and_then(|r| r.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|r| r.as_array())
                        .map(|cells| {
                            cells
                                .iter()
                                .filter_map(|c| c.as_str().map(str::to_string))
                                .collect()
                        })
                        .collect()
                })
                .unwrap_or_default();
            let color = row
                .get("color")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            ModalRow::Table {
                headers,
                rows,
                color,
            }
        }
        "section" => {
            let color = row
                .get("color")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            ModalRow::Section { title: label, color }
        }
        _ => {
            let text = row
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            ModalRow::Info(text)
        }
    }
}

