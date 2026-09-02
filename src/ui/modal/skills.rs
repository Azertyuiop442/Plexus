use crate::skills::{self, SkillEntry, VendorStatus};
use crate::state::AppState;
use crate::ui::modal::model::{Modal, ModalRow};

const MODAL_ID: &str = "skills_config";

pub fn open_skills_modal(state: &mut AppState) {
    state.skills_view.path.clear();
    state.skills_view.selected_file = None;
    let mut m = Modal::new(MODAL_ID, "Skills");
    if let Some(path) = first_tracking_path() {
        if !path.as_os_str().is_empty() {
            m.persist_config = Some(path);
        }
    }

    add_browse_step(&mut m, &state.skills_view);
    add_tracker_step(&mut m, &state.skills_view);
    add_sources_step(&mut m);
    add_install_step(&mut m, &state.skills_view);

    m.commands.push(("back".into(), "Back".into()));
    m.commands.push(("refresh".into(), "Refresh".into()));
    m.commands.push(("close".into(), "Close".into()));
    m.hints.push(("Enter".into(), "Drill / Run".into()));
    m.hints.push(("Tab".into(), "Next Tab".into()));
    m.hints.push(("Esc".into(), "Dismiss".into()));
    m.select_first_selectable();
    skills::check_all_background(state.events.clone());
    state.active_modal = Some(m);
}

fn first_tracking_path() -> Option<std::path::PathBuf> {
    let first = skills::discover_vendors().into_iter().next()?;
    Some(skills::tracking_path_public(&first))
}

fn vendor_status_for(name: &str) -> VendorStatus {
    skills::vendor_statuses()
        .into_iter()
        .find(|v| v.name == name)
        .unwrap_or_else(|| VendorStatus {
            name: name.to_string(),
            url: None,
            has_repo: false,
            local_sha: String::new(),
            remote_sha: String::new(),
            behind: 0,
            branch: String::new(),
            last_error: None,
            last_check: 0,
            last_update: 0,
        })
}

fn add_browse_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    if view.path.is_empty() {
        rows.push(ModalRow::Info(
            "Vendors: each vendor folder is a git-tracked bundle of skills.".into(),
        ));
        let statuses = skills::vendor_statuses();
        let status_by_name: std::collections::HashMap<String, VendorStatus> =
            statuses.into_iter().map(|s| (s.name.clone(), s)).collect();
        for vendor in skills::discover_vendors() {
            let count = skills::discover_skills(&vendor).len();
            let status = status_by_name
                .get(&vendor)
                .cloned()
                .unwrap_or_else(|| vendor_status_for(&vendor));
            rows.push(ModalRow::Nav {
                key: format!("vendor.open.{vendor}"),
                label: vendor_browse_label(&vendor, count, &status),
                color: vendor_status_color(&status).to_string(),
            });
        }
        if rows.len() == 1 {
            rows.push(ModalRow::Info(
                "No vendor folders found. Create one under ~/.commandcode/skills/<vendor>/<skill>/SKILL.md".into(),
            ));
        }
    } else if view.path.len() == 1 {
        let vendor = &view.path[0];
        let status = vendor_status_for(vendor);
        rows.push(ModalRow::Info(format!("Vendor: {vendor}  -  {}", vendor_status_short(&status))));
        for skill in skills::discover_skills(vendor) {
            rows.push(ModalRow::Nav {
                key: format!("skill.open.{vendor}.{}", skill.name),
                label: skill_browse_label(&skill),
                color: "text".to_string(),
            });
        }
    } else if view.path.len() == 2 {
        let vendor = &view.path[0];
        let skill_name = &view.path[1];
        let skills = skills::discover_skills(vendor);
        if let Some(skill) = skills.into_iter().find(|s| &s.name == skill_name) {
            let preview = skills::read_skill_md(&skill, 20);
            rows.push(ModalRow::Info(format!(
                "Skill: {vendor}/{skill_name}  -  {} files",
                skill.extra_files.len()
            )));
            if !preview.is_empty() {
                rows.push(ModalRow::Separator("SKILL.md preview".into()));
                for line in preview.lines() {
                    rows.push(ModalRow::Info(line.to_string()));
                }
            }
            if !skill.extra_files.is_empty() {
                rows.push(ModalRow::Separator("Extra files".into()));
                for f in &skill.extra_files {
                    let name = f
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .to_string();
                    rows.push(ModalRow::Nav {
                        key: format!("file.open.{vendor}.{skill_name}.{name}"),
                        label: format!("{name}"),
                        color: "text".to_string(),
                    });
                }
            }
        }
    }
    m.add_step("1. Browse", rows);
}

fn add_tracker_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    let statuses = skills::vendor_statuses();
    let total = skills::discover_vendors().len();
    let tracked = statuses.iter().filter(|s| s.has_repo).count();
    let behind: usize = statuses.iter().filter(|s| s.is_stale()).count();
    let untracked = total.saturating_sub(tracked);
    let errors = statuses
        .iter()
        .filter(|s| s.last_error.is_some() && s.has_repo)
        .count();
    rows.push(ModalRow::Info(format!(
        "Total: {total}  Tracked: {tracked}  Behind: {behind}  Untracked: {untracked}  Errors: {errors}"
    )));
    if let Some(summary) = &view.last_update_summary {
        rows.push(ModalRow::Info(format!("Last update: {summary}")));
    }
    if let Some(upd) = &view.updating {
        rows.push(ModalRow::Progress {
            label: format!("Updating {}", upd.current),
            current: upd.done,
            total: upd.total.max(1),
        });
        if let Some(last) = &upd.last_result {
            rows.push(ModalRow::Info(format!("  {last}")));
        }
    } else {
        rows.push(ModalRow::Toggle {
            key: "update_all".into(),
            label: format!("Update All Behind  ({behind} vendor{})", if behind == 1 { "" } else { "s" }),
            enabled: behind == 0,
        });
    }
    if behind > 0 {
        rows.push(ModalRow::Separator("Stale vendors".into()));
        for s in statuses.iter().filter(|s| s.is_stale()) {
            rows.push(ModalRow::Info(format!(
                "  {}  (+{} behind)",
                s.name, s.behind
            )));
        }
    }
    if errors > 0 {
        rows.push(ModalRow::Separator("Errors".into()));
        for s in statuses.iter().filter(|s| s.last_error.is_some()) {
            rows.push(ModalRow::Info(format!(
                "  {}  {}",
                s.name,
                s.last_error.as_deref().unwrap_or("?")
            )));
        }
    }
    m.add_step("2. Tracker", rows);
}

fn add_sources_step(m: &mut Modal) {
    let mut rows: Vec<ModalRow> = Vec::new();
    rows.push(ModalRow::Info(
        "Paste a git repository URL per vendor to track and check for updates.".into(),
    ));
    rows.push(ModalRow::Info(
        "Example: https://github.com/addyosmani/agent-skills.git or user/repo".into(),
    ));
    let statuses = skills::vendor_statuses();
    let status_by_name: std::collections::HashMap<String, VendorStatus> =
        statuses.into_iter().map(|s| (s.name.clone(), s)).collect();
    for vendor in skills::discover_vendors() {
        let status = status_by_name
            .get(&vendor)
            .cloned()
            .unwrap_or_else(|| vendor_status_for(&vendor));
        let current = status.url.clone().unwrap_or_default();
        rows.push(ModalRow::Info(format!(
            "  {}  -  {}",
            vendor,
            vendor_status_short(&status)
        )));
        rows.push(ModalRow::TextInput {
            key: format!("url.{vendor}"),
            label: format!("{vendor} URL"),
            value: current,
        });
    }
    if skills::discover_vendors().is_empty() {
        rows.push(ModalRow::Info(
            "No vendor folders yet. Create ~/.commandcode/skills/<vendor>/<skill>/SKILL.md first.".into(),
        ));
    }
    m.add_step("3. Sources", rows);
}

fn add_install_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    rows.push(ModalRow::Info(
        "Download & install a skill bundle directly from any GitHub repository.".into(),
    ));
    rows.push(ModalRow::Info(
        "Example: https://github.com/addyosmani/agent-skills.git or user/repo".into(),
    ));
    if let Some(msg) = &view.last_update_summary {
        let color = if msg.starts_with('✓') { "green" } else { "red" };
        rows.push(ModalRow::InfoColored {
            text: msg.clone(),
            color: color.to_string(),
        });
    }
    rows.push(ModalRow::TextInput {
        key: "install.url".into(),
        label: "Repository URL".into(),
        value: String::new(),
    });
    rows.push(ModalRow::TextInput {
        key: "install.vendor".into(),
        label: "Vendor Name (Optional)".into(),
        value: String::new(),
    });
    rows.push(ModalRow::Toggle {
        key: "install.action".into(),
        label: "> Download & Install Skills".into(),
        enabled: false,
    });
    m.add_step("4. Install", rows);
}

fn vendor_browse_label(vendor: &str, count: usize, status: &VendorStatus) -> String {
    let badge = vendor_status_short(status);
    let plural = if count == 1 { "skill" } else { "skills" };
    format!("{vendor}  ({count} {plural})  -  {badge}")
}

fn skill_browse_label(skill: &SkillEntry) -> String {
    let n = skill.extra_files.len();
    let extra = if n == 0 {
        "no extras".to_string()
    } else if n == 1 {
        "1 file".to_string()
    } else {
        format!("{n} files")
    };
    format!("{}  -  {}", skill.name, extra)
}

fn vendor_status_color(status: &VendorStatus) -> &'static str {
    if status.last_error.is_some() && status.has_repo {
        return "red";
    }
    if !status.has_repo {
        return "muted";
    }
    if status.is_stale() {
        return "red";
    }
    "green"
}

fn vendor_status_short(status: &VendorStatus) -> String {
    if status.last_error.is_some() && status.has_repo {
        return format!("error: {}", status.last_error.as_deref().unwrap_or("?"));
    }
    if !status.has_repo {
        return "untracked".to_string();
    }
    if status.is_stale() {
        return format!("outdated  (+{} behind)", status.behind);
    }
    "up to date".to_string()
}

pub fn is_skills_modal(state: &AppState) -> bool {
    state
        .active_modal
        .as_ref()
        .map(|m| m.id == MODAL_ID)
        .unwrap_or(false)
}

pub fn current_step(state: &AppState) -> usize {
    state
        .active_modal
        .as_ref()
        .map(|m| m.current_step)
        .unwrap_or(0)
}
