use crate::state::AppState;
use crate::ui::modal::model::{Modal, ModalRow, ModalStep};

pub fn open_webhook_modal(state: &mut AppState) {
    let prefs = crate::prefs::Prefs::load();
    let local_path = crate::prefs::Prefs::local_path();
    let session = state
        .panes
        .get(state.active)
        .and_then(|p| p.lock().ok())
        .map(|p| p.state.title.clone())
        .unwrap_or_else(|| "Terminal 1".to_string());
    let workspace = state.active_workspace();
    open_webhook_modal_with_prefs(state, prefs.webhook, local_path, session, workspace);
}

pub fn open_webhook_modal_with_prefs(
    state: &mut AppState,
    wh: crate::prefs::WebhookPrefs,
    persist_path: Option<std::path::PathBuf>,
    session: String,
    workspace: String,
) {
    let mut m = Modal::new("webhook_config", "Status Webhook & Live Broadcast");
    if let Some(path) = persist_path {
        if !path.as_os_str().is_empty() {
            m.persist_config = Some(path);
        }
    }

    let mode_opts = vec![
        ("static".to_string(), "static".to_string(), "Static screen (fixed, no scrolling)".to_string()),
        ("scroll".to_string(), "scroll".to_string(), "Scrolling marquee (continuous animation)".to_string()),
    ];

    let mode_idx = mode_opts
        .iter()
        .position(|(val, _, _)| val.eq_ignore_ascii_case(&wh.mode))
        .unwrap_or(0);

    let format_opts = vec![
        ("form".to_string(), "form".to_string(), "Form Data (ESP32 Matrix: text, color, speed)".to_string()),
        ("json".to_string(), "json".to_string(), "Standard structured JSON (Home Assistant / REST)".to_string()),
    ];

    let format_idx = format_opts
        .iter()
        .position(|(val, _, _)| val.eq_ignore_ascii_case(&wh.format))
        .unwrap_or(0);

    let preview = crate::webhook::build_standby_payload(&wh, &session, &workspace).text;
    let usage_str = crate::usage::load_cached_usage()
        .map(|u| {
            let p = u.five_hour_percent();
            if p > 0.0 {
                format!("{:.0}%", p)
            } else {
                "Active".to_string()
            }
        })
        .unwrap_or_else(|| "Active".to_string());

    let step_general = ModalStep {
        title: "General".into(),
        rows: vec![
            ModalRow::Section {
                title: "Broadcast & Connection".into(),
                color: "cyan".into(),
            },
            ModalRow::Toggle {
                key: "webhook.enabled".into(),
                label: "Enable broadcast".into(),
                enabled: wh.enabled,
            },
            ModalRow::Choice {
                key: "webhook.mode".into(),
                label: "Display mode".into(),
                options: mode_opts,
                current: mode_idx,
                searchable: false,
                color: "cyan".into(),
            },
            ModalRow::TextInput {
                key: "webhook.url".into(),
                label: "Target URL (ESP32 / API)".into(),
                value: wh.url.clone(),
            },
            ModalRow::Choice {
                key: "webhook.format".into(),
                label: "Payload format".into(),
                options: format_opts,
                current: format_idx,
                searchable: false,
                color: "cyan".into(),
            },
            ModalRow::Info("Switch tabs with Tab / Shift+Tab or arrow keys.".into()),
        ],
    };

    let step_display = ModalStep {
        title: "Display".into(),
        rows: vec![
            ModalRow::Section {
                title: "Banner Settings".into(),
                color: "cyan".into(),
            },
            ModalRow::Stepper {
                key: "webhook.speed".into(),
                label: "Scroll speed (ESP32)".into(),
                value: wh.speed as i64,
                min: 5,
                max: 100,
                step: 5,
                unit: "".into(),
            },
            ModalRow::Toggle {
                key: "webhook.include_usage".into(),
                label: "Include metrics & usage rate".into(),
                enabled: wh.include_usage,
            },
            ModalRow::Separator("Preview of dispatched text".into()),
            ModalRow::InfoColored {
                text: format!("❯ {preview}"),
                color: "cyan".into(),
            },
            ModalRow::Info("Dynamic display: CMD, session, workspace, colors, and usage.".into()),
        ],
    };

    let table_rows = vec![
        vec!["Target".into(), wh.url.clone(), if wh.enabled { "Active".into() } else { "Inactive".into() }],
        vec!["Mode".into(), wh.mode.to_uppercase(), "Ready".into()],
        vec!["Format".into(), wh.format.to_uppercase(), "Ready".into()],
        vec!["Speed".into(), format!("{}", wh.speed), "OK".into()],
        vec!["Session".into(), session.clone(), "Online".into()],
        vec!["Workspace".into(), workspace.clone(), "Active".into()],
        vec!["Usage".into(), usage_str, "OK".into()],
    ];

    let step_diag = ModalStep {
        title: "Diagnostics".into(),
        rows: vec![
            ModalRow::Section {
                title: "Status & Immediate Actions".into(),
                color: "cyan".into(),
            },
            ModalRow::Table {
                headers: vec!["Parameter".into(), "Value".into(), "Status".into()],
                rows: table_rows,
                color: "cyan".into(),
            },
            ModalRow::Separator("Actions".into()),
            ModalRow::Nav {
                key: "webhook.send_standby".into(),
                label: "Send standby screen now".into(),
                color: "cyan".into(),
            },
            ModalRow::Nav {
                key: "webhook.test_ping".into(),
                label: "Send test ping".into(),
                color: "green".into(),
            },
        ],
    };

    m.steps = vec![step_general, step_display, step_diag];
    m.current_step = 0;
    m.rows = m.steps[0].rows.clone();
    state.active_modal = Some(m);
    state.dirty = true;
}

pub fn handle_webhook_modal_enter(state: &mut AppState) {
    let Some(modal) = state.active_modal.as_mut() else {
        return;
    };
    let idx = modal.selected;
    let row = match modal.rows.get(idx) {
        Some(r) => r.clone(),
        None => return,
    };

    match row {
        ModalRow::Nav { ref key, .. } => {
            if key == "webhook.send_standby" {
                modal.save();
                let prefs = crate::prefs::Prefs::load();
                let session = state
                    .panes
                    .get(state.active)
                    .and_then(|p| p.lock().ok())
                    .map(|p| p.state.title.clone())
                    .unwrap_or_else(|| "Terminal 1".to_string());
                let workspace = state.active_workspace();
                let payload = crate::webhook::build_standby_payload(&prefs.webhook, &session, &workspace);
                crate::webhook::dispatch_async(prefs.webhook.url, prefs.webhook.format, payload);
            } else if key == "webhook.test_ping" {
                modal.save();
                let prefs = crate::prefs::Prefs::load();
                let payload = crate::webhook::build_test_payload(&prefs.webhook);
                crate::webhook::dispatch_async(prefs.webhook.url, prefs.webhook.format, payload);
            }
        }
        ModalRow::Choice { .. } => {
            modal.cycle_selected();
            modal.save();
        }
        ModalRow::Toggle { ref key, enabled, .. } => {
            modal.cycle_selected();
            modal.save();
            if key == "webhook.enabled" && !enabled {
                let prefs = crate::prefs::Prefs::load();
                let session = state
                    .panes
                    .get(state.active)
                    .and_then(|p| p.lock().ok())
                    .map(|p| p.state.title.clone())
                    .unwrap_or_else(|| "Terminal 1".to_string());
                let workspace = state.active_workspace();
                let payload = crate::webhook::build_standby_payload(&prefs.webhook, &session, &workspace);
                crate::webhook::dispatch_async(prefs.webhook.url, prefs.webhook.format, payload);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_modal_initializes_with_three_steps_and_table() {
        let sidebar = crate::ui::sidebar::Sidebar::load();
        let mut state = AppState::new(sidebar);
        let wh = crate::prefs::WebhookPrefs::default();
        open_webhook_modal_with_prefs(&mut state, wh, None, "Terminal 1".to_string(), "cc-dashboard".to_string());

        let modal = state.active_modal.as_ref().unwrap();
        assert_eq!(modal.id, "webhook_config");
        assert_eq!(modal.steps.len(), 3);
        assert_eq!(modal.steps[0].title, "General");
        assert_eq!(modal.steps[1].title, "Display");
        assert_eq!(modal.steps[2].title, "Diagnostics");

        let diag_rows = &modal.steps[2].rows;
        let has_table = diag_rows.iter().any(|r| matches!(r, ModalRow::Table { .. }));
        assert!(has_table);
    }
}
