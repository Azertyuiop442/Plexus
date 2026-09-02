use crate::state::AppState;
use crate::ui::modal::{load_modal, Modal, ModalRow};

pub fn open_context_modal(state: &mut AppState) {
    if let Some(pane) = state.panes.get(state.active) {
        let p_guard = pane.lock().unwrap_or_else(|e| e.into_inner());
        let idx = state.active;
        let title = if p_guard.state.title.is_empty() || p_guard.state.title == "commandcode" {
            format!("Terminal #{}", idx + 1)
        } else {
            p_guard.state.title.clone()
        };

        let cwd_str = p_guard
            .state
            .boot_info
            .as_ref()
            .and_then(|b| b.cwd.clone())
            .unwrap_or_else(|| "~/.commandcode".to_string());

        let model_str = state
            .mods_data
            .active_model()
            .or_else(|| {
                p_guard
                    .state
                    .boot_info
                    .as_ref()
                    .and_then(|b| b.models.clone())
            })
            .unwrap_or_else(|| "command-code".to_string());

        let data_mod = state
            .mods_data
            .mods
            .iter()
            .find(|m| !m.data.turns.is_empty());

        let mut m = Modal::new("context_modal", format!("ⓘ {} - Session Context", title));

        let mut session_rows: Vec<ModalRow> = vec![
            ModalRow::Info(format!("Working Directory: {}", cwd_str)),
            ModalRow::Info(format!("Active Model: {}", model_str)),
            ModalRow::Info(String::new()),
        ];
        let turn_rows: Vec<String> = data_mod
            .map(|cm| {
                cm.data
                    .turns
                    .iter()
                    .rev()
                    .take(12)
                    .map(|t| {
                        let model = t
                            .model
                            .split('/')
                            .next_back()
                            .unwrap_or(&t.model)
                            .to_string();
                        format!(
                            "{}  {} in · {} out · cache hit {}% · {}",
                            model,
                            crate::ui::pane::format_tokens(t.input),
                            crate::ui::pane::format_tokens(t.output),
                            t.cache_hit_pct,
                            fmt_usd(t.cost)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        if turn_rows.is_empty() {
            session_rows.push(ModalRow::Info("No turns recorded yet".into()));
        } else {
            for line in turn_rows {
                session_rows.push(ModalRow::Info(line));
            }
        }
        m.add_step("Current Session", session_rows);

        let mut ws_rows: Vec<ModalRow> = Vec::new();
        let ws_lines: Vec<String> = data_mod
            .map(|cm| {
                cm.data
                    .sections
                    .iter()
                    .find(|s| s.heading == "Workspace")
                    .map(|s| s.lines.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if ws_lines.is_empty() {
            ws_rows.push(ModalRow::Info("No saved history for this workspace".into()));
        } else {
            for line in ws_lines {
                ws_rows.push(ModalRow::Info(format!("• {}", line)));
            }
        }
        m.add_step("Workspace", ws_rows);

        let mut turns_rows: Vec<ModalRow> = Vec::new();
        let turn_lines: Vec<String> = data_mod
            .map(|cm| {
                cm.data
                    .sections
                    .iter()
                    .find(|s| s.heading == "Turns")
                    .map(|s| s.lines.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if turn_lines.is_empty() {
            turns_rows.push(ModalRow::Info("No past turns recorded".into()));
        } else {
            for line in turn_lines {
                turns_rows.push(ModalRow::Info(format!("• {}", line)));
            }
        }
        m.add_step("Turns & Total", turns_rows);

        let total_lines: Vec<String> = data_mod
            .map(|cm| {
                cm.data
                    .sections
                    .iter()
                    .find(|s| s.heading == "Total")
                    .map(|s| s.lines.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        for line in total_lines {
            m.sticky_footer.push(ModalRow::Separator("Total".into()));
            m.sticky_footer.push(ModalRow::Info(format!("◆ {}", line)));
        }

        m.set_page_size(12);

        state.active_modal = Some(m);
    }
}

fn fmt_usd(n: f64) -> String {
    if !n.is_finite() || n <= 0.0 {
        "$0.00".to_string()
    } else if n >= 100.0 {
        format!("${:.0}", n)
    } else {
        format!("${:.2}", n)
    }
}

pub fn open_all_sessions_modal(state: &mut AppState) {
    open_all_sessions_modal_with_msg(state, None);
}

pub fn open_all_sessions_modal_with_msg(state: &mut AppState, feedback: Option<&str>) {
    let mut m = Modal::new("all_sessions", "Manage Sessions");

    let mut session_rows = vec![];
    session_rows.push(ModalRow::Separator("Stored Sessions".into()));
    if state.sidebar.sessions.is_empty() {
        session_rows.push(ModalRow::Info("No stored sessions found.".into()));
    } else {
        for (i, s) in state.sidebar.sessions.iter().enumerate() {
            let open = state.session_is_open(&s.id);
            let raw_title = s.title.lines().next().unwrap_or(&s.title).trim();
            let short_title = crate::ui::sidebar::session_title(raw_title, 30);
            let age = if s.age_short.is_empty() { "now" } else { &s.age_short };
            let status = if open { " [open]" } else { "" };
            let label = format!("#{} {} ({}){}", i + 1, short_title, age, status);
            let options = if open {
                vec![(
                    "Open".into(),
                    format!("open_{}", s.id),
                    "action".into(),
                )]
            } else {
                vec![
                    (
                        "Open".into(),
                        format!("open_{}", s.id),
                        "action".into(),
                    ),
                    (
                        "Delete".into(),
                        format!("del_{}", s.id),
                        "danger".into(),
                    ),
                ]
            };
            session_rows.push(ModalRow::Choice {
                key: format!("sess_{}", s.id),
                label,
                options,
                current: 0,
                searchable: false,
                color: if open { "green".into() } else { String::new() },
            });
        }
    }
    m.add_step("Sessions", session_rows);

    let mut maint_rows = vec![];
    if let Some(msg) = feedback {
        maint_rows.push(ModalRow::InfoColored {
            text: msg.to_string(),
            color: "green".into(),
        });
    }
    maint_rows.push(ModalRow::Separator("Workspace Information".into()));
    maint_rows.push(ModalRow::Info(format!("Project: {}", state.sidebar.project)));
    maint_rows.push(ModalRow::Info(format!("Stored: {} session(s)", state.sidebar.sessions.len())));
    maint_rows.push(ModalRow::Separator("Cleanup Actions".into()));
    maint_rows.push(ModalRow::Choice {
        key: "action_clean_24h".into(),
        label: "Clean sessions older than 24h (> 1 day)".into(),
        options: vec![
            ("Clean > 24h".into(), "execute_clean_24h".into(), "warning".into()),
        ],
        current: 0,
        searchable: false,
        color: "yellow".into(),
    });
    maint_rows.push(ModalRow::Choice {
        key: "action_clean_3d".into(),
        label: "Clean sessions older than 3 days (> 3 days)".into(),
        options: vec![
            ("Clean > 3d".into(), "execute_clean_3d".into(), "warning".into()),
        ],
        current: 0,
        searchable: false,
        color: "yellow".into(),
    });
    maint_rows.push(ModalRow::Choice {
        key: "action_clean_7d".into(),
        label: "Clean sessions older than 7 days (> 7 days)".into(),
        options: vec![
            ("Clean > 7d".into(), "execute_clean_7d".into(), "warning".into()),
        ],
        current: 0,
        searchable: false,
        color: "yellow".into(),
    });
    maint_rows.push(ModalRow::Choice {
        key: "action_clean_all".into(),
        label: "Clean all workspace sessions".into(),
        options: vec![
            ("Clean All".into(), "execute_clean_all".into(), "danger".into()),
        ],
        current: 0,
        searchable: false,
        color: "red".into(),
    });
    m.add_step("Maintenance", maint_rows);

    m.set_page_size(8);
    m.hints.push(("Tab".into(), "Switch Tab".into()));
    m.hints.push(("Enter".into(), "Execute".into()));
    m.hints.push(("d".into(), "Delete".into()));
    m.select_first_selectable();
    state.active_modal = Some(m);
}

pub fn open_mod_config_modal(state: &mut AppState, idx: usize) {
    if let Some(mod_item) = state.sidebar.mods.get(idx) {
        let data_dir = std::path::PathBuf::from(crate::ui::sidebar::data_dir());
        if let Some(m) = load_modal(&data_dir, &mod_item.id).or_else(|| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            load_modal(&data_dir, &mod_item.id)
        }) {
            state.active_modal = Some(m);
            return;
        }

        let mut m = Modal::new(
            &mod_item.id,
            format!("{} Wizard", mod_item.label.clone().unwrap_or_default()),
        );

        m.rows.push(ModalRow::Toggle {
            key: "enabled".into(),
            label: "Master Enable".into(),
            enabled: mod_item.enabled,
        });
        m.rows.push(ModalRow::Info(
            "Press ESC or ENTER to save and close".into(),
        ));

        m.select_first_selectable();
        state.active_modal = Some(m);
    }
}

pub fn open_full_config_modal(state: &mut AppState) {
    let config_path = crate::prefs::Prefs::config_path()
        .unwrap_or_else(std::path::PathBuf::new);
    let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));

    let mut m = Modal::new("cc_global_config", "Command Code Global Config");
    if !config_path.as_os_str().is_empty() {
        m.persist_config = Some(config_path);
    }
    let provider = json
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("command-code")
        .to_string();
    let compact = json
        .get("compactMode")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    let step1 = vec![
        ModalRow::Separator("Authentication Provider".into()),
        ModalRow::Choice {
            key: "provider".into(),
            label: "Auth Provider".into(),
            options: vec![
                (
                    "Command Code (Default)".into(),
                    "command-code".into(),
                    "auth".into(),
                ),
                (
                    "Anthropic [disabled]".into(),
                    "command-code".into(),
                    "auth".into(),
                ),
                (
                    "GitHub Copilot [disabled]".into(),
                    "command-code".into(),
                    "auth".into(),
                ),
                (
                    "Codex [disabled]".into(),
                    "command-code".into(),
                    "auth".into(),
                ),
            ],
            current: match provider.as_str() {
                "anthropic" => 1,
                "github-copilot" => 2,
                "codex" => 3,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Info("Default Model is set per-pane via /model. The full config only manages global preferences.".into()),
        ModalRow::Separator("Execution & Mode".into()),
        ModalRow::Choice {
            key: "compactMode".into(),
            label: "Compact Mode".into(),
            options: vec![
                (
                    "Default Aggressive".into(),
                    "default".into(),
                    "compact".into(),
                ),
                ("Fast Compaction".into(), "fast".into(), "compact".into()),
            ],
            current: if compact == "fast" { 1 } else { 0 },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Choice {
            key: "effort".into(),
            label: "Reasoning Effort".into(),
            options: vec![
                ("Provider Default".into(), "default".into(), "effort".into()),
                ("Low".into(), "low".into(), "effort".into()),
                ("Medium".into(), "medium".into(), "effort".into()),
                ("High".into(), "high".into(), "effort".into()),
                ("Max".into(), "max".into(), "effort".into()),
            ],
            current: match json.get("effort").and_then(|v| v.as_str()).unwrap_or("default") {
                "low" => 1,
                "medium" => 2,
                "high" => 3,
                "max" => 4,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Info("Press → or TAB for Security & Privacy".into()),
    ];
    m.add_step("1. Core & Auth", step1);

    let perm_mode = json
        .get("permissionMode")
        .or_else(|| json.get("permissions").and_then(|p| p.get("defaultMode")))
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    let step2 = vec![
        ModalRow::Separator("Permissions & Execution".into()),
        ModalRow::Choice {
            key: "permissionMode".into(),
            label: "Default Permission Mode".into(),
            options: vec![
                ("Default (Ask Changes)".into(), "default".into(), "perm".into()),
                ("Auto-Accept (Safe Reads)".into(), "auto-accept".into(), "perm".into()),
                ("Plan Mode (Read-Only)".into(), "plan".into(), "perm".into()),
                ("Bypass Mode (Full Auto)".into(), "bypass".into(), "perm".into()),
            ],
            current: match perm_mode.as_str() {
                "auto-accept" => 1,
                "plan" => 2,
                "bypass" => 3,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Toggle {
            key: "disableBypass".into(),
            label: "Disable Bypass (--yolo) Mode".into(),
            enabled: json
                .get("disableBypass")
                .or_else(|| json.get("permissions").and_then(|p| p.get("disableBypass")))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Toggle {
            key: "disableSkillShellExecution".into(),
            label: "Disable Skill Shell Execution (!`cmd`)".into(),
            enabled: json
                .get("disableSkillShellExecution")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Toggle {
            key: "disableScratchpad".into(),
            label: "Disable Session Scratchpad Directory".into(),
            enabled: json
                .get("disableScratchpad")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Separator("Privacy & Data Retention".into()),
        ModalRow::Toggle {
            key: "zeroDataRetention".into(),
            label: "Zero Data Retention (ZDR Mode)".into(),
            enabled: json
                .get("zeroDataRetention")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Toggle {
            key: "forceOAuth".into(),
            label: "Force OAuth Authentication".into(),
            enabled: json
                .get("forceOAuth")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Separator("Context & Integration".into()),
        ModalRow::Toggle {
            key: "collapsePastedText".into(),
            label: "Collapse Pasted Text (>300 chars)".into(),
            enabled: json
                .get("collapsePastedText")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        ModalRow::Toggle {
            key: "tasteLearning".into(),
            label: "Taste Learning".into(),
            enabled: json
                .get("tasteLearning")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        ModalRow::Toggle {
            key: "ideContextEnabled".into(),
            label: "IDE Context Integration".into(),
            enabled: json
                .get("ideContextEnabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        ModalRow::Toggle {
            key: "autoInstallExtension".into(),
            label: "Auto Install IDE Extension".into(),
            enabled: json
                .get("autoInstallExtension")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        ModalRow::Info("Press → or TAB for Export & Advanced".into()),
    ];
    m.add_step("2. Security & Privacy", step2);

    let export_fmt = json
        .get("defaultExportFormat")
        .and_then(|v| v.as_str())
        .unwrap_or("html")
        .to_string();
    let share_gist_fmt = json
        .get("defaultShareGistFormat")
        .and_then(|v| v.as_str())
        .unwrap_or("html")
        .to_string();
    let filter_mode = json
        .get("treeFilterMode")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    let step3 = vec![
        ModalRow::Separator("Export & Share Configuration".into()),
        ModalRow::Choice {
            key: "defaultExportFormat".into(),
            label: "Default Export Format".into(),
            options: vec![
                (
                    "HTML Interactive Document".into(),
                    "html".into(),
                    "export".into(),
                ),
                (
                    "JSONL Standard Trajectory".into(),
                    "jsonl".into(),
                    "export".into(),
                ),
                ("Markdown Plain Report".into(), "md".into(), "export".into()),
            ],
            current: match export_fmt.as_str() {
                "jsonl" => 1,
                "md" => 2,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Choice {
            key: "defaultShareGistFormat".into(),
            label: "Default Share Gist Format".into(),
            options: vec![
                (
                    "HTML Interactive Document".into(),
                    "html".into(),
                    "export".into(),
                ),
                (
                    "JSONL Standard Trajectory".into(),
                    "jsonl".into(),
                    "export".into(),
                ),
                ("Markdown Plain Report".into(), "md".into(), "export".into()),
            ],
            current: match share_gist_fmt.as_str() {
                "jsonl" => 1,
                "md" => 2,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Choice {
            key: "treeFilterMode".into(),
            label: "Tree Filter Mode".into(),
            options: vec![
                ("Default Clean".into(), "default".into(), "filter".into()),
                ("No Tools Output".into(), "no-tools".into(), "filter".into()),
                (
                    "User Input Only".into(),
                    "user-only".into(),
                    "filter".into(),
                ),
                ("All Entries Traced".into(), "all".into(), "filter".into()),
            ],
            current: match filter_mode.as_str() {
                "no-tools" => 1,
                "user-only" => 2,
                "all" => 3,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Separator("Advanced Controls".into()),
        ModalRow::Toggle {
            key: "show_splash".into(),
            label: "Show splash screen on launch".into(),
            enabled: json
                .get("show_splash")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        },
        ModalRow::Toggle {
            key: "onDemandToolDescriptions".into(),
            label: "On-Demand Tool Descriptions".into(),
            enabled: json
                .get("onDemandToolDescriptions")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Toggle {
            key: "branchSummarySkipPrompt".into(),
            label: "Branch Summary Skip Prompt".into(),
            enabled: json
                .get("branchSummarySkipPrompt")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        ModalRow::Info("Press ESC or ENTER to save and close".into()),
    ];
    m.add_step("3. Export & Advanced", step3);

    let feature_models = json
        .get("featureModels")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let model_choices = pricing_model_choices();
    let mut step4 = vec![ModalRow::Separator("Feature Models (per-feature override)".into())];
    for (feature_key, label) in [
        ("branchSummarization", "Branch Summarization"),
        ("compaction", "Compaction"),
        ("tasteLearning", "Taste Learning"),
        ("tasteOnboarding", "Taste Onboarding"),
        ("titleGeneration", "Title Generation"),
        ("toolDescription", "Tool Description"),
        ("vision", "Vision"),
    ] {
        let current = feature_models
            .get(feature_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut row_choices = model_choices.clone();
        let current_idx = if current.is_empty() {
            0
        } else {
            let found = row_choices.iter().position(|(_, id, _)| {
                id == &current
                    || id.eq_ignore_ascii_case(&current)
                    || (!id.is_empty() && base_model_key(id) == base_model_key(&current))
            });
            match found {
                Some(idx) => idx,
                None => {
                    let label = short_model_label(&current);
                    row_choices.push((label, current.clone(), "custom".into()));
                    row_choices.len() - 1
                }
            }
        };
        step4.push(ModalRow::Choice {
            key: format!("featureModels.{feature_key}"),
            label: label.into(),
            options: row_choices,
            current: current_idx,
            searchable: true,
            color: String::new(),
        });
    }
    step4.push(ModalRow::Info(
        "Left/Right on a row = inline cycle; Enter = searchable picker.".into(),
    ));
    m.add_step("4. Feature Models", step4);

    m.set_page_size(8);
    m.select_first_selectable();

    state.active_modal = Some(m);
}

pub fn open_ai_prefs_modal(state: &mut AppState) {
    let config_path = crate::prefs::Prefs::config_path()
        .unwrap_or_else(std::path::PathBuf::new);
    let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));

    let mut m = Modal::new("ai_prefs", "AI Prefs");
    if !config_path.as_os_str().is_empty() {
        m.persist_config = Some(config_path);
    }

    let model_choices = pricing_model_choices();
    let feature_models = json
        .get("featureModels")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut rows = vec![ModalRow::Separator("Model per background feature".into())];
    for (feature_key, label) in [
        ("branchSummarization", "Branch Summarization"),
        ("compaction", "Compaction"),
        ("tasteLearning", "Taste Learning"),
        ("tasteOnboarding", "Taste Onboarding"),
        ("titleGeneration", "Title Generation"),
        ("toolDescription", "Tool Description"),
        ("vision", "Vision"),
    ] {
        let current = feature_models
            .get(feature_key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut row_choices = model_choices.clone();
        let current_idx = if current.is_empty() {
            0
        } else {
            let found = row_choices.iter().position(|(_, id, _)| {
                id == &current
                    || id.eq_ignore_ascii_case(&current)
                    || (!id.is_empty() && base_model_key(id) == base_model_key(&current))
            });
            match found {
                Some(idx) => idx,
                None => {
                    let label = short_model_label(&current);
                    row_choices.push((label, current.clone(), "custom".into()));
                    row_choices.len() - 1
                }
            }
        };
        rows.push(ModalRow::Choice {
            key: format!("featureModels.{feature_key}"),
            label: label.into(),
            options: row_choices,
            current: current_idx,
            searchable: true,
            color: String::new(),
        });
    }

    rows.push(ModalRow::Separator("Feature Toggles".into()));
    rows.push(ModalRow::Toggle {
        key: "imageVisionEnabled".into(),
        label: "Image Vision (Vision Mode)".into(),
        enabled: json
            .get("imageVisionEnabled")
            .or_else(|| json.get("vision"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    });
    rows.push(ModalRow::Toggle {
        key: "tasteLearning".into(),
        label: "Taste Learning".into(),
        enabled: json
            .get("tasteLearning")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    });
    rows.push(ModalRow::Toggle {
        key: "onDemandToolDescriptions".into(),
        label: "On-Demand Tool Descriptions".into(),
        enabled: json
            .get("onDemandToolDescriptions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    });
    rows.push(ModalRow::Toggle {
        key: "branchSummarySkipPrompt".into(),
        label: "Branch Summary Skip Prompt".into(),
        enabled: json
            .get("branchSummarySkipPrompt")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    });
    rows.push(ModalRow::Toggle {
        key: "zeroDataRetention".into(),
        label: "Zero Data Retention (ZDR)".into(),
        enabled: json
            .get("zeroDataRetention")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    });
    rows.push(ModalRow::Toggle {
        key: "collapsePastedText".into(),
        label: "Collapse Pasted Text (>300 chars)".into(),
        enabled: json
            .get("collapsePastedText")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    });

    rows.push(ModalRow::Info(
        "Left/Right = inline cycle; Enter = searchable picker or toggle.".into(),
    ));
    m.add_step("Feature Models", rows);

    m.add_step("Peak Hours", peak_hours_rows());

    m.select_first_selectable();

    state.active_modal = Some(m);
}

fn base_model_key(id: &str) -> String {
    id.trim()
        .to_lowercase()
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim_end_matches("-free")
        .to_string()
}

fn pricing_model_choices() -> Vec<(String, String, String)> {
    let mut out = vec![("Default (curated)".to_string(), String::new(), "default".to_string())];
    let raw = std::fs::read_to_string(
        home_join(".commandcode/mods/cost-tracker-pricing.json"),
    )
    .unwrap_or_default();
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return out;
    };
    let Some(models) = json.get("models").and_then(|v| v.as_array()) else {
        return out;
    };

    struct Entry {
        id: String,
        label: String,
        category: String,
        free: bool,
        order: usize,
    }
    let mut best: std::collections::HashMap<String, Entry> = std::collections::HashMap::new();
    for (free_id, free_label) in [
        ("minimax/minimax-m2.7-free", "MiniMax-M2.7"),
        ("minimax/minimax-m3-free", "MiniMax-M3"),
        ("poolside/laguna-s-2.1-free", "Laguna S-2.1"),
    ] {
        let key = base_model_key(free_id);
        best.insert(
            key,
            Entry {
                id: free_id.to_string(),
                label: free_label.to_string(),
                category: "free".to_string(),
                free: true,
                order: 0,
            },
        );
    }

    for (i, m) in models.iter().enumerate() {
        let Some(id) = m.get("id").and_then(|v| v.as_str()) else { continue };
        if id.is_empty() { continue; }
        let free = id.to_lowercase().ends_with("-free") || id.to_lowercase().contains("free");
        let input_price = m.get("inputPerM").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let is_free_row = free || input_price == 0.0;
        let key = base_model_key(id);
        let raw_cat = m
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("open")
            .to_string();
        let cat = if is_free_row { "free".to_string() } else { raw_cat };
        let entry = Entry {
            id: id.to_string(),
            label: short_model_label(id),
            category: cat,
            free: is_free_row,
            order: i + 10,
        };
        match best.get_mut(&key) {
            None => {
                best.insert(key, entry);
            }
            Some(existing) => {
                if is_free_row && !existing.free {
                    *existing = entry;
                }
            }
        }
    }
    let mut rows: Vec<Entry> = best.into_values().collect();
    rows.sort_by(|a, b| a.order.cmp(&b.order));
    for e in rows {
        let label = if e.free && !e.label.to_lowercase().contains("free") {
            format!("{} · free", e.label)
        } else {
            e.label
        };
        out.push((label, e.id, e.category));
    }
    out
}

fn peak_countdown(windows_utc: &str) -> String {
    let now_secs = current_epoch_secs();
    let now_utc_mins = ((now_secs / 60).rem_euclid(1440)) as i32;
    let mut clean = windows_utc.to_string();
    for sep in ['\u{2013}', '\u{2014}', '\u{2212}'] {
        clean = clean.replace(sep, "-");
    }
    let clean = clean.replace("UTC", "");
    let mut windows: Vec<(i32, i32)> = Vec::new();
    for part in clean.split('&') {
        let seg = part.trim();
        let pieces: Vec<&str> = seg.split('-').collect();
        if pieces.len() != 2 {
            continue;
        }
        let Ok(start_h) = pieces[0].trim().parse::<i32>() else { continue };
        let Ok(mut end_h) = pieces[1].trim().parse::<i32>() else { continue };
        if end_h == 0 {
            end_h = 24;
        }
        windows.push((start_h * 60, end_h * 60));
    }
    if windows.is_empty() {
        return "off-peak".to_string();
    }
    for &(start_m, end_m) in &windows {
        if now_utc_mins >= start_m && now_utc_mins < end_m {
            let remain = end_m - now_utc_mins;
            let rh = remain / 60;
            let rm = remain % 60;
            return if rh > 0 && rm > 0 {
                format!("PEAK NOW (ends in {rh}h {rm:02}m)")
            } else if rh > 0 {
                format!("PEAK NOW (ends in {rh}h)")
            } else {
                format!("PEAK NOW (ends in {rm}m)")
            };
        }
    }
    let mut min_delta = i32::MAX;
    for &(start_m, _) in &windows {
        let delta = if start_m > now_utc_mins {
            start_m - now_utc_mins
        } else {
            start_m + 1440 - now_utc_mins
        };
        if delta < min_delta {
            min_delta = delta;
        }
    }
    if min_delta == i32::MAX {
        return "off-peak".to_string();
    }
    let dh = min_delta / 60;
    let dm = min_delta % 60;
    if dh > 0 && dm > 0 {
        format!("in {dh}h {dm:02}m")
    } else if dh > 0 {
        format!("in {dh}h")
    } else {
        format!("in {dm}m")
    }
}

fn peak_hours_rows() -> Vec<ModalRow> {
    let mut rows = vec![];

    let tz = std::env::var("TZ").unwrap_or_default();
    let tz_name = if tz.is_empty() {
        "local".to_string()
    } else {
        tz
    };
    let now_local = chrono_local_now();
    let offset = local_utc_offset_secs(current_epoch_secs());
    rows.push(ModalRow::InfoColored {
        text: format!(
            "Your timezone: {tz_name} (UTC{:+02}:{:02}) - local time {}",
            offset / 3600,
            (offset % 3600) / 60,
            now_local
        ),
        color: "blue".into(),
    });

    let price_raw = std::fs::read_to_string(
        home_join(".commandcode/mods/cost-tracker-pricing.json"),
    )
    .unwrap_or_default();
    let price_json: serde_json::Value =
        serde_json::from_str(&price_raw).unwrap_or(serde_json::json!({}));

    let alerts = read_peak_alerts();

    let default_peak_models = [
        ("deepseek/deepseek-v4-pro", "00-08 UTC"),
        ("deepseek/deepseek-v4-flash", "00-08 UTC"),
        ("opensource/deepseek-v4-flash-vision-exp", "00-08 UTC"),
    ];
    let mut added_ids = std::collections::HashSet::new();

    if let Some(models) = price_json.get("models").and_then(|v| v.as_array()) {
        for m in models {
            let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let tod = match m.get("timeOfDay") {
                Some(t) if t.is_object() => t,
                _ => continue,
            };
            let windows_utc = tod
                .get("windows")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if windows_utc.is_empty() {
                continue;
            }
            added_ids.insert(id.to_string());
            let short_model = short_model_label(id);
            let countdown = peak_countdown(&windows_utc);
            rows.push(ModalRow::Toggle {
                key: format!("peakAlert.{id}"),
                label: format!("{short_model} - {countdown}"),
                enabled: alerts
                    .get(&format!("peakAlert.{id}"))
                    .copied()
                    .unwrap_or(false),
            });
        }
    }

    for (id, windows_utc) in default_peak_models {
        if !added_ids.contains(id) {
            let short_model = short_model_label(id);
            let countdown = peak_countdown(windows_utc);
            rows.push(ModalRow::Toggle {
                key: format!("peakAlert.{id}"),
                label: format!("{short_model} - {countdown}"),
                enabled: alerts
                    .get(&format!("peakAlert.{id}"))
                    .copied()
                    .unwrap_or(false),
            });
        }
    }

    rows.push(ModalRow::Info(
        "Toggle ON to receive peak hour alerts in the status bar before running a turn."
            .into(),
    ));
    rows
}

fn home_join(rel: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home).join(rel)
}

fn chrono_local_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let offset_secs = local_utc_offset_secs(secs);
    let local = secs + offset_secs;
    let h = (local / 3600).rem_euclid(24);
    let min = (local % 3600) / 60;
    format!("{h:02}:{min:02}")
}

fn local_utc_offset_secs(_epoch: i64) -> i64 {

    let out = std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    parse_tz_offset(&out)
}

fn parse_tz_offset(s: &str) -> i64 {
    let b = s.as_bytes();
    if b.len() != 5 || (b[0] != b'+' && b[0] != b'-') {
        return 0;
    }
    let sign = if b[0] == b'-' { -1i64 } else { 1 };
    let hh = s[1..3].parse::<i64>().unwrap_or(0);
    let mm = s[3..5].parse::<i64>().unwrap_or(0);
    sign * (hh * 3600 + mm * 60)
}

fn current_epoch_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn short_model_label(id: &str) -> String {
    match id.split_once('/') {
        Some((_vendor, base)) => base.to_string(),
        None => id.to_string(),
    }
}

const PEAK_ALERTS_FILE: &str = ".commandcode/mods/cost-tracker-history/peak-alerts.json";

fn read_peak_alerts() -> std::collections::HashMap<String, bool> {
    let raw = std::fs::read_to_string(home_join(PEAK_ALERTS_FILE)).unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .filter_map(|(k, v)| v.as_bool().map(|b| (k, b)))
                .collect()
        })
        .unwrap_or_default()
}

pub fn save_peak_alert(modal_rows: &[ModalRow]) {
    let toggles: Vec<(String, bool)> = modal_rows
        .iter()
        .filter_map(|r| match r {
            ModalRow::Toggle { key, enabled, .. } if key.starts_with("peakAlert.") => {
                Some((key.clone(), *enabled))
            }
            _ => None,
        })
        .collect();
    if toggles.is_empty() {
        return;
    }
    let path = home_join(PEAK_ALERTS_FILE);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut map = read_peak_alerts();
    for (k, v) in toggles {
        map.insert(k, v);
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(
        map.into_iter()
            .map(|(k, v)| (k, serde_json::Value::Bool(v)))
            .collect(),
    ))
    .unwrap_or_default();
    let _ = crate::ipc::atomic_write(&path, &json);
}

pub fn sync_modal_toggles(state: &mut AppState) {
    if let Some(ref mut modal) = state.active_modal {
        let mod_id = modal.id.clone();
        let mut master_enabled = true;
        let mut status_bar = true;
        let mut ctx_btn = true;

        for row in &modal.rows {
            if let ModalRow::Toggle { key, enabled, .. } = row {
                if key == "enabled" {
                    master_enabled = *enabled;
                } else if key == "show_status_bar" {
                    status_bar = *enabled;
                } else if key == "show_context_btn" {
                    ctx_btn = *enabled;
                }
            }
        }

        if let Some(mod_item) = state.sidebar.mods.iter_mut().find(|m| m.id == mod_id) {
            mod_item.enabled = master_enabled;
        }

        if mod_id == "examplemod" || mod_id == "examplemod" || mod_id.contains("cost") {
            state.sidebar.show_cost_bar = status_bar;
            state.sidebar.show_context_btn = ctx_btn;
        }

        if mod_id == "cc_global_config" {

            let mut patch = serde_json::Map::new();
            for step in &modal.steps {
                for row in &step.rows {
                    match row {
                        ModalRow::Toggle { key, enabled, .. } => {
                            patch.insert(key.clone(), serde_json::json!(enabled));
                        }
                        ModalRow::Choice {
                            key,
                            options,
                            current,
                            ..
                        } => {
                            if let Some((_, val, _)) = options.get(*current) {
                                patch.insert(key.clone(), serde_json::json!(val));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if !patch.is_empty() {
                if let Some(cfg_path) = crate::prefs::Prefs::config_path() {
                    let _ = crate::ipc::merge_write_json(&cfg_path, &patch);
                }
            }
        }

        if mod_id == "skills_config" {
            for step in &modal.steps {
                for row in &step.rows {
                    if let ModalRow::TextInput { key, value, .. } = row {
                        if let Some(vendor) = key.strip_prefix("url.") {
                            let trimmed = value.trim();
                            if !trimmed.is_empty() {
                                let _ = crate::skills::attach_url(vendor, trimmed);
                            }
                        }
                    }
                }
            }
            for row in &modal.rows {
                if let ModalRow::TextInput { key, value, .. } = row {
                    if let Some(vendor) = key.strip_prefix("url.") {
                        let trimmed = value.trim();
                        if !trimmed.is_empty() {
                            let _ = crate::skills::attach_url(vendor, trimmed);
                        }
                    }
                }
            }
        }

        if modal.dirty {
            if modal.id == "ai_prefs" {
                save_peak_alert(&modal.all_rows());
            }
            modal.save();
            if modal.id == "auto_retry_config" {
                let prefs = crate::prefs::Prefs::load();
                state.sidebar.auto_retry_enabled = prefs.auto_retry.enabled;
            }
            if modal.id == "sounds_config" {
                let prefs = crate::prefs::Prefs::load();
                state.sidebar.sound_notifications = prefs.sounds.enabled;
            }
        }
    }
}

pub fn open_keybind_help_modal(state: &mut AppState) {
    let mut m = Modal::new("keybind_help", "Keyboard Shortcuts");

    let step1 = vec![
        ModalRow::Separator("Terminal Tabs & Windows".into()),
        ModalRow::Info("  Ctrl+T            New terminal tab".into()),
        ModalRow::Info("  Ctrl+W            Close current tab".into()),
        ModalRow::Info("  Ctrl+Tab          Next terminal tab".into()),
        ModalRow::Info("  Ctrl+Shift+Tab    Previous terminal tab".into()),
        ModalRow::Info("  Ctrl+1..9         Direct jump to tab N".into()),
        ModalRow::Info("  Ctrl+B            Toggle sidebar collapse/expand".into()),
        ModalRow::Info("  Ctrl+N            Terminal Navigator jump list".into()),
        ModalRow::Info(String::new()),
        ModalRow::Info("Press → or TAB for Search & Controls".into()),
    ];
    m.add_step("1. Tabs & Windows", step1);

    let step2 = vec![
        ModalRow::Separator("Search & Navigation Controls".into()),
        ModalRow::Info("  Cmd+K / Ctrl+K    Global Finder (Tabs, Files, Output)".into()),
        ModalRow::Info("  Ctrl+I            Terminal Context & Token Cost Metrics".into()),
        ModalRow::Info("  Ctrl+E            Rename active terminal title".into()),
        ModalRow::Info("  PgUp / PgDn       Scroll terminal buffer up/down".into()),
        ModalRow::Info("  Cmd+P / ?         Open Keyboard Shortcuts help".into()),
        ModalRow::Info(String::new()),
        ModalRow::Info("Press → or TAB for Sidebar & Actions".into()),
    ];
    m.add_step("2. Search & Controls", step2);

    let step3 = vec![
        ModalRow::Separator("Sidebar & Mod Actions".into()),
        ModalRow::Info("  ↑ / ↓  or  j / k  Navigate sidebar rows and items".into()),
        ModalRow::Info("  Tab  or  ← / →    Switch submenus and step tabs".into()),
        ModalRow::Info("  Enter / Space     Open session / toggle / run action".into()),
        ModalRow::Info("  d                 Delete highlighted session".into()),
        ModalRow::Info("  Ctrl+R            Hot reload multiplexer dashboard".into()),
        ModalRow::Info("  Esc               Close modal / back to terminal".into()),
    ];
    m.add_step("3. Sidebar & Actions", step3);
    m.select_first_selectable();

    state.active_modal = Some(m);
}

pub fn open_navigator_modal(state: &mut AppState) {
    let mut m = Modal::new("navigator", "Navigator");
    for (i, pane) in state.panes.iter().enumerate() {
        let p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let title = if p.state.title.is_empty() {
            format!("Terminal {}", i + 1)
        } else {
            p.state.title.clone()
        };
        let state_label = match p.state.agent_state {
            crate::agent_state::AgentState::Working => "working",
            crate::agent_state::AgentState::Blocked => "blocked",
            crate::agent_state::AgentState::Idle => "idle",
        };
        m.rows.push(ModalRow::Choice {
            key: format!("pane_{i}"),
            label: format!("{}. {}", i + 1, title),
            options: vec![
                (
                    format!("Focus · {state_label}"),
                    format!("focus_{i}"),
                    "action".into(),
                ),
                ("Close".into(), format!("close_{i}"), "danger".into()),
            ],
            current: 0,
            searchable: false,
            color: String::new(),
        });
    }
    if state.panes.is_empty() {
        m.rows.push(ModalRow::Info("No panes".into()));
    }
    m.hints.push(("↑↓".into(), "Navigate".into()));
    m.hints.push(("Enter".into(), "Focus".into()));
    m.hints.push(("d".into(), "Close pane".into()));
    m.select_first_selectable();
    state.active_modal = Some(m);
}

pub fn open_list_modal(
    state: &mut AppState,
    mod_id: &str,
    modal: &crate::ui::mod_bridge::ModModal,
) {
    let title = if modal.title.is_empty() {
        format!("{} - Select", mod_id)
    } else {
        modal.title.clone()
    };
    let mut m = Modal::new(format!("list_{}", mod_id), title);

    if let Some(msg) = &modal.confirm {
        m.rows.push(ModalRow::Info(format!("⚠ {}", msg)));
        m.rows.push(ModalRow::Info(String::new()));
        m.hints.push(("⚠".into(), "Destructive".into()));
    }

    if !modal.readonly {
        m.pickup_command = modal.pickup_command.clone().unwrap_or_default();
    }

    if let Some(prog) = &modal.progress {
        m.rows.push(ModalRow::Progress {
            label: prog.label.clone(),
            current: prog.current,
            total: prog.total.max(1),
        });
        m.hints.push(("Esc".into(), "Close".into()));
        m.select_first_selectable();
        state.active_modal = Some(m);
        return;
    }
    for item in &modal.items {
        let label = if item.detail.is_empty() {
            item.label.clone()
        } else {
            format!("{}  {}", item.label, item.detail)
        };
        if modal.readonly {

            m.rows.push(ModalRow::Info(label));
        } else {
            m.rows.push(ModalRow::Choice {
                key: format!("item_{}", m.run_files.len()),
                label,
                options: vec![(
                    String::new(),
                    format!("sel_{}", m.run_files.len()),
                    "action".into(),
                )],
                current: 0,
                searchable: false,
                color: item.color.clone(),
            });
        }
        m.run_files.push(item.value.clone());
    }
    if modal.items.is_empty() {
        m.rows.push(ModalRow::Info("No items".into()));
    }

    if !modal.readonly {
        m.hints.push(("↑↓".into(), "Navigate".into()));
        m.hints.push(("Enter".into(), "Select".into()));
    } else {
        m.hints.push(("Esc".into(), "Close".into()));
    }
    for a in &modal.actions {
        m.hints.push((a.key.clone(), a.label.clone()));
    }
    m.select_first_selectable();
    state.active_modal = Some(m);
}

pub fn open_update_progress_modal(state: &mut AppState, label: &str, current: usize, total: usize) {
    let mut m = Modal::new("update_progress", "Updating Plexus");
    m.rows.push(ModalRow::Separator("Installation in Progress".into()));
    m.rows.push(ModalRow::Progress {
        label: label.to_string(),
        current,
        total: total.max(1),
    });
    m.rows.push(ModalRow::Info("Downloading, compiling and installing in background...".into()));
    m.rows.push(ModalRow::Info("Log: /tmp/plexus-update.log".into()));
    m.hints.push(("Auto-Reload".into(), "Plexus will restart once complete".into()));
    m.select_first_selectable();
    state.active_modal = Some(m);
}

pub fn open_update_modal(state: &mut AppState, status: &str) {
    let mut m = Modal::new("update_status", "Plexus Update");
    let mut rows = vec![];
    rows.push(ModalRow::Separator("Update Status".into()));
    rows.push(ModalRow::InfoColored {
        text: status.to_string(),
        color: "green".into(),
    });
    rows.push(ModalRow::Info("The update is building and installing in background.".into()));
    rows.push(ModalRow::Info("Log file: /tmp/plexus-update.log".into()));
    rows.push(ModalRow::Info("You can continue using Plexus normally.".into()));
    m.rows = rows;
    m.hints.push(("Esc / Enter".into(), "Dismiss".into()));
    m.select_first_selectable();
    state.active_modal = Some(m);
}


