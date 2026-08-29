
use crate::state::AppState;
use crate::ui::modal::model::{Modal, ModalRow};

pub fn open_auto_retry_modal(state: &mut AppState) {
    let prefs = crate::prefs::Prefs::load();
    let local_path = crate::prefs::Prefs::local_path();
    open_auto_retry_modal_with_prefs(state, prefs.auto_retry, local_path);
}

pub fn open_auto_retry_modal_with_prefs(
    state: &mut AppState,
    ar: crate::prefs::AutoRetryPrefs,
    persist_path: Option<std::path::PathBuf>,
) {
    let mut m = Modal::new("auto_retry_config", "Error Recovery & Auto-Retry");
    if let Some(path) = persist_path {
        if !path.as_os_str().is_empty() {
            m.persist_config = Some(path);
        }
    }

    let step1 = vec![
        ModalRow::Separator("Transient Error Targets".into()),
        ModalRow::Info("Select transient errors to auto-retry. Permanent errors (401, 404, 400) are excluded.".into()),
        ModalRow::Toggle {
            key: "auto_retry.retry_rate_limit".into(),
            label: "Rate Limits (429) & Provider Quotas".into(),
            enabled: ar.retry_rate_limit,
        },
        ModalRow::Toggle {
            key: "auto_retry.retry_server_error".into(),
            label: "Server Outages (500, 502, 503, 504)".into(),
            enabled: ar.retry_server_error,
        },
        ModalRow::Toggle {
            key: "auto_retry.retry_network_timeout".into(),
            label: "Network Drops & Timeouts (ETIMEDOUT, ECONNRESET)".into(),
            enabled: ar.retry_network_timeout,
        },
        ModalRow::Toggle {
            key: "auto_retry.retry_stream_drop".into(),
            label: "Stream Interrupted & Premature EOF".into(),
            enabled: ar.retry_stream_drop,
        },
        ModalRow::Toggle {
            key: "auto_retry.retry_tool_failure".into(),
            label: "Tool Execution Failures (Non-Fatal)".into(),
            enabled: ar.retry_tool_failure,
        },
        ModalRow::Info("Press → or TAB for Retry Strategy".into()),
    ];
    m.add_step("1. Target Errors", step1);

    let step2 = vec![
        ModalRow::Separator("Strategy & Exponential Backoff".into()),
        ModalRow::Toggle {
            key: "auto_retry.enabled".into(),
            label: "Enable Auto-Retry System".into(),
            enabled: ar.enabled,
        },
        ModalRow::Stepper {
            key: "auto_retry.max_retries".into(),
            label: "Max Retry Attempts".into(),
            value: ar.max_retries,
            min: 0,
            max: 20,
            step: 1,
            unit: " tries".into(),
        },
        ModalRow::Choice {
            key: "auto_retry.backoff_mode".into(),
            label: "Backoff Algorithm".into(),
            options: vec![
                ("Exponential Backoff (2s, 4s, 8s...)".into(), "exponential".into(), "backoff".into()),
                ("Linear Backoff (2s, 4s, 6s...)".into(), "linear".into(), "backoff".into()),
                ("Immediate (Fast retry)".into(), "immediate".into(), "backoff".into()),
            ],
            current: match ar.backoff_mode.as_str() {
                "linear" => 1,
                "immediate" => 2,
                _ => 0,
            },
            searchable: false,
            color: String::new(),
        },
        ModalRow::Stepper {
            key: "auto_retry.base_delay_secs".into(),
            label: "Base Delay".into(),
            value: ar.base_delay_secs,
            min: 1,
            max: 10,
            step: 1,
            unit: "s".into(),
        },
        ModalRow::Stepper {
            key: "auto_retry.max_delay_secs".into(),
            label: "Max Delay Cap".into(),
            value: ar.max_delay_secs,
            min: 5,
            max: 120,
            step: 5,
            unit: "s".into(),
        },
        ModalRow::Toggle {
            key: "auto_retry.random_jitter".into(),
            label: "Random Jitter (+/- 200ms anti-collision)".into(),
            enabled: ar.random_jitter,
        },
        ModalRow::Info("Press → or TAB for Recovery Actions".into()),
    ];
    m.add_step("2. Retry Strategy", step2);

    let step3 = vec![
        ModalRow::Separator("Recovery Action & Feedback".into()),
        ModalRow::TextInput {
            key: "auto_retry.prompt".into(),
            label: "Recovery Prompt".into(),
            value: ar.prompt,
        },
        ModalRow::Info("Prompt injected automatically into session upon transient error detection.".into()),
        ModalRow::Toggle {
            key: "auto_retry.show_countdown".into(),
            label: "Show Countdown Banner in Session".into(),
            enabled: ar.show_countdown,
        },
        ModalRow::Toggle {
            key: "auto_retry.notify_on_failure".into(),
            label: "Notify when All Retries Exhausted".into(),
            enabled: ar.notify_on_failure,
        },
    ];
    m.add_step("3. Action & Prompt", step3);

    m.commands.push(("save".into(), "Save & Close".into()));
    m.hints.push(("←/→".into(), "Tabs / Adjust".into()));
    m.hints.push(("Space".into(), "Toggle".into()));
    m.hints.push(("Esc".into(), "Dismiss".into()));
    m.select_first_selectable();
    state.active_modal = Some(m);
}
