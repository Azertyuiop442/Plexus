
use std::collections::BTreeMap;
use std::fs;

use crate::ui::mod_bridge::contract::{ModData, ModLiveData, ModsData};

pub fn mods_data_dir() -> std::path::PathBuf {
    crate::ipc::data_dir().join("mods-data")
}

pub fn is_enabled_by_config(id: &str, config: Option<&serde_json::Value>) -> bool {
    match config.and_then(|cfg| cfg.get(id)).and_then(|v| v.as_bool()) {
        Some(false) => false,
        _ => true,
    }
}

fn is_tty_scoped(stem: &str) -> bool {
    let Some((_, suffix)) = stem.rsplit_once('-') else {
        return false;
    };
    let suffix = suffix.trim_start_matches("tty").trim_start_matches("s");
    !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
}

pub(crate) fn canonical_mod_id(stem: &str) -> String {
    let mut id = stem.to_string();
    if let Some((base, suffix)) = stem.rsplit_once('-') {
        let s = suffix.trim_start_matches("tty").trim_start_matches("s");
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
            id = base.to_string();
        }
    }
    id
}

struct ModBridgeGuard {
    last_good: Option<ModData>,
    last_seq: u64,
    last_updated_at: u64,
    last_mtime: Option<std::time::SystemTime>,
    last_size: u64,
}

impl Default for ModBridgeGuard {
    fn default() -> Self {
        Self {
            last_good: None,
            last_seq: 0,
            last_updated_at: 0,
            last_mtime: None,
            last_size: 0,
        }
    }
}

thread_local! {

    static BRIDGE_GUARDS: std::cell::RefCell<BTreeMap<String, ModBridgeGuard>> =
        std::cell::RefCell::new(BTreeMap::new());
}

#[cfg(test)]
fn guarded_parse(guard_key: &str, raw: &str) -> Option<ModData> {
    let parsed: Option<ModData> = serde_json::from_str(raw).ok();
    let seq = parsed.as_ref().and_then(|d| d.seq).unwrap_or(0);
    let updated_at = parsed.as_ref().and_then(|d| d.updated_at).unwrap_or(0);

    BRIDGE_GUARDS.with(|g| {
        let mut map = g.borrow_mut();
        let guard = map.entry(guard_key.to_string()).or_default();
        match parsed {
            Some(data)
                if seq > guard.last_seq
                    || (updated_at > 0 && updated_at > guard.last_updated_at)
                    || guard.last_good.is_none() =>
            {
                guard.last_seq = seq;
                guard.last_updated_at = updated_at;
                guard.last_good = Some(data.clone());
                Some(data)
            }
            Some(_) => guard.last_good.clone(),
            None => guard.last_good.clone(),
        }
    })
}

fn guarded_parse_file(guard_key: &str, path: &std::path::Path) -> Option<ModData> {
    let meta = fs::metadata(path).ok();
    let mtime = meta.as_ref().and_then(|m| m.modified().ok());
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

    let cached = BRIDGE_GUARDS.with(|g| {
        let map = g.borrow();
        if let Some(guard) = map.get(guard_key) {
            if guard.last_good.is_some()
                && mtime.is_some()
                && guard.last_mtime == mtime
                && guard.last_size == size
            {
                return Some(guard.last_good.clone());
            }
        }
        None
    });

    if let Some(data) = cached {
        return data;
    }

    let Ok(raw) = fs::read_to_string(path) else {
        return BRIDGE_GUARDS.with(|g| g.borrow().get(guard_key).and_then(|guard| guard.last_good.clone()));
    };

    let parsed: Option<ModData> = serde_json::from_str(&raw).ok();
    let seq = parsed.as_ref().and_then(|d| d.seq).unwrap_or(0);
    let updated_at = parsed.as_ref().and_then(|d| d.updated_at).unwrap_or(0);

    BRIDGE_GUARDS.with(|g| {
        let mut map = g.borrow_mut();
        let guard = map.entry(guard_key.to_string()).or_default();
        guard.last_mtime = mtime;
        guard.last_size = size;
        match parsed {
            Some(data)
                if seq > guard.last_seq
                    || (updated_at > 0 && updated_at > guard.last_updated_at)
                    || guard.last_good.is_none() =>
            {
                guard.last_seq = seq;
                guard.last_updated_at = updated_at;
                guard.last_good = Some(data.clone());
                Some(data)
            }
            Some(_) => guard.last_good.clone(),
            None => guard.last_good.clone(),
        }
    })
}

impl ModsData {

    pub fn load_with_tty(tty: Option<&str>) -> Self {
        let dir = mods_data_dir();
        let mut mods = Vec::new();
        let Ok(entries) = fs::read_dir(dir) else {
            return Self::default();
        };
        let mut files: BTreeMap<String, (String, std::path::PathBuf)> = BTreeMap::new();
        let mut shared: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();
        let mut scoped_streams: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();

        let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();

        let mut tty_scoped_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };

            if stem.ends_with('-') {
                continue;
            }
            if let Some(tty) = tty {
                if let Some(base) = stem.strip_suffix(&format!("-{tty}")) {
                    scoped_streams.insert(format!("{base}@{tty}"), path.clone());
                    known.insert(base.to_string());
                    continue;
                }
            }
            if is_tty_scoped(stem) {

                if let Some(base) = stem.rsplit_once('-') {
                    tty_scoped_ids.insert(base.0.to_string());
                }
                continue;
            }
            known.insert(stem.to_string());
            shared.insert(stem.to_string(), path);
        }
        for (id, path) in shared {
            if tty.is_some() && tty_scoped_ids.contains(&id) {

                continue;
            }
            files.entry(id.clone()).or_insert((id, path));
        }
        for (key, path) in scoped_streams {
            let mod_id = key.split('@').next().unwrap_or(&key).to_string();
            files.insert(mod_id, (key, path));
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let cfg_path = std::path::PathBuf::from(home).join(".commandcode/config.json");
        let config = fs::read_to_string(&cfg_path)
            .ok()
            .or_else(|| fs::read_to_string(crate::ipc::ipc_path("config.json")).ok())
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
        for (id, (guard_key, path)) in files {
            if let Some(data) = guarded_parse_file(&guard_key, &path) {
                if is_enabled_by_config(&id, config.as_ref()) {
                    mods.push(ModLiveData { id, data });
                }
            }
        }
        ModsData { mods, known_mod_ids: known }
    }

    #[allow(dead_code)]
    pub fn load() -> Self {
        Self::load_with_tty(None)
    }

    pub fn segments(&self) -> Vec<crate::ui::mod_bridge::contract::ModSegment> {
        self.mods
            .iter()
            .filter(|m| m.data.mode.is_none())
            .flat_map(|m| m.data.segments.iter().cloned())
            .collect()
    }

    pub fn modes(&self) -> Vec<crate::ui::mod_bridge::contract::ModMode> {
        self.mods
            .iter()
            .filter_map(|m| m.data.mode.clone())
            .collect()
    }

    #[allow(dead_code)]
    pub fn sections(&self) -> Vec<crate::ui::mod_bridge::contract::ModSection> {
        self.mods
            .iter()
            .flat_map(|m| m.data.sections.iter().cloned())
            .collect()
    }

    pub fn live_blocks(&self) -> Vec<crate::ui::sidebar::LiveBlock> {
        self.mods
            .iter()
            .flat_map(|m| {
                m.data.live_blocks.iter().map(|b| {
                    crate::ui::sidebar::LiveBlock {
                        id: b.id.clone(),
                        label: b.label.clone(),
                        phase: b.phase.clone(),
                        agents: b
                            .agents
                            .iter()
                            .map(|a| crate::ui::sidebar::LiveAgent {
                                label: a.label.clone(),
                                status: a.status.clone(),
                            })
                            .collect(),
                        terminal: b.terminal,
                        done: b.done,
                        aborted: b.aborted,
                        stalled: b.stalled,
                        hint: b.hint.clone(),
                        open_path: b.open_path.clone(),
                        copy_text: b.copy_text.clone(),
                        resume_command: b.resume_command.clone(),
                        session_id: b.session_id.clone(),
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::mod_bridge::contract::{ModData, ModLiveData, ModModal};

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn canonical_mod_id_strips_tty_suffix() {
        assert_eq!(canonical_mod_id("examplemod"), "examplemod");
        assert_eq!(canonical_mod_id("examplemod-ttys003"), "examplemod");
        assert_eq!(canonical_mod_id("examplemod-s12"), "examplemod");
        assert_eq!(canonical_mod_id("mymod-ttys001"), "mymod");
        assert_eq!(canonical_mod_id("my-mod-extra"), "my-mod-extra");
    }

    #[test]
    fn deserializes_turns_with_cache_read_alias() {

        let raw = r#"{
            "model": "Laguna Free",
            "workspace": "/tmp/test-workspace",
            "turns": [
                { "model": "deepseek/deepseek-v4-flash", "input": 122819, "output": 13704, "cacheRead": 77184, "cost": 0.014 }
            ]
        }"#;
        let data: ModData = serde_json::from_str(raw).unwrap();
        assert_eq!(data.workspace, "/tmp/test-workspace");
        assert_eq!(data.turns.len(), 1);
        let t = &data.turns[0];
        assert_eq!(t.model, "deepseek/deepseek-v4-flash");
        assert_eq!(t.input, 122819);
        assert_eq!(t.output, 13704);
        assert_eq!(t.cache_read, 77184);
        assert_eq!(t.cost, 0.014);
    }

    #[test]
    fn deserializes_context_usage_gauge() {

        let raw = r#"{
            "model": "deepseek/deepseek-v4-flash",
            "contextUsage": { "used": 234540, "max": 1000000, "pct": 0.23454 }
        }"#;
        let data: ModData = serde_json::from_str(raw).unwrap();
        let cu = data.context_usage.expect("context_usage parsed");
        assert_eq!(cu.used, 234540);
        assert_eq!(cu.max, 1_000_000);
        assert!((cu.pct - 0.23454).abs() < 1e-9);
    }

    #[test]
    fn deserializes_updated_at_staleness_signal() {

        let raw = r#"{
            "updatedAt": 1786200000000,
            "model": "deepseek/deepseek-v4-flash"
        }"#;
        let data: ModData = serde_json::from_str(raw).unwrap();
        assert_eq!(data.updated_at, Some(1786200000000));

        let raw2 = r#"{"model": "x"}"#;
        let data2: ModData = serde_json::from_str(raw2).unwrap();
        assert_eq!(data2.updated_at, None);
    }

    #[test]
    fn deserializes_confirm_banner() {

        let raw = r#"{
            "id": "cost-clean-confirm",
            "title": "Confirm",
            "pending": true,
            "confirm": "This deletes EVERY saved session.",
            "items": [
                { "label": "Yes, delete everything", "value": "all-confirmed" },
                { "label": "Cancel", "value": "cancel" }
            ]
        }"#;
        let modal: ModModal = serde_json::from_str(raw).unwrap();
        assert_eq!(modal.id, "cost-clean-confirm");
        assert_eq!(
            modal.confirm.as_deref(),
            Some("This deletes EVERY saved session.")
        );
        assert_eq!(modal.items.len(), 2);
        assert_eq!(modal.items[0].value, "all-confirmed");
    }

    #[test]
    fn loads_and_aggregates_mod_files() {
        let dir = std::env::temp_dir().join("cc-mod-bridge-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("mods-data")).unwrap();
        fs::write(
            dir.join("mods-data/examplemod.json"),
            r#"{
                "segments": [
                    { "text": "$ 0.61", "color": "yellow", "bold": true },
                    { "text": "52.6M tokens", "color": "text" }
                ],
                "sections": [
                    { "heading": "Example Mod", "lines": ["cost    $0.6144", "turns   285"] }
                ]
            }"#,
        )
        .unwrap();
        fs::write(
            dir.join("mods-data/git-status.json"),
            r#"{
                "segments": [ { "text": "3 files", "color": "green" } ],
                "sections": [ { "heading": "Git", "lines": ["  src/mux.rs", "  src/state.rs"] } ]
            }"#,
        )
        .unwrap();

        let mut mods = Vec::new();
        for entry in fs::read_dir(dir.join("mods-data")).unwrap().flatten() {
            let path = entry.path();
            let id = path.file_stem().unwrap().to_string_lossy().to_string();
            let raw = fs::read_to_string(&path).unwrap();
            let data: ModData = serde_json::from_str(&raw).unwrap();
            mods.push(ModLiveData { id, data });
        }
        mods.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].id, "examplemod");
        assert_eq!(mods[0].data.segments.len(), 2);
        assert_eq!(mods[0].data.sections[0].lines[0], "cost    $0.6144");
        assert_eq!(mods[1].id, "git-status");
        assert_eq!(mods[1].data.segments[0].text, "3 files");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabled_mods_are_filtered_by_config() {
        let config = serde_json::json!({
            "examplemod": false,
            "git-status": true,
        });
        assert!(!is_enabled_by_config("examplemod", Some(&config)));
        assert!(is_enabled_by_config("git-status", Some(&config)));

        assert!(is_enabled_by_config("othermod", Some(&config)));
        assert!(is_enabled_by_config("othermod", None));
    }

    #[test]
    fn load_with_tty_prefers_scoped_bridge_file() {

        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_dir = std::env::var("CC_SIDEBAR_DIR").ok();
        let iso = std::env::temp_dir().join(format!(
            "cc-bridge-tty-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t").replace(':', "_")
        ));
        std::env::set_var("CC_SIDEBAR_DIR", &iso);
        let dir = mods_data_dir();
        let _ = fs::create_dir_all(&dir);
        let shared =
            r#"{ "segments": [ { "text": "$ 99.99", "color": "yellow", "bold": true } ] }"#;
        let scoped = r#"{ "segments": [ { "text": "$ 0.00", "color": "yellow", "bold": true } ] }"#;
        fs::write(dir.join("examplemod.json"), shared).unwrap();
        fs::write(dir.join("examplemod-ttys999.json"), scoped).unwrap();

        let global = ModsData::load();
        let seg = global.segments();
        assert_eq!(seg.len(), 1, "other-terminal scoped files must not leak in");
        assert_eq!(seg[0].text, "$ 99.99");

        let scoped_data = ModsData::load_with_tty(Some("ttys999"));
        let seg = scoped_data.segments();
        assert_eq!(seg[0].text, "$ 0.00");

        let _ = fs::remove_dir_all(&iso);
        match old_dir {
            Some(d) => std::env::set_var("CC_SIDEBAR_DIR", d),
            None => std::env::remove_var("CC_SIDEBAR_DIR"),
        }
    }

    #[test]
    fn fresh_tty_without_scoped_file_shows_spinner_not_shared_stats() {

        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_dir = std::env::var("CC_SIDEBAR_DIR").ok();
        let iso = std::env::temp_dir().join(format!(
            "cc-bridge-fresh-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t").replace(':', "_")
        ));
        std::env::set_var("CC_SIDEBAR_DIR", &iso);
        let dir = mods_data_dir();
        let _ = fs::create_dir_all(&dir);

        fs::write(
            dir.join("examplemod.json"),
            r#"{ "segments": [ { "text": "$ 99.99", "color": "yellow", "bold": true } ], "updatedAt": 9999999999999 }"#,
        )
        .unwrap();
        fs::write(
            dir.join("examplemod-ttys999.json"),
            r#"{ "segments": [ { "text": "$ 5.00", "color": "yellow", "bold": true } ], "updatedAt": 9999999999999 }"#,
        )
        .unwrap();

        let fresh = ModsData::load_with_tty(Some("ttysNEW"));
        assert!(
            fresh.mods.iter().all(|m| m.id != "examplemod"),
            "fresh tty must not show another terminal's shared stats"
        );

        let global = ModsData::load();
        let seg = global.segments();
        assert_eq!(seg.len(), 1);
        assert_eq!(seg[0].text, "$ 99.99");

        let _ = fs::remove_dir_all(&iso);
        match old_dir {
            Some(d) => std::env::set_var("CC_SIDEBAR_DIR", d),
            None => std::env::remove_var("CC_SIDEBAR_DIR"),
        }
    }

    #[test]
    fn absent_mod_is_not_known_anywhere() {

        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_dir = std::env::var("CC_SIDEBAR_DIR").ok();
        let iso = std::env::temp_dir().join(format!(
            "cc-bridge-absent-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("t").replace(':', "_")
        ));
        std::env::set_var("CC_SIDEBAR_DIR", &iso);
        let dir = mods_data_dir();
        let _ = fs::create_dir_all(&dir);

        fs::write(
            dir.join("modpresent.json"),
            r#"{ "segments": [], "updatedAt": 9999999999999 }"#,
        )
        .unwrap();

        let data = ModsData::load_with_tty(Some("ttysNEW"));
        assert!(
            !data.known_mod_ids.contains("modabsent"),
            "absent mod must not be known"
        );
        assert!(
            data.known_mod_ids.contains("modpresent"),
            "present mod is known"
        );

        let global = ModsData::load();
        assert!(global.known_mod_ids.contains("modpresent"));
        assert!(!global.known_mod_ids.contains("modabsent"));

        let _ = fs::remove_dir_all(&iso);
        match old_dir {
            Some(d) => std::env::set_var("CC_SIDEBAR_DIR", d),
            None => std::env::remove_var("CC_SIDEBAR_DIR"),
        }
    }

    #[test]
    fn guarded_parse_keeps_last_good_on_corrupt_bridge() {

        let good = r#"{
            "seq": 1,
            "segments": [ { "text": "$ 0.61", "color": "yellow", "bold": true } ],
            "turns": [ { "model": "x", "input": 1, "output": 2 } ]
        }"#;
        let data = guarded_parse("examplemod", good).expect("good bridge parses");
        assert_eq!(data.segments.len(), 1);

        let corrupt = r#"{ "seq": 2, "segments": [ { "text": "$ 0.6" "#;
        let data2 =
            guarded_parse("examplemod", corrupt).expect("corrupt falls back to last good");
        assert_eq!(data2.segments.len(), 1);
        assert_eq!(data2.segments[0].text, "$ 0.61");
    }

    #[test]
    fn guarded_parse_ignores_out_of_order_seq() {

        let fresh = r#"{ "seq": 5, "segments": [ { "text": "fresh", "color": "green" } ] }"#;
        let data = guarded_parse("examplemod", fresh).expect("seq 5 accepted");
        assert_eq!(data.segments[0].text, "fresh");

        let stale = r#"{ "seq": 3, "segments": [ { "text": "stale", "color": "green" } ] }"#;
        let data2 = guarded_parse("examplemod", stale).expect("stale falls back to last good");
        assert_eq!(data2.segments[0].text, "fresh");

        let same = r#"{ "seq": 5, "segments": [ { "text": "dup", "color": "green" } ] }"#;
        let data3 = guarded_parse("examplemod", same).expect("dup seq falls back");
        assert_eq!(data3.segments[0].text, "fresh");
    }

    #[test]
    fn guarded_parse_is_per_mod_isolated() {

        let _ = guarded_parse(
            "moda",
            r#"{ "seq": 1, "segments": [ { "text": "A" } ] }"#,
        );
        let other = guarded_parse("modb", r#"{ "seq": 1, "segments": [ { "text": "B" } ] }"#);
        assert_eq!(other.expect("modb parses").segments[0].text, "B");
    }

    #[test]
    fn contract_deserializes_seq_modid_unknownpricing() {
        let raw = r#"{
            "seq": 7,
            "modId": "examplemod",
            "unknownPricing": true,
            "contextUsage": { "used": 100, "max": 200000, "pct": 0.0005 }
        }"#;
        let data: ModData = serde_json::from_str(raw).unwrap();
        assert_eq!(data.seq, Some(7));
        assert_eq!(data.mod_id, "examplemod");
        assert!(data.unknown_pricing);
        let cu = data.context_usage.expect("context usage");
        assert_eq!(cu.used, 100);
        assert_eq!(cu.max, 200_000);
    }

    #[test]
    fn contract_deserializes_generic_panel() {

        let raw = r#"{
            "modId": "examplemod",
            "seq": 3,
            "panels": [{
                "id": "git-main",
                "modId": "examplemod",
                "title": "git · status",
                "state": "ready",
                "tabs": [{ "id": "status", "label": "Status" }],
                "activeTab": "status",
                "columns": [{ "key": "x", "label": "", "width": 2 }],
                "rows": [
                    { "id": "src/mux.rs", "cells": ["M", "src/mux.rs"], "color": "yellow",
                      "value": "{\"action\":\"diff_file\",\"args\":[\"src/mux.rs\"]}" }
                ],
                "detail": { "title": "main", "lines": [{ "text": " M src/mux.rs", "color": "yellow" }] },
                "footer": [{ "key": "a", "label": "Stage" }],
                "summary": { "label": "1 file", "color": "yellow" },
                "futureField": true
            }]
        }"#;
        let data: ModData = serde_json::from_str(raw).unwrap();
        assert_eq!(data.mod_id, "examplemod");
        let panels = &data.panels;
        assert_eq!(panels.len(), 1);
        let p = &panels[0];
        assert_eq!(p.id, "git-main");
        assert_eq!(p.mod_id, "examplemod");
        assert_eq!(p.state, "ready");
        assert_eq!(p.tabs.len(), 1);
        assert_eq!(p.active_tab, "status");
        assert_eq!(p.rows.len(), 1);
        assert_eq!(p.rows[0].id, "src/mux.rs");
        assert_eq!(p.rows[0].color, "yellow");
        assert_eq!(p.detail.as_ref().unwrap().lines.len(), 1);
        assert_eq!(p.footer.len(), 1);
        assert_eq!(p.summary.as_ref().unwrap().label, "1 file");
    }
}

