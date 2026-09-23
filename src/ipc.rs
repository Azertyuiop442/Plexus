
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
pub static HOME_LOCK: Mutex<()> = Mutex::new(());

pub fn home_dir() -> PathBuf {
    for key in ["HOME", "USERPROFILE"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }
    match (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        (Ok(drive), Ok(path)) if !drive.is_empty() && !path.is_empty() => {
            PathBuf::from(format!("{drive}{path}"))
        }
        _ => PathBuf::new(),
    }
}

pub fn default_sidebar_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::temp_dir().join("cc-sidebar")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/tmp/cc-sidebar")
    }
}

pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

pub fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("json"),
        std::process::id()
    ));
    std::fs::write(&tmp, content)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {

            let _ = std::fs::remove_file(&tmp);
            std::fs::write(path, content)
        }
    }
}

pub fn merge_write_json(path: &Path, patch: &serde_json::Map<String, serde_json::Value>) -> Option<()> {
    let mut map: serde_json::Map<String, serde_json::Value> =
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
    for (k, v) in patch {
        map.insert(k.clone(), v.clone());
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(map)).ok()?;
    atomic_write(path, &json).ok()?;
    Some(())
}

pub fn data_dir() -> PathBuf {
    let dir = std::env::var("CC_SIDEBAR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_sidebar_dir());
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    dir
}

pub fn data_dir_str() -> String {
    data_dir().to_string_lossy().to_string()
}

pub fn ipc_path(name: &str) -> String {
    format!("{}/{}", data_dir_str(), name)
}

const DIAG_LOG_MAX_BYTES: usize = 256 * 1024;

pub fn log_reset(name: &str) {
    let _ = std::fs::write(
        Path::new(&ipc_path(name)),
        format!("=== session {} starts ===\n", std::process::id()),
    );
}

pub fn log_append(name: &str, line: &str) {
    let path_str = ipc_path(name);
    let path = Path::new(&path_str);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let day_secs = (now_ms / 1000) % 86_400;
    let hms = format!(
        "{:02}:{:02}:{:02}.{:03}",
        day_secs / 3600,
        (day_secs / 60) % 60,
        day_secs % 60,
        now_ms % 1000
    );
    let entry = format!("[{hms}] {line}\n");
    let mut existing = std::fs::read(&path).unwrap_or_default();
    existing.extend_from_slice(entry.as_bytes());
    if existing.len() > DIAG_LOG_MAX_BYTES {
        existing = existing.split_off(existing.len() - DIAG_LOG_MAX_BYTES);
        if let Some(pos) = existing.iter().position(|&b| b == b'\n') {
            let _ = existing.drain(..=pos);
        }
    }
    let _ = std::fs::write(&path, existing);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvGuard {
        saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn clear(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|k| {
                    let old = std::env::var_os(k);
                    std::env::remove_var(k);
                    (*k, old)
                })
                .collect();
            Self { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    fn home_dir_prefers_home_over_userprofile() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::clear(&["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]);
        std::env::set_var("HOME", "/home/unix");
        std::env::set_var("USERPROFILE", "C:\\Users\\win");
        assert_eq!(home_dir(), PathBuf::from("/home/unix"));
    }

    #[test]
    fn home_dir_falls_back_to_userprofile() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::clear(&["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]);
        std::env::set_var("USERPROFILE", "C:\\Users\\win");
        assert_eq!(home_dir(), PathBuf::from("C:\\Users\\win"));
    }

    #[test]
    fn home_dir_falls_back_to_drive_and_path() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::clear(&["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]);
        std::env::set_var("HOMEDRIVE", "C:");
        std::env::set_var("HOMEPATH", "\\Users\\legacy");
        assert_eq!(home_dir(), PathBuf::from("C:\\Users\\legacy"));
    }

    #[test]
    fn home_dir_empty_when_no_variable_is_set() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::clear(&["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]);
        assert!(home_dir().as_os_str().is_empty());
    }

    #[test]
    fn home_dir_ignores_empty_home() {
        let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _env = EnvGuard::clear(&["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]);
        std::env::set_var("HOME", "");
        std::env::set_var("USERPROFILE", "C:\\Users\\win");
        assert_eq!(home_dir(), PathBuf::from("C:\\Users\\win"));
    }

    #[test]
    fn default_sidebar_dir_ends_with_cc_sidebar() {
        assert_eq!(
            default_sidebar_dir().file_name().and_then(|n| n.to_str()),
            Some("cc-sidebar")
        );
    }
}

