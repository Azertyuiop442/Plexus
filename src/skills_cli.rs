use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn home() -> PathBuf {
    crate::ipc::home_dir()
}

pub fn parse_skill_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let after_cmd = if let Some(idx) = trimmed.find("add ") {
        &trimmed[idx + 4..]
    } else {
        trimmed
    };
    let mut tokens = after_cmd.split_whitespace();
    while let Some(tok) = tokens.next() {
        if tok.starts_with('-') {
            if tok == "-a" || tok == "--agent" || tok == "-s" || tok == "--skill" {
                let _ = tokens.next();
            }
            continue;
        }
        return tok.trim_matches(|c| c == '\'' || c == '"').to_string();
    }
    trimmed.to_string()
}

pub fn read_skill_lock_url(skill_name: &str) -> Option<String> {
    let lock_path = home().join(".agents").join(".skill-lock.json");
    let raw = fs::read_to_string(lock_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let skill_obj = value.get("skills")?.get(skill_name)?;

    if let Some(url) = skill_obj.get("sourceUrl").and_then(|u| u.as_str()) {
        if !url.trim().is_empty() {
            return Some(url.trim().to_string());
        }
    }

    if let Some(source) = skill_obj.get("source").and_then(|s| s.as_str()) {
        let trimmed = source.trim();
        if trimmed.contains('/') && !trimmed.contains("://") {
            return Some(format!("https://github.com/{trimmed}.git"));
        }
    }

    None
}

pub fn install_via_skills_cli(target: &str) -> Result<(String, usize), String> {
    let clean = parse_skill_input(target);
    if clean.is_empty() {
        return Err("Skill package or repository target is empty".into());
    }
    let cmd = format!("npx -y skills add {} -g -a command-code -y", clean);
    let output = Command::new("sh")
        .args(["-c", &cmd])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output()
        .map_err(|e| format!("failed to spawn npx: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err = if !stderr.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        return Err(format!("skills add failed: {err}"));
    }

    let vendor_name = if clean.contains('/') && !clean.contains("://") {
        clean.split('/').next().unwrap_or("custom").to_string()
    } else {
        clean
            .trim_end_matches(".git")
            .split('/')
            .last()
            .unwrap_or("custom")
            .to_string()
    };
    Ok((vendor_name, 1))
}

pub fn run_skills_cli_update() -> Result<String, String> {
    let output = Command::new("sh")
        .args(["-c", "npx -y skills update -g -y"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output()
        .map_err(|e| format!("failed to spawn npx update: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err = if !stderr.trim().is_empty() {
            stderr.trim()
        } else {
            stdout.trim()
        };
        return Err(format!("update failed: {err}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with('✓') || trimmed.contains("up to date") || trimmed.contains("Updated") {
            return Ok(trimmed.to_string());
        }
    }
    Ok("Skills update completed".to_string())
}

pub fn remove_from_skill_lock(skill_name: &str) {
    let lock_path = home().join(".agents").join(".skill-lock.json");
    let Ok(raw) = fs::read_to_string(&lock_path) else {
        return;
    };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    if let Some(skills) = value.get_mut("skills").and_then(|s| s.as_object_mut()) {
        if skills.remove(skill_name).is_some() {
            if let Ok(serialized) = serde_json::to_string_pretty(&value) {
                let _ = fs::write(&lock_path, serialized);
            }
        }
    }
}

pub fn uninstall_skill(skill_name: &str) -> Result<String, String> {
    let clean = skill_name.trim();
    if clean.is_empty() {
        return Err("Skill name is empty".into());
    }

    let all_skills = crate::skills_meta::discover_all_skills_meta();
    let meta = all_skills.into_iter().find(|s| s.name == clean);

    if let Some(m) = meta {
        if let Some(parent_dir) = m.path.parent() {
            if parent_dir.exists() {
                let _ = fs::remove_dir_all(parent_dir);
            }
            if !m.is_flat {
                if let Some(vendor_dir) = parent_dir.parent() {
                    let is_empty = fs::read_dir(vendor_dir)
                        .map(|mut r| r.next().is_none())
                        .unwrap_or(false);
                    if is_empty {
                        let _ = fs::remove_dir(vendor_dir);
                    }
                }
            }
        }
    }

    let cmd = format!("npx -y skills remove {} -g -y", clean);
    let _ = Command::new("sh")
        .args(["-c", &cmd])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "echo")
        .output();

    remove_from_skill_lock(clean);

    let mut prefs = crate::prefs::Prefs::load();
    if prefs.skills.disabled_skills.iter().any(|s| s == clean) {
        prefs.skills.disabled_skills.retain(|s| s != clean);
        prefs.save();
    }

    Ok(format!("✓ Successfully uninstalled {clean}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skill_input_handles_npx_command() {
        assert_eq!(
            parse_skill_input("npx skills add gustavo-fior/craft"),
            "gustavo-fior/craft"
        );
        assert_eq!(
            parse_skill_input("npx -y skills add gustavo-fior/craft --copy -y"),
            "gustavo-fior/craft"
        );
        assert_eq!(
            parse_skill_input("skills add addyosmani/agent-skills -g"),
            "addyosmani/agent-skills"
        );
    }

    #[test]
    fn parse_skill_input_handles_plain_repo_and_urls() {
        assert_eq!(parse_skill_input("gustavo-fior/craft"), "gustavo-fior/craft");
        assert_eq!(
            parse_skill_input("https://github.com/gustavo-fior/craft.git"),
            "https://github.com/gustavo-fior/craft.git"
        );
        assert_eq!(
            parse_skill_input("git@github.com:gustavo-fior/craft.git"),
            "git@github.com:gustavo-fior/craft.git"
        );
    }

    #[test]
    fn uninstall_skill_validates_empty_name() {
        assert!(uninstall_skill("").is_err());
        assert!(uninstall_skill("   ").is_err());
    }
}
