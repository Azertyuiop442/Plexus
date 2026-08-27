
use std::path::Path;

use super::model::{Modal, ModalRow};

impl Modal {

    pub fn save(&self) {
        if let Some(path) = &self.persist {
            if let Ok(json) = serde_json::to_string_pretty(&self.to_json()) {
                let _ = crate::ipc::atomic_write(path, &json);
            }
        }
        if let Some(cfg_path) = &self.persist_config {
            self.save_config(cfg_path);
        }
    }

    fn save_config(&self, cfg_path: &Path) {
        let mut patch = serde_json::Map::new();
        let mut nested: std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>> =
            std::collections::BTreeMap::new();
        let all_rows: Vec<&ModalRow> = if !self.steps.is_empty() {
            self.steps.iter().flat_map(|s| s.rows.iter()).collect()
        } else {
            self.rows.iter().collect()
        };

        for row in all_rows {
            match row {
                ModalRow::Toggle { key, enabled, .. } => {
                    if key == "enabled" {
                        patch.insert(self.id.clone(), serde_json::json!(enabled));
                    } else if let Some((parent, child)) = key.split_once('.') {
                        nested
                            .entry(parent.to_string())
                            .or_default()
                            .insert(child.to_string(), serde_json::json!(enabled));
                    } else {
                        patch.insert(key.clone(), serde_json::json!(enabled));
                    }
                }
                ModalRow::Choice {
                    key,
                    options,
                    current,
                    ..
                } => {
                    if let Some((_, val, _)) = options.get(*current) {
                        if let Some((parent, child)) = key.split_once('.') {
                            nested
                                .entry(parent.to_string())
                                .or_default()
                                .insert(child.to_string(), serde_json::json!(val));
                        } else {
                            patch.insert(key.clone(), serde_json::json!(val));
                        }
                    }
                }
                ModalRow::TextInput { key, value, .. } => {
                    if let Some((parent, child)) = key.split_once('.') {
                        nested
                            .entry(parent.to_string())
                            .or_default()
                            .insert(child.to_string(), serde_json::json!(value));
                    } else {
                        patch.insert(key.clone(), serde_json::json!(value));
                    }
                }
                ModalRow::Stepper { key, value, .. } => {
                    if let Some((parent, child)) = key.split_once('.') {
                        nested
                            .entry(parent.to_string())
                            .or_default()
                            .insert(child.to_string(), serde_json::json!(value));
                    } else {
                        patch.insert(key.clone(), serde_json::json!(value));
                    }
                }
                _ => {}
            }
        }
        if patch.is_empty() && nested.is_empty() {
            return;
        }
        let json = {

            let mut map: serde_json::Map<String, serde_json::Value> =
                std::fs::read_to_string(cfg_path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .and_then(|v| v.as_object().cloned())
                    .unwrap_or_default();
            for (k, v) in &patch {
                map.insert(k.clone(), v.clone());
            }
            for (parent, children) in &nested {
                let entry = map
                    .entry(parent.clone())
                    .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                if let Some(obj) = entry.as_object_mut() {
                    for (ck, cv) in children {
                        obj.insert(ck.clone(), cv.clone());
                    }
                }
            }
            serde_json::to_string_pretty(&serde_json::Value::Object(map))
        };
        if let Ok(json) = json {
            let _ = crate::ipc::atomic_write(cfg_path, &json);

            if self.mirror {
                let _ = std::fs::create_dir_all(crate::ipc::data_dir());
                let _ = crate::ipc::atomic_write(
                    Path::new(&crate::ipc::ipc_path("config.json")),
                    &json,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Modal, ModalRow};
    use super::super::model::ModalStep;
    use std::path::PathBuf;

    fn fresh_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cc-dashboard-persist-{name}"));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.json");
        let _ = std::fs::remove_file(&path);
        std::fs::write(
            &path,
            r#"{"theme":"dark","provider":"command-code","featureModels":{"vision":"keep-me"}}"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn nested_text_input_merges_into_existing_parent_object() {
        let path = fresh_path("nested-text");
        let mut m = Modal::new("cc_global_config", "test");
        m.mirror = false;
        m.persist_config = Some(path.clone());
        m.steps.push(ModalStep {
            title: "Feature Models".into(),
            rows: vec![ModalRow::TextInput {
                key: "featureModels.branchSummarization".into(),
                label: "Branch Summarization".into(),
                value: "minimax/minimax-m3-free".into(),
            }],
        });

        m.save();

        let raw = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(v["theme"], "dark");
        assert_eq!(v["provider"], "command-code");

        assert_eq!(v["featureModels"]["vision"], "keep-me");

        assert_eq!(
            v["featureModels"]["branchSummarization"],
            "minimax/minimax-m3-free"
        );

        assert!(v["featureModels"].is_object());
    }

    #[test]
    fn nested_creates_parent_object_when_missing() {
        let path = fresh_path("nested-missing");

        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();

        let mut m = Modal::new("cc_global_config", "test");
        m.mirror = false;
        m.persist_config = Some(path.clone());
        m.steps.push(ModalStep {
            title: "Compaction".into(),
            rows: vec![ModalRow::TextInput {
                key: "featureModels.compaction".into(),
                label: "Compaction".into(),
                value: "openai/gpt-5".into(),
            }],
        });

        m.save();

        let raw = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["featureModels"]["compaction"], "openai/gpt-5");
    }

    #[test]
    fn top_level_key_still_works_unchanged() {
        let path = fresh_path("top-level");
        let mut m = Modal::new("cc_global_config", "test");
        m.mirror = false;
        m.persist_config = Some(path.clone());
        m.rows.push(ModalRow::Toggle {
            key: "tasteLearning".into(),
            label: "Taste Learning".into(),
            enabled: false,
        });

        m.save();

        let raw = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["tasteLearning"], false);

        assert_eq!(v["theme"], "dark");
    }
}

