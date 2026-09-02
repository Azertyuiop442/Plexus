use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::mux_events::{MuxEvent, MuxEventSender};
use crate::prefs::Prefs;

const TRACKING_FILENAME: &str = ".tracking.json";
const ERRORS_LOG: &str = "skills-errors.log";
const GIT_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendorStatus {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub has_repo: bool,
    #[serde(default)]
    pub local_sha: String,
    #[serde(default)]
    pub remote_sha: String,
    #[serde(default)]
    pub behind: usize,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_check: u64,
    #[serde(default)]
    pub last_update: u64,
}

impl VendorStatus {
    pub fn is_stale(&self) -> bool {
        self.has_repo && self.behind > 0
    }
}

#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub name: String,
    pub skill_md: PathBuf,
    pub extra_files: Vec<PathBuf>,
}

fn home() -> PathBuf {
    crate::ipc::home_dir()
}

fn skills_root() -> PathBuf {
    home().join(".commandcode").join("skills")
}

fn vendor_path(vendor: &str) -> PathBuf {
    skills_root().join(vendor)
}

fn tracking_path(vendor: &str) -> PathBuf {
    vendor_path(vendor).join(TRACKING_FILENAME)
}

pub fn tracking_path_public(vendor: &str) -> PathBuf {
    tracking_path(vendor)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn log_err(line: &str) {
    crate::ipc::log_append(ERRORS_LOG, line);
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name.len() <= 128
}

fn run_git(cwd: &Path, args: &[&str]) -> Option<String> {
    let _ = GIT_TIMEOUT_SECS;
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output()
        .ok()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log_err(&format!(
            "git {} in {} failed: stderr={} stdout={}",
            args.join(" "),
            cwd.display(),
            stderr.trim(),
            stdout.trim()
        ));
        return None;
    }
    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn read_tracking(vendor: &str) -> Option<VendorStatus> {
    let raw = fs::read_to_string(tracking_path(vendor)).ok()?;
    let mut status: VendorStatus = serde_json::from_str(&raw).ok()?;
    status.name = vendor.to_string();
    Some(status)
}

fn write_tracking(status: &VendorStatus) -> Result<(), String> {
    let path = tracking_path(&status.name);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(status).map_err(|e| e.to_string())?;
    crate::ipc::atomic_write(&path, &json).map_err(|e| e.to_string())
}

pub fn discover_vendors() -> Vec<String> {
    let root = skills_root();
    let entries = match fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            log_err(&format!("discover_vendors: cannot read {}: {e}", root.display()));
            return Vec::new();
        }
    };
    let mut out: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            if is_hidden_name(&name) || !is_safe_name(&name) {
                return None;
            }
            if vendor_has_skill(&path) {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    out.sort();
    out
}

fn vendor_has_skill(vendor_dir: &Path) -> bool {
    let entries = match fs::read_dir(vendor_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };
    entries.filter_map(|e| e.ok()).any(|entry| {
        let p = entry.path();
        if !p.is_dir() {
            return false;
        }
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => return false,
        };
        if is_hidden_name(name) {
            return false;
        }
        p.join("SKILL.md").is_file()
    })
}

pub fn discover_skills(vendor: &str) -> Vec<SkillEntry> {
    let root = vendor_path(vendor);
    let entries = match fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            log_err(&format!(
                "discover_skills({vendor}): cannot read {}: {e}",
                root.display()
            ));
            return Vec::new();
        }
    };
    let mut out: Vec<SkillEntry> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            if is_hidden_name(&name) || !is_safe_name(&name) {
                return None;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.is_file() {
                return None;
            }
            let extra = skill_files_in(&path);
            Some(SkillEntry {
                name: name.clone(),
                skill_md,
                extra_files: extra,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn skill_files_in(skill_dir: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(skill_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()) != Some("SKILL.md"))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !is_hidden_name(n))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

pub fn read_skill_md(skill: &SkillEntry, max_lines: usize) -> String {
    let raw = match fs::read_to_string(&skill.skill_md) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    raw.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

pub fn normalize_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if !trimmed.contains("://") && !trimmed.starts_with("git@") && !trimmed.starts_with('/') {
        if trimmed.split('/').count() == 2 && !trimmed.contains(' ') {
            let clean = trimmed.trim_end_matches(".git");
            return format!("https://github.com/{clean}.git");
        }
    }
    trimmed.to_string()
}

pub fn vendor_status(name: &str) -> VendorStatus {
    let folder = vendor_path(name);
    let mut status = read_tracking(name).unwrap_or_else(|| VendorStatus {
        name: name.to_string(),
        url: None,
        has_repo: false,
        local_sha: String::new(),
        remote_sha: String::new(),
        behind: 0,
        branch: "main".into(),
        last_error: None,
        last_check: 0,
        last_update: 0,
    });

    if status.url.is_none() && folder.join(".git").exists() {
        status.url = run_git(&folder, &["config", "--get", "remote.origin.url"]);
    }

    if let Some(ref u) = status.url {
        if !u.trim().is_empty() {
            status.has_repo = true;
        }
    }

    status
}

pub fn check_vendor_remote(name: &str) -> VendorStatus {
    let mut status = vendor_status(name);
    let Some(raw_url) = status.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
        status.has_repo = false;
        status.last_error = None;
        let _ = write_tracking(&status);
        return status;
    };
    let normalized = normalize_url(raw_url);
    status.has_repo = true;

    let output = Command::new("git")
        .args(["ls-remote", &normalized, "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let remote_sha = stdout
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().next())
                .unwrap_or("")
                .trim()
                .to_string();

            if !remote_sha.is_empty() {
                status.remote_sha = remote_sha.clone();
                if status.local_sha.is_empty() {
                    status.local_sha = remote_sha;
                }
                status.behind = if status.local_sha != status.remote_sha { 1 } else { 0 };
                status.last_error = None;
                status.last_check = now_secs();
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            status.last_error = Some(format!("git remote check failed: {}", stderr.trim()));
            status.last_check = now_secs();
        }
        Err(e) => {
            status.last_error = Some(format!("spawn git failed: {e}"));
            status.last_check = now_secs();
        }
    }

    let _ = write_tracking(&status);
    status
}

pub fn vendor_statuses() -> Vec<VendorStatus> {
    let mut out: Vec<VendorStatus> = discover_vendors()
        .into_iter()
        .map(|v| vendor_status(&v))
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn validate_url(url: &str) -> Result<String, String> {
    let normalized = normalize_url(url);
    if normalized.is_empty() {
        return Err("URL is empty".into());
    }
    if normalized.len() > 512 {
        return Err("URL too long".into());
    }
    let looks_like = normalized.contains("://")
        || normalized.starts_with("git@")
        || normalized.starts_with('/');
    if !looks_like {
        return Err("URL must contain :// or start with git@".into());
    }
    Ok(normalized)
}

pub fn attach_url(vendor: &str, url: &str) -> Result<VendorStatus, String> {
    if !is_safe_name(vendor) {
        return Err(format!("invalid vendor name: {vendor}"));
    }
    let valid_url = validate_url(url)?;
    let folder = vendor_path(vendor);
    if !folder.exists() {
        return Err(format!("vendor folder not found: {vendor}"));
    }

    let mut status = read_tracking(vendor).unwrap_or_else(|| VendorStatus {
        name: vendor.to_string(),
        url: None,
        has_repo: true,
        local_sha: String::new(),
        remote_sha: String::new(),
        behind: 0,
        branch: "main".into(),
        last_error: None,
        last_check: 0,
        last_update: 0,
    });
    status.url = Some(valid_url.clone());
    status.has_repo = true;
    status.last_error = None;

    if !folder.join(".git").exists() {
        let _ = run_git(&folder, &["init", "--quiet"]);
    }
    let _ = run_git(&folder, &["remote", "remove", "origin"]);
    let _ = run_git(&folder, &["remote", "add", "origin", &valid_url]);

    let _ = write_tracking(&status);
    Ok(vendor_status(vendor))
}

pub fn pull_vendor(vendor: &str) -> Result<String, String> {
    let status = vendor_status(vendor);
    let Some(url) = status.url.filter(|u| !u.trim().is_empty()) else {
        return Err(format!("vendor '{vendor}' has no remote URL configured"));
    };

    let (_, count) = install_skills_bundle(&url, Some(vendor))?;
    let mut updated_status = vendor_status(vendor);
    updated_status.local_sha = updated_status.remote_sha.clone();
    updated_status.behind = 0;
    updated_status.last_update = now_secs();
    updated_status.last_error = None;
    let _ = write_tracking(&updated_status);

    Ok(format!("Updated {count} skill(s) for {vendor}"))
}

#[allow(dead_code)]
pub fn update_all_behind() -> Vec<(String, Result<String, String>)> {
    let mut out = Vec::new();
    for status in vendor_statuses() {
        if !status.is_stale() {
            continue;
        }
        let name = status.name.clone();
        let result = pull_vendor(&name);
        out.push((name, result));
    }
    out
}

pub fn update_all_behind_async(events: MuxEventSender) {
    let targets: Vec<String> = vendor_statuses()
        .into_iter()
        .filter(|s| s.is_stale())
        .map(|s| s.name)
        .collect();
    let total = targets.len();
    std::thread::spawn(move || {
        let mut ok = 0usize;
        let mut failed = 0usize;
        for (idx, name) in targets.iter().enumerate() {
            let done = idx;
            let result = pull_vendor(name);
            let last_result = match &result {
                Ok(_) => {
                    ok += 1;
                    Some(format!("✓ {name}"))
                }
                Err(e) => {
                    failed += 1;
                    Some(format!("✗ {name}: {e}"))
                }
            };
            let _ = events.send(MuxEvent::SkillsUpdateProgress {
                done: done + 1,
                total,
                current: name.clone(),
                last_result,
            });
        }
        let _ = events.send(MuxEvent::SkillsUpdateDone { ok, failed });
    });
}

pub fn install_skills_bundle(url: &str, vendor_override: Option<&str>) -> Result<(String, usize), String> {
    let valid_url = validate_url(url)?;
    let clean = valid_url.trim().trim_end_matches(".git");
    let mut parts: Vec<&str> = if clean.contains("://") {
        clean.split("://").nth(1).unwrap_or("").split('/').collect()
    } else {
        clean.split('/').collect()
    };
    if parts.len() > 2 && (parts[0] == "github.com" || parts[0] == "gitlab.com") {
        parts.remove(0);
    }
    let default_vendor = parts.first().copied().unwrap_or("custom");
    let default_repo = parts.get(1).copied().unwrap_or("skills");
    let chosen_vendor = vendor_override
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_vendor);

    if !is_safe_name(chosen_vendor) {
        return Err(format!("invalid vendor name: {chosen_vendor}"));
    }

    let temp_dir = std::env::temp_dir().join(format!("cc-skill-dl-{}", now_secs()));
    let _ = fs::remove_dir_all(&temp_dir);
    let _ = fs::create_dir_all(&temp_dir);

    let clone_res = Command::new("git")
        .current_dir(&temp_dir)
        .args(["clone", "--depth", "1", valid_url.trim(), "repo"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output()
        .map_err(|e| format!("failed to spawn git clone: {e}"))?;

    if !clone_res.status.success() {
        let _ = fs::remove_dir_all(&temp_dir);
        let err = String::from_utf8_lossy(&clone_res.stderr);
        return Err(format!("git clone failed: {}", err.trim()));
    }

    let repo_dir = temp_dir.join("repo");
    let head_sha = run_git(&repo_dir, &["rev-parse", "HEAD"]).unwrap_or_default();
    let dest_vendor_dir = skills_root().join(chosen_vendor);
    let _ = fs::create_dir_all(&dest_vendor_dir);

    let mut installed_count = 0;
    let mut mds = Vec::new();
    find_skill_mds(&repo_dir, &mut mds);

    for md_path in &mds {
        let parent = match md_path.parent() {
            Some(p) => p,
            None => continue,
        };
        let skill_name = if parent == repo_dir {
            default_repo.to_string()
        } else {
            parent.file_name().and_then(|n| n.to_str()).unwrap_or(default_repo).to_string()
        };

        if !is_safe_name(&skill_name) {
            continue;
        }

        let target_skill_dir = dest_vendor_dir.join(&skill_name);
        let _ = fs::create_dir_all(&target_skill_dir);

        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.filter_map(|e| e.ok()) {
                let src_p = entry.path();
                let fname = match src_p.file_name() {
                    Some(f) => f,
                    None => continue,
                };
                let dst_p = target_skill_dir.join(fname);
                if src_p.is_file() {
                    let _ = fs::copy(&src_p, &dst_p);
                } else if src_p.is_dir() {
                    let dname = fname.to_str().unwrap_or("");
                    if !is_hidden_name(dname) && dname != "node_modules" {
                        let _ = copy_dir_all(&src_p, &dst_p);
                    }
                }
            }
        }
        installed_count += 1;
    }

    let repo_refs = repo_dir.join("references");
    if repo_refs.is_dir() {
        let _ = copy_dir_all(&repo_refs, &dest_vendor_dir.join("references"));
    }

    let _ = attach_url(chosen_vendor, &valid_url);
    if !head_sha.is_empty() {
        let mut st = vendor_status(chosen_vendor);
        st.local_sha = head_sha.clone();
        st.remote_sha = head_sha;
        st.behind = 0;
        st.last_update = now_secs();
        st.last_error = None;
        let _ = write_tracking(&st);
    }
    let _ = fs::remove_dir_all(&temp_dir);

    if installed_count == 0 {
        return Err("No SKILL.md found in the cloned repository".into());
    }

    Ok((chosen_vendor.to_string(), installed_count))
}

fn find_skill_mds(dir: &Path, acc: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !is_hidden_name(name) && name != "node_modules" && name != "target" {
                    find_skill_mds(&p, acc);
                }
            } else if p.is_file() {
                let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if fname.eq_ignore_ascii_case("SKILL.md") {
                    acc.push(p);
                }
            }
        }
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn count_updates() -> usize {
    vendor_statuses().iter().filter(|s| s.is_stale()).count()
}

fn max_last_check() -> u64 {
    vendor_statuses()
        .iter()
        .map(|s| s.last_check)
        .max()
        .unwrap_or(0)
}

pub fn check_all_background(events: MuxEventSender) {
    let cooldown = Prefs::load().skills.check_cooldown_secs.max(1);
    let now = now_secs();
    let last = max_last_check();
    if last > 0 && now.saturating_sub(last) < cooldown {
        let snapshot: Vec<VendorStatus> = vendor_statuses();
        let _ = events.send(MuxEvent::SkillsUpdated { vendors: snapshot });
        return;
    }

    std::thread::spawn(move || {
        let names = discover_vendors();
        let results: Vec<VendorStatus> = names.iter().map(|n| check_vendor_remote(n)).collect();
        let _ = events.send(MuxEvent::SkillsUpdated { vendors: results });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static HOME_LOCK: &Mutex<()> = &crate::ipc::HOME_LOCK;

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

    fn fresh_home(label: &str) -> TestHome {
        let orig = std::env::var_os("HOME");
        let path = std::env::temp_dir().join(format!(
            "cc-skills-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        std::env::set_var("HOME", &path);
        TestHome { orig, path }
    }

    fn seed_skill(vendor: &str, skill: &str) {
        let folder = home().join(".commandcode/skills").join(vendor).join(skill);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("SKILL.md"), "# stub").unwrap();
    }

    #[test]
    fn discover_vendors_returns_folders_containing_skill_md() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let h = fresh_home("discover-vendors");
        let skills = h.join(".commandcode/skills");
        fs::create_dir_all(&skills).unwrap();
        fs::create_dir_all(skills.join("alpha/one")).unwrap();
        fs::write(skills.join("alpha/one/SKILL.md"), "x").unwrap();
        fs::create_dir_all(skills.join("alpha/two")).unwrap();
        fs::write(skills.join("alpha/two/SKILL.md"), "x").unwrap();
        fs::create_dir_all(skills.join("beta/only")).unwrap();
        fs::write(skills.join("beta/only/SKILL.md"), "x").unwrap();
        fs::create_dir_all(skills.join("orphan")).unwrap();
        fs::write(skills.join("orphan/README.md"), "x").unwrap();
        let names = discover_vendors();
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn discover_skills_returns_skill_folders() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("discover-skills");
        seed_skill("alpha", "one");
        seed_skill("alpha", "two");
        let skills = discover_skills("alpha");
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "one");
        assert!(skills[0].skill_md.ends_with("alpha/one/SKILL.md"));
    }

    #[test]
    fn vendor_status_reads_tracking_json() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let h = fresh_home("tracking-json");
        seed_skill("alpha", "one");
        let payload = serde_json::json!({
            "name": "alpha",
            "url": "https://example.com/alpha.git",
            "has_repo": true,
            "local_sha": "abc",
            "remote_sha": "def",
            "behind": 2,
            "branch": "main",
            "last_check": 1000,
            "last_update": 500,
        });
        let path = h.join(".commandcode/skills/alpha/.tracking.json");
        fs::write(&path, payload.to_string()).unwrap();
        let status = vendor_status("alpha");
        assert_eq!(status.url.as_deref(), Some("https://example.com/alpha.git"));
        assert_eq!(status.name, "alpha");
    }

    #[test]
    fn attach_url_rejects_unsafe_vendor_name() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("attach-unsafe");
        let result = attach_url("../etc", "https://x");
        assert!(result.is_err());
    }

    #[test]
    fn attach_url_rejects_invalid_url() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("attach-bad-url");
        seed_skill("alpha", "one");
        assert!(attach_url("alpha", "not a url").is_err());
        assert!(attach_url("alpha", "").is_err());
    }

    #[test]
    fn count_updates_zero_when_no_vendors_are_stale() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("count-zero");
        seed_skill("alpha", "one");
        let n = count_updates();
        assert_eq!(n, 0);
    }

    #[test]
    fn install_skills_bundle_rejects_invalid_url() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("install-bad-url");
        assert!(install_skills_bundle("invalid url", None).is_err());
        assert!(install_skills_bundle("", None).is_err());
    }

    #[test]
    fn install_and_update_lifecycle_roundtrip() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("install-cycle");

        let remote_dir = std::env::temp_dir().join(format!(
            "cc-fake-remote-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&remote_dir);
        let skill_dir = remote_dir.join("sample-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Initial Skill").unwrap();

        let _ = Command::new("git").args(["init"]).current_dir(&remote_dir).output();
        let _ = Command::new("git").args(["config", "user.name", "Test"]).current_dir(&remote_dir).output();
        let _ = Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(&remote_dir).output();
        let _ = Command::new("git").args(["add", "."]).current_dir(&remote_dir).output();
        let _ = Command::new("git").args(["commit", "-m", "initial"]).current_dir(&remote_dir).output();

        let remote_path_str = remote_dir.to_str().unwrap();
        let res = install_skills_bundle(remote_path_str, Some("testvendor"));
        assert!(res.is_ok());
        let (vendor, count) = res.unwrap();
        assert_eq!(vendor, "testvendor");
        assert_eq!(count, 1);

        let skills = discover_skills("testvendor");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "sample-skill");

        let status = vendor_status("testvendor");
        assert!(status.has_repo);
        assert_eq!(status.behind, 0);
        assert!(!status.local_sha.is_empty());
        assert_eq!(status.local_sha, status.remote_sha);

        fs::write(skill_dir.join("SKILL.md"), "# Updated Skill").unwrap();
        let _ = Command::new("git").args(["add", "."]).current_dir(&remote_dir).output();
        let _ = Command::new("git").args(["commit", "-m", "update"]).current_dir(&remote_dir).output();

        let checked = check_vendor_remote("testvendor");
        assert!(checked.is_stale());
        assert_eq!(checked.behind, 1);

        let pull_res = pull_vendor("testvendor");
        assert!(pull_res.is_ok());

        let after_pull = vendor_status("testvendor");
        assert_eq!(after_pull.behind, 0);
        assert!(!after_pull.is_stale());
        assert_eq!(after_pull.local_sha, after_pull.remote_sha);

        let content = fs::read_to_string(&skills[0].skill_md).unwrap();
        assert_eq!(content, "# Updated Skill");

        let _ = fs::remove_dir_all(&remote_dir);
    }

    #[test]
    fn install_real_skills_bundle_from_github() {
        let _g = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _h = fresh_home("install-github");
        let res = install_skills_bundle("netresearch/agent-rules-skill", None);
        if let Ok((vendor, count)) = res {
            assert_eq!(vendor, "netresearch");
            assert!(count >= 1);
            let skills = discover_skills("netresearch");
            assert!(!skills.is_empty());
            let status = vendor_status("netresearch");
            assert!(status.has_repo);
            assert_eq!(status.behind, 0);
            assert!(!status.local_sha.is_empty());
        }
    }
}
