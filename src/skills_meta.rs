use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillMeta {
    pub name: String,
    pub vendor: String,
    pub description: String,
    pub path: PathBuf,
    pub is_flat: bool,
    pub extra_files_count: usize,
}

pub fn parse_frontmatter_name(raw: &str) -> Option<String> {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return None;
    }
    for line in lines.iter().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            let val = rest.trim().trim_matches(|c| c == '\'' || c == '"');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

pub fn parse_frontmatter_description(raw: &str) -> Option<String> {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return extract_first_paragraph(raw);
    }

    let mut in_desc = false;
    let mut desc_lines: Vec<String> = Vec::new();

    for line in lines.iter().skip(1) {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            break;
        }

        if trimmed.starts_with("description:") {
            in_desc = true;
            let after = trimmed["description:".len()..].trim();
            let cleaned = after.trim_matches(|c| c == '>' || c == '|').trim();
            let unquoted = cleaned.trim_matches(|c| c == '\'' || c == '"').trim();
            if !unquoted.is_empty() {
                desc_lines.push(unquoted.to_string());
            }
            continue;
        }

        if in_desc {
            let is_indented = line.starts_with(' ') || line.starts_with('\t');
            if is_indented && !trimmed.is_empty() {
                let unquoted = trimmed.trim_matches(|c| c == '\'' || c == '"').trim();
                desc_lines.push(unquoted.to_string());
            } else if !trimmed.is_empty() {
                break;
            }
        }
    }

    if !desc_lines.is_empty() {
        let joined = desc_lines.join(" ");
        let cleaned = joined.split_whitespace().collect::<Vec<_>>().join(" ");
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }

    extract_first_paragraph(raw)
}

fn extract_first_paragraph(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("---")
            || trimmed.starts_with("```")
        {
            continue;
        }
        return Some(trimmed.to_string());
    }
    None
}

pub fn load_skill_meta(path: &Path, fallback_name: &str, vendor: &str, is_flat: bool, extra_files_count: usize) -> SkillMeta {
    let content = fs::read_to_string(path).unwrap_or_default();
    let name = parse_frontmatter_name(&content).unwrap_or_else(|| fallback_name.to_string());
    let description = parse_frontmatter_description(&content).unwrap_or_else(|| "Agent skill".to_string());
    SkillMeta {
        name,
        vendor: vendor.to_string(),
        description,
        path: path.to_path_buf(),
        is_flat,
        extra_files_count,
    }
}

pub fn resolve_family(path: &Path, skill_name: &str, folder_vendor: &str, is_flat: bool) -> String {
    if !is_flat {
        return folder_vendor.to_string();
    }

    if let Some(parent) = path.parent() {
        let tracking_file = parent.join(".tracking.json");
        if let Ok(raw) = fs::read_to_string(&tracking_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(url) = val.get("url").and_then(|u| u.as_str()) {
                    let clean = url.trim().trim_end_matches(".git");
                    if let Some(repo) = clean.split('/').last() {
                        let repo = repo.trim();
                        if !repo.is_empty() && repo != "skills" && repo != "agent-skills" {
                            return repo.to_string();
                        }
                    }
                }
            }
        }
    }

    let candidates = [folder_vendor, skill_name];
    for cand in candidates {
        if let Some((prefix, _)) = cand.split_once('-') {
            let prefix = prefix.trim();
            if !prefix.is_empty() {
                let base_agent = crate::skills::agents_skills_root().join(prefix);
                let base_cc = crate::skills::skills_root().join(prefix);
                if base_agent.exists() || base_cc.exists() {
                    return prefix.to_string();
                }
            }
        }
    }

    folder_vendor.to_string()
}

pub fn discover_all_skills_meta() -> Vec<SkillMeta> {
    let mut skills = Vec::new();
    let vendors = crate::skills::discover_vendors();

    for vendor in vendors {
        let entries = crate::skills::discover_skills(&vendor);
        for entry in entries {
            let is_flat = entry.name == vendor;
            let family = resolve_family(&entry.skill_md, &entry.name, &vendor, is_flat);
            let meta = load_skill_meta(
                &entry.skill_md,
                &entry.name,
                &family,
                is_flat,
                entry.extra_files.len(),
            );
            skills.push(meta);
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

pub fn filter_skills<'a>(skills: &'a [SkillMeta], query: &str) -> Vec<&'a SkillMeta> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return skills.iter().collect();
    }
    skills
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&q)
                || s.vendor.to_lowercase().contains(&q)
                || s.description.to_lowercase().contains(&q)
        })
        .collect()
}

pub fn format_active_skills_prompt(skills: &[SkillMeta], disabled: &[String]) -> String {
    let enabled: Vec<&SkillMeta> = skills
        .iter()
        .filter(|s| !disabled.iter().any(|d| d == &s.name))
        .collect();

    if enabled.is_empty() {
        return "Apply the appropriate skill if applicable to the task.".to_string();
    }

    let mut out = String::from("Active Agent Skills:\n");
    for s in &enabled {
        out.push_str(&format!("- {}: {}\n", s.name, s.description));
    }
    out.push_str("Apply these active skills when appropriate for the task.");
    out
}

pub fn build_active_skills_prompt(disabled: &[String]) -> String {
    let all = discover_all_skills_meta();
    format_active_skills_prompt(&all, disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_extracts_name_and_single_line_desc() {
        let md = "---\nname: my-skill\ndescription: \"Handles complex API migrations.\"\n---\n# Body";
        assert_eq!(parse_frontmatter_name(md), Some("my-skill".into()));
        assert_eq!(
            parse_frontmatter_description(md),
            Some("Handles complex API migrations.".into())
        );
    }

    #[test]
    fn parse_frontmatter_extracts_multiline_block_desc() {
        let md = "---\nname: craft\ndescription: >\n  Design engineering concepts from Craft\n  covering typography and motion details.\nmetadata:\n  author: test\n---\n";
        assert_eq!(parse_frontmatter_name(md), Some("craft".into()));
        assert_eq!(
            parse_frontmatter_description(md),
            Some("Design engineering concepts from Craft covering typography and motion details.".into())
        );
    }

    #[test]
    fn fallback_extracts_first_prose_line() {
        let md = "# Title\n\nThis is a fallback description for a plain skill.\n\nMore content.";
        assert_eq!(parse_frontmatter_name(md), None);
        assert_eq!(
            parse_frontmatter_description(md),
            Some("This is a fallback description for a plain skill.".into())
        );
    }

    #[test]
    fn filter_skills_matches_query_on_name_or_desc() {
        let s1 = SkillMeta {
            name: "craft-design".into(),
            vendor: "craft".into(),
            description: "Typography and styling".into(),
            path: PathBuf::from("/test1"),
            is_flat: true,
            extra_files_count: 0,
        };
        let s2 = SkillMeta {
            name: "test-driven".into(),
            vendor: "addy".into(),
            description: "Automated test suites".into(),
            path: PathBuf::from("/test2"),
            is_flat: false,
            extra_files_count: 2,
        };
        let list = vec![s1, s2];

        let res_craft = filter_skills(&list, "craft");
        assert_eq!(res_craft.len(), 1);
        assert_eq!(res_craft[0].name, "craft-design");

        let res_test = filter_skills(&list, "test");
        assert_eq!(res_test.len(), 1);
        assert_eq!(res_test[0].name, "test-driven");

        let res_all = filter_skills(&list, "");
        assert_eq!(res_all.len(), 2);
    }

    #[test]
    fn format_active_skills_prompt_includes_only_enabled_skills() {
        let s1 = SkillMeta {
            name: "craft".into(),
            vendor: "craft".into(),
            description: "UI concepts".into(),
            path: PathBuf::from("/test1"),
            is_flat: true,
            extra_files_count: 0,
        };
        let s2 = SkillMeta {
            name: "tdd".into(),
            vendor: "addy".into(),
            description: "Unit testing".into(),
            path: PathBuf::from("/test2"),
            is_flat: false,
            extra_files_count: 0,
        };
        let list = vec![s1, s2];
        let disabled = vec!["tdd".to_string()];
        let prompt = format_active_skills_prompt(&list, &disabled);
        assert!(prompt.contains("Active Agent Skills:"));
        assert!(prompt.contains("- craft: UI concepts"));
        assert!(!prompt.contains("- tdd:"));

        let disabled_all = vec!["craft".to_string(), "tdd".to_string()];
        let fallback = format_active_skills_prompt(&list, &disabled_all);
        assert_eq!(fallback, "Apply the appropriate skill if applicable to the task.");
    }

    #[test]
    fn resolve_family_extracts_from_tracking_or_prefix() {
        let non_flat = resolve_family(Path::new("/dummy/SKILL.md"), "test", "myvendor", false);
        assert_eq!(non_flat, "myvendor");

        let flat_fallback = resolve_family(Path::new("/dummy/SKILL.md"), "custom", "custom", true);
        assert_eq!(flat_fallback, "custom");
    }
}
