
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
pub static HOME_LOCK: Mutex<()> = Mutex::new(());

pub fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_default()
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
        .unwrap_or_else(|_| PathBuf::from("/tmp/cc-sidebar"));
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

