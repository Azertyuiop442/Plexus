
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
        assert_eq!(loaded.skills, p.skills);
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
        std::env::remove_var("HOME");
        let p = Prefs::load();
        std::env::remove_var("HOME");
        assert_eq!(p.sidebar_w, 25);
        if let Some(h) = old {
            std::env::set_var("HOME", h);
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
            loaded.skill_injection, false,
            "missing skill_injection in JSON must default to false (safe)"
        );
    }
}

