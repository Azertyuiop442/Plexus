use crate::skills::{self, VendorStatus};
use crate::state::AppState;
use crate::ui::modal::model::{Modal, ModalRow};

const MODAL_ID: &str = "skills_config";

pub fn open_skills_modal(state: &mut AppState) {
    let (preserved_step, preserved_selected, preserved_page) = if is_skills_modal(state) {
        if let Some(modal) = state.active_modal.as_ref() {
            (modal.current_step, modal.selected, modal.page)
        } else {
            (0, 0, 0)
        }
    } else {
        (0, 0, 0)
    };
    let mut m = Modal::new(MODAL_ID, "Skills");
    m.set_page_size(10);
    if let Some(path) = first_tracking_path() {
        if !path.as_os_str().is_empty() {
            m.persist_config = Some(path);
        }
    }

    m.hints.push(("Space/Enter".into(), "Toggle".into()));
    m.hints.push(("v".into(), "Branch".into()));
    m.hints.push(("a/n".into(), "All/None".into()));
    m.hints.push(("i".into(), "Info".into()));

    add_browse_step(&mut m, &state.skills_view);
    add_tracker_step(&mut m, &state.skills_view);
    add_sources_step(&mut m, &state.skills_view);
    add_install_step(&mut m, &state.skills_view);

    m.set_step(preserved_step);
    if preserved_selected < m.rows.len() && m.rows[preserved_selected].is_selectable() {
        m.page = preserved_page.min(m.page_count().saturating_sub(1));
        m.selected = preserved_selected;
    } else {
        m.select_first_selectable();
    }
    state.active_modal = Some(m);
}

pub fn open_skills_modal_fresh(state: &mut AppState) {
    state.skills_view.path.clear();
    state.skills_view.filter_query.clear();
    state.skills_view.show_detail = None;
    state.skills_view.selected_file = None;
    state.skills_view.selected_vendor = None;
    state.skills_view.selected_family = None;
    state.skills_view.pending_delete = None;
    skills::check_all_background(state.events.clone());
    open_skills_modal(state);
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
    let all_skills = crate::skills_meta::discover_all_skills_meta();
    let prefs = crate::prefs::Prefs::load();
    let total = all_skills.len();
    let active_count = all_skills
        .iter()
        .filter(|s| prefs.skills.is_skill_enabled(&s.name))
        .count();

    if let Some(detail_name) = &view.show_detail {
        let back_label = if let Some(fam) = &view.selected_family {
            format!("‹ Back to {fam} Skills")
        } else {
            "‹ Back to Skills List".into()
        };
        rows.push(ModalRow::Nav {
            key: "skill.detail_back".into(),
            label: back_label,
            color: "accent".into(),
        });
        if let Some(skill) = all_skills.iter().find(|s| &s.name == detail_name) {
            let is_enabled = prefs.skills.is_skill_enabled(&skill.name);
            let status_msg = if is_enabled {
                "✓ Status: ACTIVE (Injected in Agent System Prompt)"
            } else {
                "✗ Status: DISABLED (Excluded from Agent Prompt)"
            };
            let status_color = if is_enabled { "green" } else { "red" };
            rows.push(ModalRow::InfoColored {
                text: status_msg.into(),
                color: status_color.into(),
            });
            rows.push(ModalRow::Toggle {
                key: format!("skill.toggle.{}", skill.name),
                label: "Injection Status".into(),
                enabled: is_enabled,
            });
            rows.push(ModalRow::Separator(format!("Metadata: {}", skill.name)));
            rows.push(ModalRow::Info(format!(
                "Vendor: {}   ·   Extras: {} files",
                skill.vendor, skill.extra_files_count
            )));
            rows.push(ModalRow::Info(format!("Path: {}", skill.path.display())));
            rows.push(ModalRow::Separator("Description".into()));
            for chunk in wrap_text(&skill.description, 60) {
                rows.push(ModalRow::Info(format!("  {chunk}")));
            }
            rows.push(ModalRow::Separator("Danger Zone".into()));
            if view.pending_delete.as_deref() == Some(&skill.name) {
                rows.push(ModalRow::InfoColored {
                    text: format!("⚠ Confirm permanent deletion of '{}'?", skill.name),
                    color: "red".into(),
                });
                rows.push(ModalRow::Nav {
                    key: format!("skill.delete.execute.{}", skill.name),
                    label: "✓ Yes, Delete Skill Permanently".into(),
                    color: "red".into(),
                });
                rows.push(ModalRow::Nav {
                    key: "skill.delete.cancel".into(),
                    label: "✗ Cancel".into(),
                    color: "accent".into(),
                });
            } else {
                rows.push(ModalRow::Nav {
                    key: format!("skill.delete.request.{}", skill.name),
                    label: "✗ Uninstall / Delete Skill".into(),
                    color: "red".into(),
                });
            }
        }
    } else if let Some(family) = &view.selected_family {
        rows.push(ModalRow::Nav {
            key: "family.back".into(),
            label: "‹ Back to Skill Families".into(),
            color: "accent".into(),
        });
        let fam_skills: Vec<&crate::skills_meta::SkillMeta> =
            all_skills.iter().filter(|s| &s.vendor == family).collect();
        let fam_active = fam_skills
            .iter()
            .filter(|s| prefs.skills.is_skill_enabled(&s.name))
            .count();
        let fam_total = fam_skills.len();
        rows.push(ModalRow::InfoColored {
            text: format!("Family: {family}  [{fam_active}/{fam_total} active]"),
            color: "accent".into(),
        });
        rows.push(ModalRow::Nav {
            key: format!("family.enable_all.{family}"),
            label: "✓ Enable All Skills in Family".into(),
            color: "green".into(),
        });
        rows.push(ModalRow::Nav {
            key: format!("family.disable_all.{family}"),
            label: "✗ Disable All Skills in Family".into(),
            color: "overlay0".into(),
        });
        rows.push(ModalRow::Separator(
            "Family Skills (Space to toggle, 'i' for details)".into(),
        ));
        for skill in fam_skills {
            let enabled = prefs.skills.is_skill_enabled(&skill.name);
            let label = format_skill_row(&skill.name, &skill.vendor, &skill.description);
            rows.push(ModalRow::Toggle {
                key: format!("skill.toggle.{}", skill.name),
                label,
                enabled,
            });
        }
    } else if !view.filter_query.trim().is_empty() {
        rows.push(ModalRow::Info(format!(
            "Active: {active_count}/{total} skills"
        )));
        rows.push(ModalRow::TextInput {
            key: "skills.filter".into(),
            label: "Filter Skills".into(),
            value: view.filter_query.clone(),
        });
        rows.push(ModalRow::Nav {
            key: "skills.clear_filter".into(),
            label: "✗ Clear Search Filter".into(),
            color: "overlay1".into(),
        });
        rows.push(ModalRow::Separator("Search Results".into()));
        let filtered = crate::skills_meta::filter_skills(&all_skills, &view.filter_query);
        if filtered.is_empty() {
            rows.push(ModalRow::Info(format!(
                "No skills matching \"{}\"",
                view.filter_query
            )));
        } else {
            for skill in filtered {
                let enabled = prefs.skills.is_skill_enabled(&skill.name);
                let label = format_skill_row(&skill.name, &skill.vendor, &skill.description);
                rows.push(ModalRow::Toggle {
                    key: format!("skill.toggle.{}", skill.name),
                    label,
                    enabled,
                });
            }
        }
    } else {
        let mut families: Vec<String> = all_skills.iter().map(|s| s.vendor.clone()).collect();
        families.sort();
        families.dedup();
        rows.push(ModalRow::Info(format!(
            "Active: {active_count}/{total} skills across {} families",
            families.len()
        )));

        rows.push(ModalRow::TextInput {
            key: "skills.filter".into(),
            label: "Filter Skills".into(),
            value: view.filter_query.clone(),
        });

        rows.push(ModalRow::Nav {
            key: "skills.enable_all".into(),
            label: "✓ Enable All Skills".into(),
            color: "green".into(),
        });
        rows.push(ModalRow::Nav {
            key: "skills.disable_all".into(),
            label: "✗ Disable All Skills".into(),
            color: "overlay0".into(),
        });

        rows.push(ModalRow::Separator(
            "Skill Families (Toggle family or open to manage individual skills)".into(),
        ));

        for family in &families {
            let fam_skills: Vec<&crate::skills_meta::SkillMeta> =
                all_skills.iter().filter(|s| &s.vendor == family).collect();
            let fam_active = fam_skills
                .iter()
                .filter(|s| prefs.skills.is_skill_enabled(&s.name))
                .count();
            let fam_total = fam_skills.len();
            rows.push(ModalRow::Toggle {
                key: format!("family.toggle.{family}"),
                label: format!("{family:<22} [{fam_active}/{fam_total} active]"),
                enabled: fam_active > 0,
            });
            rows.push(ModalRow::Nav {
                key: format!("family.open.{family}"),
                label: format!("  ↳ Manage {family} skills ({fam_total})"),
                color: "accent".into(),
            });
        }
    }

    m.add_step("1. Skills", rows);
}

fn format_skill_row(name: &str, vendor: &str, desc: &str) -> String {
    let name_col = truncate(name, 22);
    let vendor_pill = format!("[{vendor}]");
    let desc_short = truncate(desc, 34);
    format!("{name_col:<22} {vendor_pill:<14} {desc_short}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn wrap_text(s: &str, max_len: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= max_len {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn add_tracker_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    let statuses = skills::vendor_statuses();
    let total = skills::discover_vendors().len();
    let tracked = statuses.iter().filter(|s| s.has_repo).count();
    let behind: usize = statuses.iter().filter(|s| s.is_stale()).count();
    let errors = statuses
        .iter()
        .filter(|s| s.last_error.is_some() && s.has_repo)
        .count();
    rows.push(ModalRow::Info(format!(
        "{tracked}/{total} tracked · {behind} outdated · {errors} errors"
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
        if behind > 0 {
            rows.push(ModalRow::Nav {
                key: "update_all".into(),
                label: format!(
                    "Update All Behind ({behind} vendor{})",
                    if behind == 1 { "" } else { "s" }
                ),
                color: "accent".into(),
            });
        } else {
            rows.push(ModalRow::Info(
                "✓ All tracked skill bundles are up to date".into(),
            ));
        }
        rows.push(ModalRow::Nav {
            key: "skills.cli_update".into(),
            label: "Check & Update via skills CLI (npx skills update)".into(),
            color: "blue".into(),
        });
    }
    if behind > 0 {
        rows.push(ModalRow::Separator("Outdated vendors".into()));
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
    m.add_step("2. Track", rows);
}

fn add_sources_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    let statuses = skills::vendor_statuses();
    let status_by_name: std::collections::HashMap<String, VendorStatus> =
        statuses.into_iter().map(|s| (s.name.clone(), s)).collect();

    if let Some(vendor) = &view.selected_vendor {
        rows.push(ModalRow::Nav {
            key: "source.back".into(),
            label: "‹ Back to Sources List".into(),
            color: "accent".into(),
        });
        let status = status_by_name
            .get(vendor)
            .cloned()
            .unwrap_or_else(|| vendor_status_for(vendor));
        let current = status.url.clone().unwrap_or_default();
        rows.push(ModalRow::InfoColored {
            text: format!("Vendor: {vendor}  [{}]", vendor_status_short(&status)),
            color: "accent".into(),
        });
        if status.has_repo {
            rows.push(ModalRow::Info(format!(
                "Branch: {}  ·  Behind: {}",
                status.branch, status.behind
            )));
        }
        rows.push(ModalRow::TextInput {
            key: format!("url.{vendor}"),
            label: "Git Remote URL".into(),
            value: current,
        });
        rows.push(ModalRow::Nav {
            key: format!("source.save.{vendor}"),
            label: "Save Remote URL".into(),
            color: "green".into(),
        });
        if status.has_repo {
            rows.push(ModalRow::Nav {
                key: format!("source.update.{vendor}"),
                label: "Pull / Update Vendor".into(),
                color: "accent".into(),
            });
        }
    } else {
        let vendors = skills::discover_vendors();
        rows.push(ModalRow::Info(format!(
            "{} vendor source{} (select to inspect or configure repository URL):",
            vendors.len(),
            if vendors.len() == 1 { "" } else { "s" }
        )));
        for vendor in vendors {
            let status = status_by_name
                .get(&vendor)
                .cloned()
                .unwrap_or_else(|| vendor_status_for(&vendor));
            let status_txt = vendor_status_short(&status);
            let label = format!("{vendor:<24} [{status_txt}]");
            rows.push(ModalRow::Nav {
                key: format!("source.open.{vendor}"),
                label,
                color: "text".into(),
            });
        }
        if skills::discover_vendors().is_empty() {
            rows.push(ModalRow::Info(
                "No vendor folders found in ~/.commandcode/skills/".into(),
            ));
        }
    }
    m.add_step("3. Sources", rows);
}

fn add_install_step(m: &mut Modal, view: &crate::state::SkillsView) {
    let mut rows: Vec<ModalRow> = Vec::new();
    if let Some(msg) = &view.last_update_summary {
        let color = if msg.starts_with('✓') { "green" } else { "red" };
        rows.push(ModalRow::InfoColored {
            text: msg.clone(),
            color: color.to_string(),
        });
    }
    rows.push(ModalRow::TextInput {
        key: "install.url".into(),
        label: "Commande npx ou repo GitHub".into(),
        value: String::new(),
    });
    rows.push(ModalRow::TextInput {
        key: "install.vendor".into(),
        label: "Vendor Name (Optional)".into(),
        value: String::new(),
    });
    rows.push(ModalRow::Nav {
        key: "install.action".into(),
        label: "Download & Install Skills".into(),
        color: "accent".into(),
    });
    m.add_step("4. Add", rows);
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

pub fn sync_skills_toggles(rows: &[ModalRow]) {
    let mut prefs = crate::prefs::Prefs::load();
    let mut changed = false;
    for row in rows {
        if let ModalRow::Toggle { key, enabled, .. } = row {
            if let Some(skill_name) = key.strip_prefix("skill.toggle.") {
                let is_enabled = prefs.skills.is_skill_enabled(skill_name);
                if *enabled != is_enabled {
                    if *enabled {
                        prefs.skills.disabled_skills.retain(|s| s != skill_name);
                    } else if !prefs.skills.disabled_skills.contains(&skill_name.to_string()) {
                        prefs.skills.disabled_skills.push(skill_name.to_string());
                    }
                    changed = true;
                }
            } else if let Some(family) = key.strip_prefix("family.toggle.") {
                let all = crate::skills_meta::discover_all_skills_meta();
                let fam_skills: Vec<String> = all
                    .iter()
                    .filter(|s| s.vendor == family)
                    .map(|s| s.name.clone())
                    .collect();
                if *enabled {
                    for s in &fam_skills {
                        prefs.skills.disabled_skills.retain(|d| d != s);
                    }
                } else {
                    for s in &fam_skills {
                        if !prefs.skills.disabled_skills.contains(s) {
                            prefs.skills.disabled_skills.push(s.clone());
                        }
                    }
                }
                changed = true;
            }
        }
    }
    if changed {
        prefs.save();
    }
}

pub fn handle_skills_browse(state: &mut AppState, key: &str) {
    if key == "skill.detail_back" {
        state.skills_view.show_detail = None;
        open_skills_modal(state);
        return;
    }
    if key == "family.back" {
        state.skills_view.selected_family = None;
        open_skills_modal(state);
        return;
    }
    if let Some(family) = key.strip_prefix("family.open.") {
        state.skills_view.selected_family = Some(family.to_string());
        open_skills_modal(state);
        return;
    }
    if let Some(family) = key.strip_prefix("family.toggle.") {
        let all_skills = crate::skills_meta::discover_all_skills_meta();
        let fam_skills: Vec<String> = all_skills
            .iter()
            .filter(|s| s.vendor == family)
            .map(|s| s.name.clone())
            .collect();
        let mut prefs = crate::prefs::Prefs::load();
        prefs.skills.toggle_vendor(&fam_skills);
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if let Some(family) = key.strip_prefix("family.enable_all.") {
        let mut prefs = crate::prefs::Prefs::load();
        let all_skills = crate::skills_meta::discover_all_skills_meta();
        for s in all_skills.iter().filter(|s| s.vendor == family) {
            prefs.skills.disabled_skills.retain(|d| d != &s.name);
        }
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if let Some(family) = key.strip_prefix("family.disable_all.") {
        let mut prefs = crate::prefs::Prefs::load();
        let all_skills = crate::skills_meta::discover_all_skills_meta();
        for s in all_skills.iter().filter(|s| s.vendor == family) {
            if !prefs.skills.disabled_skills.contains(&s.name) {
                prefs.skills.disabled_skills.push(s.name.clone());
            }
        }
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if key == "skills.enable_all" {
        let mut prefs = crate::prefs::Prefs::load();
        prefs.skills.enable_all();
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if key == "skills.disable_all" {
        let all: Vec<String> = crate::skills_meta::discover_all_skills_meta()
            .into_iter()
            .map(|s| s.name)
            .collect();
        let mut prefs = crate::prefs::Prefs::load();
        prefs.skills.disable_all(&all);
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if key == "skills.clear_filter" {
        state.skills_view.filter_query.clear();
        open_skills_modal(state);
        return;
    }
    if key == "skills.filter" {
        if let Some(modal) = state.active_modal.as_mut() {
            if modal.editing_text {
                modal.editing_text = false;
                let idx = modal.selected.min(modal.rows.len().saturating_sub(1));
                if let Some(ModalRow::TextInput { value, .. }) = modal.rows.get(idx) {
                    state.skills_view.filter_query = value.clone();
                }
                open_skills_modal(state);
                return;
            } else {
                modal.editing_text = true;
                return;
            }
        }
    }
    if let Some(name) = key.strip_prefix("skill.detail.") {
        state.skills_view.show_detail = Some(name.to_string());
        open_skills_modal(state);
        return;
    }
    if let Some(name) = key.strip_prefix("skill.toggle.") {
        let mut prefs = crate::prefs::Prefs::load();
        prefs.skills.toggle_skill(name);
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if let Some(vendor) = key.strip_prefix("vendor.toggle.") {
        let all_skills = crate::skills_meta::discover_all_skills_meta();
        let vendor_skills: Vec<String> = all_skills
            .iter()
            .filter(|s| s.vendor == vendor)
            .map(|s| s.name.clone())
            .collect();
        let mut prefs = crate::prefs::Prefs::load();
        prefs.skills.toggle_vendor(&vendor_skills);
        prefs.save();
        open_skills_modal(state);
        return;
    }
    if let Some(rest) = key.strip_prefix("skill.open.") {
        let mut parts = rest.splitn(2, '.');
        let _vendor = parts.next().unwrap_or("");
        let skill = parts.next().unwrap_or("");
        if !skill.is_empty() {
            state.skills_view.show_detail = Some(skill.to_string());
            open_skills_modal(state);
        }
        return;
    }
    if key == "skill.delete.cancel" {
        state.skills_view.pending_delete = None;
        open_skills_modal(state);
        return;
    }
    if let Some(name) = key.strip_prefix("skill.delete.request.") {
        state.skills_view.pending_delete = Some(name.to_string());
        open_skills_modal(state);
        return;
    }
    if let Some(name) = key.strip_prefix("skill.delete.execute.") {
        let res = crate::skills_cli::uninstall_skill(name);
        state.skills_view.pending_delete = None;
        state.skills_view.show_detail = None;
        state.skills_view.last_update_summary = Some(match res {
            Ok(msg) => msg,
            Err(err) => format!("✗ {err}"),
        });
        open_skills_modal(state);
        return;
    }
    if let Some(rest) = key.strip_prefix("file.open.") {
        let mut parts = rest.splitn(3, '.');
        let _vendor = parts.next().unwrap_or("");
        let _skill = parts.next().unwrap_or("");
        let file = parts.next().unwrap_or("").to_string();
        state.skills_view.selected_file = Some(file);
        crate::mux_core::input::write_pickup_and_close(state);
        return;
    }
}

pub fn handle_skills_back(state: &mut AppState) {
    let step = current_step(state);
    if step == 0 && state.skills_view.pending_delete.is_some() {
        state.skills_view.pending_delete = None;
        open_skills_modal(state);
        return;
    }
    if step == 0 && state.skills_view.show_detail.is_some() {
        state.skills_view.show_detail = None;
        open_skills_modal(state);
        return;
    }
    if step == 0 && state.skills_view.selected_family.is_some() {
        state.skills_view.selected_family = None;
        open_skills_modal(state);
        return;
    }
    if step == 2 && state.skills_view.selected_vendor.is_some() {
        state.skills_view.selected_vendor = None;
        open_skills_modal(state);
        return;
    }
    if step == 0 && !state.skills_view.path.is_empty() {
        state.skills_view.path.pop();
        state.skills_view.selected_file = None;
        open_skills_modal(state);
        return;
    }
    if step > 0 {
        if let Some(modal) = state.active_modal.as_mut() {
            modal.set_step(step - 1);
        }
        return;
    }
    state.active_modal = None;
}

pub fn handle_skills_vendor_toggle_shortcut(state: &mut AppState) {
    let Some(modal) = state.active_modal.as_ref() else {
        return;
    };
    if modal.id != MODAL_ID || modal.current_step != 0 {
        return;
    }
    let Some(row) = modal.rows.get(modal.selected) else {
        return;
    };
    let target = match row {
        ModalRow::Toggle { key, .. } => key
            .strip_prefix("family.toggle.")
            .or_else(|| key.strip_prefix("skill.toggle.")),
        ModalRow::Nav { key, .. } => key
            .strip_prefix("family.open.")
            .or_else(|| key.strip_prefix("skill.detail.")),
        _ => None,
    };
    let Some(target) = target else {
        return;
    };
    let all = crate::skills_meta::discover_all_skills_meta();
    let vendor_name = if all.iter().any(|s| s.vendor == target) {
        target.to_string()
    } else if let Some(skill) = all.iter().find(|s| s.name == target) {
        skill.vendor.clone()
    } else {
        return;
    };
    let vendor_skills: Vec<String> = all
        .iter()
        .filter(|s| s.vendor == vendor_name)
        .map(|s| s.name.clone())
        .collect();
    let mut prefs = crate::prefs::Prefs::load();
    prefs.skills.toggle_vendor(&vendor_skills);
    prefs.save();
    open_skills_modal(state);
}

pub fn handle_skills_info_shortcut(state: &mut AppState) {
    let Some(modal) = state.active_modal.as_ref() else {
        return;
    };
    if modal.id != MODAL_ID || modal.current_step != 0 {
        return;
    }
    let Some(row) = modal.rows.get(modal.selected) else {
        return;
    };
    if let Some(skill_name) = match row {
        ModalRow::Toggle { key, .. } => key.strip_prefix("skill.toggle."),
        ModalRow::Nav { key, .. } => key.strip_prefix("skill.detail."),
        _ => None,
    } {
        state.skills_view.show_detail = Some(skill_name.to_string());
        open_skills_modal(state);
        return;
    }
    if let Some(family) = match row {
        ModalRow::Toggle { key, .. } => key.strip_prefix("family.toggle."),
        ModalRow::Nav { key, .. } => key.strip_prefix("family.open."),
        _ => None,
    } {
        state.skills_view.selected_family = Some(family.to_string());
        open_skills_modal(state);
    }
}

pub fn handle_skills_all_shortcut(state: &mut AppState, enable: bool) {
    let Some(modal) = state.active_modal.as_ref() else {
        return;
    };
    if modal.id != MODAL_ID || modal.current_step != 0 {
        return;
    }
    let mut prefs = crate::prefs::Prefs::load();
    if enable {
        prefs.skills.enable_all();
    } else {
        let all: Vec<String> = crate::skills_meta::discover_all_skills_meta()
            .into_iter()
            .map(|s| s.name)
            .collect();
        prefs.skills.disable_all(&all);
    }
    prefs.save();
    open_skills_modal(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_skill_row_truncates_and_aligns() {
        let row = format_skill_row("very-long-skill-name-that-exceeds-boundary", "vendor", "A quick description");
        assert!(row.contains("[vendor]"));
        assert!(row.contains("A quick description"));
    }

    #[test]
    fn wrap_text_splits_at_word_boundaries() {
        let text = "This is a clean test paragraph that should be wrapped cleanly across multiple lines.";
        let lines = wrap_text(text, 25);
        assert!(lines.len() >= 3);
        for line in &lines {
            assert!(line.len() <= 35);
        }
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn sync_skills_toggles_updates_disabled_skills() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut p = crate::prefs::Prefs::default();
        p.skills.disabled_skills.clear();
        p.save();

        let rows = vec![
            ModalRow::Toggle {
                key: "skill.toggle.craft".into(),
                label: "craft".into(),
                enabled: false,
            },
            ModalRow::Toggle {
                key: "skill.toggle.tdd".into(),
                label: "tdd".into(),
                enabled: true,
            },
        ];
        sync_skills_toggles(&rows);

        let loaded = crate::prefs::Prefs::load();
        assert!(!loaded.skills.is_skill_enabled("craft"));
        assert!(loaded.skills.is_skill_enabled("tdd"));
    }
}
