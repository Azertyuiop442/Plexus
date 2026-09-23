
use std::path::{Path, PathBuf};

fn meta_path(home: &Path, project: &str, session_id: &str) -> PathBuf {
    home.join(".commandcode")
        .join("projects")
        .join(project)
        .join(format!("{session_id}.meta.json"))
}

fn parse_model(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let model = value.get("model")?.as_str()?.trim();
    if model.is_empty() || model.eq_ignore_ascii_case("unknown") {
        return None;
    }
    Some(model.to_string())
}

pub fn read_session_model_in(home: &Path, project: &str, session_id: &str) -> Option<String> {
    let project = project.trim();
    let session_id = session_id.trim();
    if project.is_empty() || session_id.is_empty() {
        return None;
    }
    let raw = std::fs::read_to_string(meta_path(home, project, session_id)).ok()?;
    parse_model(&raw)
}

pub fn read_session_model(project: &str, session_id: &str) -> Option<String> {
    let home = crate::ipc::home_dir();
    if home.as_os_str().is_empty() {
        return None;
    }
    read_session_model_in(&home, project, session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cc-session-meta-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".commandcode/projects/proj")).unwrap();
        dir
    }

    #[test]
    fn reads_model_from_meta_file() {
        let home = scratch("present");
        std::fs::write(
            meta_path(&home, "proj", "abc"),
            r#"{ "title": "x", "model": "z-ai/glm-5.3-flash" }"#,
        )
        .unwrap();
        assert_eq!(
            read_session_model_in(&home, "proj", "abc").as_deref(),
            Some("z-ai/glm-5.3-flash")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn missing_meta_returns_none() {
        let home = scratch("missing");
        assert_eq!(read_session_model_in(&home, "proj", "abc"), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn blank_and_unknown_models_return_none() {
        let home = scratch("blank");
        std::fs::write(meta_path(&home, "proj", "a"), r#"{ "model": "   " }"#).unwrap();
        std::fs::write(meta_path(&home, "proj", "b"), r#"{ "model": "Unknown" }"#).unwrap();
        std::fs::write(meta_path(&home, "proj", "c"), r#"{ "title": "no model" }"#).unwrap();
        assert_eq!(read_session_model_in(&home, "proj", "a"), None);
        assert_eq!(read_session_model_in(&home, "proj", "b"), None);
        assert_eq!(read_session_model_in(&home, "proj", "c"), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn blank_identifiers_return_none() {
        let home = scratch("ids");
        assert_eq!(read_session_model_in(&home, "", "abc"), None);
        assert_eq!(read_session_model_in(&home, "proj", "  "), None);
        let _ = std::fs::remove_dir_all(&home);
    }
}
