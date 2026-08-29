use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mux_events::{MuxEvent, MuxEventSender};

const CHECK_COOLDOWN_SECS: u64 = 300;

pub fn parse_semver(raw: &str) -> (u32, u32, u32) {
    let clean = raw.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = clean.split('.');
    let major = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| {
        let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
    }).unwrap_or(0);
    (major, minor, patch)
}

pub fn is_newer_version(remote: &str, current: &str) -> bool {
    parse_semver(remote) > parse_semver(current)
}

fn cache_path() -> PathBuf {
    crate::ipc::home_dir().join(".commandcode/update-check.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn local_dev_version() -> Option<String> {
    let path = crate::ipc::home_dir().join(".commandcode/mods/cc-dashboard/Cargo.toml");
    let raw = fs::read_to_string(path).ok()?;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("version =") {
            let ver = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

pub fn current_version() -> String {
    let bin_ver = env!("CARGO_PKG_VERSION");
    if let Some(dev_ver) = local_dev_version() {
        if !is_newer_version(bin_ver, &dev_ver) {
            return dev_ver;
        }
    }
    bin_ver.to_string()
}

fn read_cached_update() -> Option<String> {
    let path = cache_path();
    let raw = fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let last_check = json.get("last_check").and_then(|v| v.as_u64()).unwrap_or(0);
    let current_ver = current_version();
    if now_secs().saturating_sub(last_check) < CHECK_COOLDOWN_SECS {
        let latest = json.get("latest_version").and_then(|v| v.as_str())?;
        if is_newer_version(latest, &current_ver) {
            return Some(latest.to_string());
        }
    }
    None
}

fn write_cached_update(latest: &str) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let current_ver = current_version();
    let payload = serde_json::json!({
        "last_check": now_secs(),
        "latest_version": latest,
        "current_version": current_ver
    });
    let _ = fs::write(path, payload.to_string());
}



fn fetch_github_version(repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let output = Command::new("curl")
        .args(["-s", "-m", "3", "-H", "User-Agent: cc-dashboard", &url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    json.get("tag_name")
        .or_else(|| json.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn fetch_raw_cargo_version(url: &str) -> Option<String> {
    let output = Command::new("curl")
        .args(["-s", "-m", "3", url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?;
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("version =") {
            let ver = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

pub fn check_remote_version() -> Option<String> {
    let gh_candidates = ["Azertyuiop442/Plexus"];
    let raw_cargo_urls = [
        "https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/Cargo.toml",
        "https://raw.githubusercontent.com/Azertyuiop442/Plexus/main/Cargo.toml",
    ];

    let mut best_version: Option<String> = None;

    for url in raw_cargo_urls {
        if let Some(v) = fetch_raw_cargo_version(url) {
            if let Some(ref current_best) = best_version {
                if is_newer_version(&v, current_best) {
                    best_version = Some(v);
                }
            } else {
                best_version = Some(v);
            }
        }
    }

    for repo in gh_candidates {
        if let Some(v) = fetch_github_version(repo) {
            if let Some(ref current_best) = best_version {
                if is_newer_version(&v, current_best) {
                    best_version = Some(v);
                }
            } else {
                best_version = Some(v);
            }
        }
    }

    best_version
}

pub fn check_for_updates_background(events: MuxEventSender) {
    if let Some(cached) = read_cached_update() {
        let _ = events.send(MuxEvent::UpdateAvailable { version: cached });
    }

    std::thread::spawn(move || {
        let current = current_version();
        if let Some(remote) = check_remote_version() {
            write_cached_update(&remote);
            if is_newer_version(&remote, &current) {
                let _ = events.send(MuxEvent::UpdateAvailable { version: remote });
            }
        } else {
            write_cached_update(&current);
        }
    });
}

pub fn perform_update_with_events(events: MuxEventSender) {
    std::thread::spawn(move || {
        let _ = events.send(MuxEvent::UpdateProgress {
            label: "Preparing build environment...".into(),
            current: 15,
            total: 100,
        });

        let home = crate::ipc::home_dir();
        let install_sh = home.join(".commandcode/mods/cc-dashboard/install.sh");

        let log_file = match std::fs::File::create("/tmp/plexus-update.log") {
            Ok(f) => f,
            Err(e) => {
                let _ = events.send(MuxEvent::UpdateCompleted {
                    success: false,
                    error: Some(format!("Could not create log file: {e}")),
                });
                return;
            }
        };
        let err_file = match log_file.try_clone() {
            Ok(f) => f,
            Err(e) => {
                let _ = events.send(MuxEvent::UpdateCompleted {
                    success: false,
                    error: Some(format!("Could not clone log file: {e}")),
                });
                return;
            }
        };

        let _ = events.send(MuxEvent::UpdateProgress {
            label: "Compiling release binary...".into(),
            current: 45,
            total: 100,
        });

        let status = if install_sh.exists() {
            Command::new("bash")
                .arg(install_sh)
                .stdout(std::process::Stdio::from(log_file))
                .stderr(std::process::Stdio::from(err_file))
                .status()
        } else {
            Command::new("bash")
                .args(["-c", "curl -fsSL https://raw.githubusercontent.com/Azertyuiop442/Plexus/public/install.sh | bash"])
                .stdout(std::process::Stdio::from(log_file))
                .stderr(std::process::Stdio::from(err_file))
                .status()
        };

        match status {
            Ok(s) if s.success() => {
                let _ = events.send(MuxEvent::UpdateProgress {
                    label: "✓ Installed! Reloading Plexus...".into(),
                    current: 100,
                    total: 100,
                });
                std::thread::sleep(std::time::Duration::from_millis(600));
                let _ = events.send(MuxEvent::UpdateCompleted {
                    success: true,
                    error: None,
                });
            }
            Ok(s) => {
                let err_msg = format!("Installer exited with code {:?}", s.code());
                let _ = events.send(MuxEvent::UpdateCompleted {
                    success: false,
                    error: Some(err_msg),
                });
            }
            Err(e) => {
                let _ = events.send(MuxEvent::UpdateCompleted {
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    });
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_semver_extracts_correct_components() {
        assert_eq!(parse_semver("0.1.1"), (0, 1, 1));
        assert_eq!(parse_semver("v1.2.3"), (1, 2, 3));
        assert_eq!(parse_semver("V2.0.0-beta.1"), (2, 0, 0));
        assert_eq!(parse_semver("invalid"), (0, 0, 0));
    }

    #[test]
    fn is_newer_version_detects_upgrades() {
        assert!(is_newer_version("0.1.2", "0.1.1"));
        assert!(is_newer_version("0.2.0", "0.1.9"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(!is_newer_version("0.1.1", "0.1.1"));
        assert!(!is_newer_version("0.1.0", "0.1.1"));
    }
}
