
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const PREFS_PATH: &str = ".commandcode/cc-dashboard-prefs.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRetryPrefs {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub retry_rate_limit: bool,
    #[serde(default = "default_true")]
    pub retry_server_error: bool,
    #[serde(default = "default_true")]
    pub retry_network_timeout: bool,
    #[serde(default)]
    pub retry_stream_drop: bool,
    #[serde(default)]
    pub retry_tool_failure: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: i64,
    #[serde(default = "default_backoff_mode")]
    pub backoff_mode: String,
    #[serde(default = "default_base_delay")]
    pub base_delay_secs: i64,
    #[serde(default = "default_max_delay")]
    pub max_delay_secs: i64,
    #[serde(default = "default_true")]
    pub random_jitter: bool,
    #[serde(default = "default_retry_prompt")]
    pub prompt: String,
    #[serde(default = "default_true")]
    pub show_countdown: bool,
    #[serde(default = "default_true")]
    pub notify_on_failure: bool,
}

fn default_max_retries() -> i64 {
    3
}
fn default_backoff_mode() -> String {
    "exponential".into()
}
fn default_base_delay() -> i64 {
    2
}
fn default_max_delay() -> i64 {
    30
}
fn default_retry_prompt() -> String {
    "continue".into()
}

impl Default for AutoRetryPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            retry_rate_limit: true,
            retry_server_error: true,
            retry_network_timeout: true,
            retry_stream_drop: false,
            retry_tool_failure: false,
            max_retries: 3,
            backoff_mode: "exponential".into(),
            base_delay_secs: 2,
            max_delay_secs: 30,
            random_jitter: true,
            prompt: "continue".into(),
            show_countdown: true,
            notify_on_failure: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillsPrefs {
    #[serde(default = "default_true")]
    pub auto_update_check: bool,
    #[serde(default = "default_skills_cooldown")]
    pub check_cooldown_secs: u64,
    #[serde(default)]
    pub injection_enabled: bool,
    #[serde(default = "default_skills_injection_prompt")]
    pub injection_prompt: String,
    #[serde(default)]
    pub disabled_skills: Vec<String>,
    #[serde(default)]
    pub active_prompt: Option<String>,
}

impl SkillsPrefs {
    pub fn is_skill_enabled(&self, name: &str) -> bool {
        !self.disabled_skills.iter().any(|s| s == name)
    }

    pub fn refresh_active_prompt(&mut self) {
        self.active_prompt = Some(crate::skills_meta::build_active_skills_prompt(&self.disabled_skills));
    }

    pub fn toggle_skill(&mut self, name: &str) {
        if self.is_skill_enabled(name) {
            self.disabled_skills.push(name.to_string());
        } else {
            self.disabled_skills.retain(|s| s != name);
        }
        self.refresh_active_prompt();
    }

    pub fn enable_all(&mut self) {
        self.disabled_skills.clear();
        self.refresh_active_prompt();
    }

    pub fn disable_all(&mut self, all_skills: &[String]) {
        self.disabled_skills = all_skills.to_vec();
        self.refresh_active_prompt();
    }

    pub fn toggle_vendor(&mut self, all_vendor_skills: &[String]) {
        let all_enabled = all_vendor_skills.iter().all(|s| self.is_skill_enabled(s));
        if all_enabled {
            for s in all_vendor_skills {
                if !self.disabled_skills.contains(s) {
                    self.disabled_skills.push(s.clone());
                }
            }
        } else {
            for s in all_vendor_skills {
                self.disabled_skills.retain(|item| item != s);
            }
        }
        self.refresh_active_prompt();
    }
}

fn default_skills_injection_prompt() -> String {
    "Apply the appropriate skill if applicable to the task.".into()
}

fn default_skills_cooldown() -> u64 {
    300
}

impl Default for SkillsPrefs {
    fn default() -> Self {
        Self {
            auto_update_check: true,
            check_cooldown_secs: 300,
            injection_enabled: false,
            injection_prompt: default_skills_injection_prompt(),
            disabled_skills: Vec::new(),
            active_prompt: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundPrefs {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sound_completed")]
    pub sound_completed: String,
    #[serde(default = "default_sound_blocked")]
    pub sound_blocked: String,
}

fn default_sound_completed() -> String {
    "Glass".into()
}

fn default_sound_blocked() -> String {
    "Sosumi".into()
}

impl Default for SoundPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            sound_completed: default_sound_completed(),
            sound_blocked: default_sound_blocked(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default = "default_true")]
    pub show_banner: bool,
    #[serde(default)]
    pub yolo_mode: bool,
    #[serde(default)]
    pub skill_injection: bool,
    #[serde(default = "default_true")]
    pub taste_learning: bool,
    #[serde(default = "default_true")]
    pub ide_context: bool,
    #[serde(default = "default_true")]
    pub show_cost_bar: bool,
    #[serde(default = "default_true")]
    pub show_context_btn: bool,
    #[serde(default = "default_true")]
    pub show_usage: bool,
    #[serde(default = "default_sidebar_w")]
    pub sidebar_w: u16,
    #[serde(default = "default_true")]
    pub sidebar_open: bool,
    #[serde(default)]
    pub auto_retry: AutoRetryPrefs,
    #[serde(default)]
    pub skills: SkillsPrefs,
    #[serde(default)]
    pub sounds: SoundPrefs,
    #[serde(default)]
    pub webhook: WebhookPrefs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookPrefs {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_webhook_url")]
    pub url: String,
    #[serde(default = "default_webhook_format")]
    pub format: String,
    #[serde(default = "default_webhook_speed")]
    pub speed: u32,
    #[serde(default = "default_true")]
    pub include_usage: bool,
    #[serde(default = "default_webhook_mode")]
    pub mode: String,
}

fn default_webhook_url() -> String {
    "http://matrix.local/api".to_string()
}

fn default_webhook_format() -> String {
    "form".to_string()
}

fn default_webhook_speed() -> u32 {
    25
}

fn default_webhook_mode() -> String {
    "static".to_string()
}

impl Default for WebhookPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_webhook_url(),
            format: default_webhook_format(),
            speed: default_webhook_speed(),
            include_usage: true,
            mode: default_webhook_mode(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_sidebar_w() -> u16 {
    25
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            show_banner: true,
            yolo_mode: false,
            skill_injection: false,
            taste_learning: true,
            ide_context: true,
            show_cost_bar: true,
            show_context_btn: true,
            show_usage: true,
            sidebar_w: 25,
            sidebar_open: true,
            auto_retry: AutoRetryPrefs::default(),
            skills: SkillsPrefs::default(),
            sounds: SoundPrefs::default(),
            webhook: WebhookPrefs::default(),
        }
    }
}

impl Prefs {

    fn store_paths() -> Option<(PathBuf, PathBuf)> {
        let home = crate::ipc::home_dir();
        if home.as_os_str().is_empty() {
            return None;
        }
        Some((
            home.join(PREFS_PATH),
            home.join(".commandcode/config.json"),
        ))
    }

    pub fn local_path() -> Option<PathBuf> {
        let home = crate::ipc::home_dir();
        if home.as_os_str().is_empty() {
            return None;
        }
        Some(home.join(PREFS_PATH))
    }

    pub fn config_path() -> Option<PathBuf> {
        let home = crate::ipc::home_dir();
        if home.as_os_str().is_empty() {
            return None;
        }
        Some(home.join(".commandcode/config.json"))
    }

    pub fn load() -> Self {
        let (local_path, shared_path) = match Self::store_paths() {
            Some(p) => p,
            None => return Self::default(),
        };
        let mut prefs: Prefs = fs::read_to_string(&local_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();

        if let Some(raw) = fs::read_to_string(&local_path).ok() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(b) = v.get("skill_injection").and_then(|x| x.as_bool()) {
                    prefs.skill_injection = b;
                }
            }
        }

        let shared: Option<serde_json::Value> = fs::read_to_string(&shared_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok());
        if let Some(v) = shared.as_ref().and_then(|c| c.get("yolo")).and_then(|v| v.as_bool()) {
            prefs.yolo_mode = v;
        }
        if let Some(v) = shared
            .as_ref()
            .and_then(|c| c.get("tasteLearning"))
            .and_then(|v| v.as_bool())
        {
            prefs.taste_learning = v;
        }
        if let Some(v) = shared
            .as_ref()
            .and_then(|c| c.get("ideContextEnabled"))
            .and_then(|v| v.as_bool())
        {
            prefs.ide_context = v;
        }
        prefs
    }

    pub fn set_yolo_mode(on: bool) {
        let mut prefs = Self::load();
        if prefs.yolo_mode == on {
            return;
        }
        prefs.yolo_mode = on;
        prefs.save();
    }

    pub fn save(&self) {
        let Some((local_path, shared_path)) = Self::store_paths() else {
            return;
        };

        let local = serde_json::json!({
            "show_banner": self.show_banner,
            "show_cost_bar": self.show_cost_bar,
            "show_context_btn": self.show_context_btn,
            "show_usage": self.show_usage,
            "sidebar_w": self.sidebar_w,
            "sidebar_open": self.sidebar_open,
            "yolo_mode": self.yolo_mode,
            "skill_injection": self.skill_injection,
            "taste_learning": self.taste_learning,
            "ide_context": self.ide_context,
            "auto_retry": self.auto_retry,
            "skills": self.skills,
            "sounds": self.sounds,
            "webhook": self.webhook,
        });
        if let Ok(json) = serde_json::to_string_pretty(&local) {
            let _ = crate::ipc::atomic_write(&local_path, &json);
        }

        let mut patch = serde_json::Map::new();
        patch.insert("yolo".into(), serde_json::json!(self.yolo_mode));
        patch.insert(
            "tasteLearning".into(),
            serde_json::json!(self.taste_learning),
        );
        patch.insert(
            "ideContextEnabled".into(),
            serde_json::json!(self.ide_context),
        );
        let _ = crate::ipc::merge_write_json(&shared_path, &patch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    struct TestHome {
        orig: Option<std::ffi::OsString>,
        path: PathBuf,
    }

    impl std::ops::Deref for TestHome {
        type Target = Path;
        fn deref(&self) -> &Self::Target {
            &self.path
        }
    }

    impl AsRef<Path> for TestHome {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            if let Some(ref o) = self.orig {
                std::env::set_var("HOME", o);
            } else {
                std::env::remove_var("HOME");
            }
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn test_home(key: &str) -> TestHome {
        let orig = std::env::var_os("HOME");
        let path = std::env::temp_dir().join(format!("cc-prefs-{key}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        std::env::set_var("HOME", &path);
        TestHome { orig, path }
    }

    #[test]
    fn prefs_roundtrip() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _home = test_home("roundtrip");
        let p = Prefs {
            show_banner: false,
            yolo_mode: true,
            skill_injection: true,
            taste_learning: false,
            ide_context: true,
            show_cost_bar: false,
            show_context_btn: true,
            show_usage: false,
            sidebar_w: 31,
            sidebar_open: false,
            auto_retry: AutoRetryPrefs {
                enabled: false,
                retry_rate_limit: true,
                retry_server_error: false,
                retry_network_timeout: true,
                retry_stream_drop: true,
                retry_tool_failure: false,
                max_retries: 5,
                backoff_mode: "linear".into(),
                base_delay_secs: 4,
                max_delay_secs: 45,
                random_jitter: false,
                prompt: "retry step".into(),
                show_countdown: false,
                notify_on_failure: true,
            },
            skills: SkillsPrefs {
                auto_update_check: false,
                check_cooldown_secs: 60,
                injection_enabled: true,
                injection_prompt: "Apply the appropriate skill if applicable to the task.".into(),
                disabled_skills: Vec::new(),
                active_prompt: None,
            },
            sounds: SoundPrefs {
                enabled: false,
                sound_completed: "Ping".into(),
                sound_blocked: "Basso".into(),
            },
            webhook: WebhookPrefs {
                enabled: true,
                url: "http://test.local/api".into(),
                format: "form".into(),
                speed: 30,
                include_usage: false,
                mode: "static".into(),
            },
        };
        p.save();

        let loaded = Prefs::load();

        assert_eq!(loaded.show_banner, p.show_banner);
        assert_eq!(loaded.show_cost_bar, p.show_cost_bar);
        assert_eq!(loaded.show_usage, p.show_usage);
        assert_eq!(loaded.sidebar_w, p.sidebar_w);
        assert_eq!(loaded.sidebar_open, p.sidebar_open);
        assert_eq!(loaded.yolo_mode, p.yolo_mode);
        assert_eq!(loaded.taste_learning, p.taste_learning);
        assert_eq!(loaded.ide_context, p.ide_context);
        assert_eq!(loaded.auto_retry, p.auto_retry);
        assert_eq!(loaded.sounds, p.sounds);
        assert_eq!(loaded.skills, p.skills);
        assert_eq!(loaded.webhook, p.webhook);
    }

    #[test]
    fn shared_keys_win_and_config_is_not_clobbered() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let home = test_home("shared");
        let cfg = home.join(".commandcode/config.json");
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();

        fs::write(
            &cfg,
            r#"{ "provider": "command-code", "theme": "dark", "model": "x" }"#,
        )
        .unwrap();

        let loaded = Prefs::load();

        loaded.save();
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(after["provider"], "command-code");
        assert_eq!(after["model"], "x");
        assert_eq!(after["theme"], "dark");
        assert_eq!(after["tasteLearning"], true);
    }

    #[test]
    fn load_without_home_uses_defaults() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let old = std::env::var("HOME").ok();
        let old_profile = std::env::var("USERPROFILE").ok();
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
        let p = Prefs::load();
        std::env::remove_var("HOME");
        assert_eq!(p.sidebar_w, 25);
        if let Some(h) = old {
            std::env::set_var("HOME", h);
        }
        if let Some(h) = old_profile {
            std::env::set_var("USERPROFILE", h);
        }
    }

    #[test]
    fn skill_injection_persists_through_save_load() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _home = test_home("skill_inj_roundtrip");

        let mut p = Prefs::default();
        p.skill_injection = true;
        p.save();

        let raw = fs::read_to_string(Prefs::local_path().unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            v.get("skill_injection").and_then(|x| x.as_bool()),
            Some(true),
            "skill_injection must be written by save()"
        );

        let loaded = Prefs::load();
        assert_eq!(loaded.skill_injection, true);
    }

    #[test]
    fn yolo_toggle_persists_immediately_in_both_stores() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _home = test_home("yolo_toggle");

        Prefs::set_yolo_mode(true);
        assert_eq!(Prefs::load().yolo_mode, true);
        let local: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(Prefs::local_path().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            local.get("yolo_mode").and_then(|v| v.as_bool()),
            Some(true),
            "the local store must carry the toggle without waiting for the sync loop"
        );
        let shared: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(Prefs::config_path().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            shared.get("yolo").and_then(|v| v.as_bool()),
            Some(true),
            "other dashboard instances read the shared store"
        );

        Prefs::set_yolo_mode(false);
        assert_eq!(Prefs::load().yolo_mode, false);
        let local: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(Prefs::local_path().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(local.get("yolo_mode").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn skill_injection_defaults_to_false_when_missing() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let home = test_home("skill_inj_default");
        let prefs_dir = home.join(".commandcode");
        fs::create_dir_all(&prefs_dir).unwrap();
        let legacy = serde_json::json!({
            "skills": {
                "injection_enabled": true,
                "injection_prompt": "Apply the appropriate skill if applicable to the task.",
            }
        });
        fs::write(
            prefs_dir.join("cc-dashboard-prefs.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        let loaded = Prefs::load();
        assert_eq!(
            loaded.skill_injection, false
        );
    }

    #[test]
    fn sound_prefs_persists_through_save_load() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _home = test_home("sound_prefs_roundtrip");

        let mut p = Prefs::default();
        p.sounds.enabled = false;
        p.sounds.sound_completed = "Ping".into();
        p.sounds.sound_blocked = "Basso".into();
        p.save();

        let raw = fs::read_to_string(Prefs::local_path().unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            v.get("sounds").and_then(|s| s.get("enabled")).and_then(|x| x.as_bool()),
            Some(false)
        );

        let loaded = Prefs::load();
        assert_eq!(loaded.sounds.enabled, false);
        assert_eq!(loaded.sounds.sound_completed, "Ping");
        assert_eq!(loaded.sounds.sound_blocked, "Basso");
    }

    #[test]
    fn skills_prefs_disabled_skills_toggle_and_persist() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _home = test_home("skills_prefs_toggle");

        let mut p = Prefs::default();
        assert!(p.skills.is_skill_enabled("craft"));
        p.skills.toggle_skill("craft");
        assert!(!p.skills.is_skill_enabled("craft"));
        assert_eq!(p.skills.disabled_skills, vec!["craft".to_string()]);
        p.save();

        let loaded = Prefs::load();
        assert!(!loaded.skills.is_skill_enabled("craft"));
        assert!(loaded.skills.is_skill_enabled("other"));

        let mut p2 = loaded;
        p2.skills.toggle_skill("craft");
        assert!(p2.skills.is_skill_enabled("craft"));
    }
}

