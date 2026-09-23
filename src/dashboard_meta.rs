use crate::ipc::ipc_path;

pub const DASHBOARD_BEAT_MS: u64 = 15_000;

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn dashboard_meta_json(pid: u32, cwd: &str, started_at_ms: u64, updated_at_ms: u64) -> String {
    serde_json::json!({
        "pid": pid,
        "cwd": cwd,
        "startedAt": started_at_ms,
        "updatedAt": updated_at_ms,
    })
    .to_string()
}

pub fn current_cwd() -> String {
    std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub fn write_dashboard_meta(started_at_ms: u64) {
    let pid = std::process::id();
    let _ = std::fs::write(ipc_path("dashboard.pid"), pid.to_string());
    let _ = std::fs::write(
        ipc_path("dashboard.json"),
        dashboard_meta_json(pid, &current_cwd(), started_at_ms, now_ms()),
    );
}

pub fn beat_dashboard_meta(started_at_ms: u64) {
    let pid = std::process::id().to_string();
    let current = std::fs::read_to_string(ipc_path("dashboard.pid")).unwrap_or_default();
    if current.trim() != pid {
        write_dashboard_meta(started_at_ms);
        return;
    }
    let _ = std::fs::write(
        ipc_path("dashboard.json"),
        dashboard_meta_json(std::process::id(), &current_cwd(), started_at_ms, now_ms()),
    );
}

pub fn remove_dashboard_meta_if_ours() {
    let pid = std::process::id().to_string();
    let current = std::fs::read_to_string(ipc_path("dashboard.pid")).unwrap_or_default();
    if current.trim() != pid {
        return;
    }
    let _ = std::fs::remove_file(ipc_path("dashboard.pid"));
    let _ = std::fs::remove_file(ipc_path("dashboard.json"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_meta_json_carries_pid_cwd_and_timestamps() {
        let raw = dashboard_meta_json(42, "/tmp/proj", 1000, 2000);
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["pid"], 42);
        assert_eq!(json["cwd"], "/tmp/proj");
        assert_eq!(json["startedAt"], 1000);
        assert_eq!(json["updatedAt"], 2000);
    }

    #[test]
    fn dashboard_meta_claims_and_releases_the_pid_file() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("cc-dashboard-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = std::env::var_os("CC_SIDEBAR_DIR");
        std::env::set_var("CC_SIDEBAR_DIR", &dir);

        write_dashboard_meta(1234);
        let pid = std::fs::read_to_string(dir.join("dashboard.pid")).unwrap();
        assert_eq!(pid.trim(), std::process::id().to_string());
        let meta: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("dashboard.json")).unwrap())
                .unwrap();
        assert_eq!(meta["pid"], std::process::id());
        assert_eq!(meta["cwd"], current_cwd());

        std::fs::write(dir.join("dashboard.pid"), "999999").unwrap();
        beat_dashboard_meta(1234);
        let pid = std::fs::read_to_string(dir.join("dashboard.pid")).unwrap();
        assert_eq!(
            pid.trim(),
            std::process::id().to_string(),
            "a foreign pid must be reclaimed by the live dashboard"
        );

        remove_dashboard_meta_if_ours();
        assert!(!dir.join("dashboard.pid").exists());
        assert!(!dir.join("dashboard.json").exists());

        std::fs::write(dir.join("dashboard.pid"), "999999").unwrap();
        std::fs::write(dir.join("dashboard.json"), "{}").unwrap();
        remove_dashboard_meta_if_ours();
        assert!(
            dir.join("dashboard.pid").exists(),
            "another dashboard's files must survive"
        );

        match old {
            Some(v) => std::env::set_var("CC_SIDEBAR_DIR", v),
            None => std::env::remove_var("CC_SIDEBAR_DIR"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
