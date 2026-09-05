use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::prefs::WebhookPrefs;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub state: String,
    pub label: String,
    pub cmd: String,
    pub app: String,
    pub workspace: String,
    pub session: String,
    pub clock: String,
    pub usage_bar: String,
    pub usage: String,
    pub mode: String,
    pub text: String,
    pub banner: String,
    pub color: String,
    pub speed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_pct: Option<f64>,
    pub time: String,
}

static LAST_PAYLOAD_HASH: AtomicU64 = AtomicU64::new(0);
static LAST_SENT_TIME: Mutex<Option<Instant>> = Mutex::new(None);

fn format_static_text(session: &str, time: &str, usage_bar: &str, usage_str: &str, state_kind: &str) -> String {
    let session_display = match state_kind {
        "running" => format!("▶ {session}"),
        "waiting_for_input" | "waiting" => format!("⚠ {session}"),
        "exited" | "error" => format!("✖ {session}"),
        _ => session.to_string(),
    };
    format!("Command\n{session_display}\n{time}  {usage_bar} {usage_str}")
}

fn format_tokens_display(tok: u64) -> String {
    if tok >= 1_000_000_000 {
        let v = tok as f64 / 1_000_000_000.0;
        let s = format!("{:.1}", v);
        if s.ends_with(".0") {
            format!("{:.0}b", v)
        } else {
            format!("{s}b").replace('.', ",")
        }
    } else if tok >= 1_000_000 {
        let v = tok as f64 / 1_000_000.0;
        let s = format!("{:.1}", v);
        if s.ends_with(".0") {
            format!("{:.0}m", v)
        } else {
            format!("{s}m").replace('.', ",")
        }
    } else if tok >= 1_000 {
        let v = tok as f64 / 1_000.0;
        let s = format!("{:.1}", v);
        if s.ends_with(".0") {
            format!("{:.0}k", v)
        } else {
            format!("{s}k").replace('.', ",")
        }
    } else {
        format!("{tok}")
    }
}

pub fn build_standby_payload(prefs: &WebhookPrefs, session: &str, workspace: &str) -> WebhookPayload {
    let now = current_time_str();
    let usage = crate::usage::load_cached_usage();
    let usage_pct = usage.as_ref().map(|u| u.five_hour_percent());
    let usage_str = if let Some(pct) = usage_pct {
        format!("{:.0}%", pct)
    } else {
        "0%".to_string()
    };
    let pct_val = usage_pct.unwrap_or(0.0);
    let (filled, empty) = crate::usage::build_ascii_bar(pct_val, 6);
    let usage_bar = format!("{}{}", filled, empty);

    let is_static = prefs.mode == "static";
    let speed = if is_static { 0 } else { prefs.speed };

    let text = if is_static {
        format_static_text(session, &now, &usage_bar, &usage_str, "standby")
    } else {
        let mut parts = vec![
            "Command [PAUSE]".to_string(),
            now.clone(),
            session.to_string(),
        ];
        if prefs.include_usage && usage_pct.is_some() {
            parts.push(format!("Usage: {usage_str}"));
        }
        parts.push("Standby".to_string());
        parts.join(" · ")
    };

    let banner = format_ascii_banner("PAUSE", &now, session, usage_pct);

    WebhookPayload {
        event: "standby".to_string(),
        state: "standby".to_string(),
        label: "PAUSE".to_string(),
        cmd: "Command".to_string(),
        app: "Command".to_string(),
        workspace: workspace.to_string(),
        session: session.to_string(),
        clock: now.clone(),
        usage_bar,
        usage: usage_str,
        mode: prefs.mode.clone(),
        text,
        banner,
        color: "#4385BE".to_string(),
        speed,
        cost: None,
        tokens: None,
        turns: None,
        usage_pct,
        time: now,
    }
}

pub fn build_test_payload(prefs: &WebhookPrefs) -> WebhookPayload {
    let now = current_time_str();
    let banner = format_ascii_banner("TEST", &now, "Ping", None);
    let usage_bar = "██░░░░".to_string();
    WebhookPayload {
        event: "test".to_string(),
        state: "test".to_string(),
        label: "TEST".to_string(),
        cmd: "Command".to_string(),
        app: "Command".to_string(),
        workspace: "Test".to_string(),
        session: "Terminal 1".to_string(),
        clock: now.clone(),
        usage_bar: usage_bar.clone(),
        usage: "33%".to_string(),
        mode: prefs.mode.clone(),
        text: format!("Command\nTerminal 1\n{now}  {usage_bar} 33%"),
        banner,
        color: "#4385BE".to_string(),
        speed: if prefs.mode == "static" { 0 } else { prefs.speed },
        cost: Some(0.0),
        tokens: Some(0),
        turns: Some(0),
        usage_pct: Some(33.0),
        time: now,
    }
}

pub fn build_status_payload(
    prefs: &WebhookPrefs,
    state_kind: &str,
    session: &str,
    workspace: &str,
    cost: Option<f64>,
    tokens: Option<u64>,
    turns: Option<u64>,
) -> WebhookPayload {
    let now = current_time_str();
    let usage = crate::usage::load_cached_usage();
    let usage_pct = usage.as_ref().map(|u| u.five_hour_percent());
    let usage_str = if let Some(pct) = usage_pct {
        format!("{:.0}%", pct)
    } else if let Some(c) = cost {
        format!("${:.2}", c)
    } else {
        "0%".to_string()
    };
    let pct_val = usage_pct.unwrap_or(0.0);
    let (filled, empty) = crate::usage::build_ascii_bar(pct_val, 6);
    let usage_bar = format!("{}{}", filled, empty);

    let (label, icon, color) = match state_kind {
        "running" => ("WORKING", "▶", "#879A39"),
        "waiting_for_input" | "waiting" => ("ACTION NEEDED", "⚠", "#D0A215"),
        "exited" | "error" => ("STOPPED", "✖", "#D14D41"),
        _ => ("PAUSE", "●", "#4385BE"),
    };

    let is_static = prefs.mode == "static";
    let speed = if is_static { 0 } else { prefs.speed };

    let text = if is_static {
        format_static_text(session, &now, &usage_bar, &usage_str, state_kind)
    } else {
        let mut parts = vec![
            format!("{icon} Command [{label}]"),
            now.clone(),
            session.to_string(),
        ];
        if prefs.include_usage {
            if usage_pct.is_some() {
                parts.push(format!("Usage: {usage_str}"));
            } else if let Some(c) = cost {
                if c > 0.0 {
                    parts.push(format!("${:.2}", c));
                }
            }
        }
        if state_kind == "idle" || state_kind == "standby" {
            parts.push("Standby".to_string());
        }
        parts.join(" · ")
    };

    let banner = format_ascii_banner(label, &now, session, usage_pct);

    WebhookPayload {
        event: "status".to_string(),
        state: state_kind.to_string(),
        label: label.to_string(),
        cmd: "Command".to_string(),
        app: "Command".to_string(),
        workspace: workspace.to_string(),
        session: session.to_string(),
        clock: now.clone(),
        usage_bar,
        usage: usage_str,
        mode: prefs.mode.clone(),
        text,
        banner,
        color: color.to_string(),
        speed,
        cost: if prefs.include_usage { cost } else { None },
        tokens: if prefs.include_usage { tokens } else { None },
        turns: if prefs.include_usage { turns } else { None },
        usage_pct: if prefs.include_usage { usage_pct } else { None },
        time: now,
    }
}

fn format_ascii_banner(label: &str, time: &str, session: &str, usage_pct: Option<f64>) -> String {
    let usage_str = usage_pct
        .map(|p| format!(" · Usage: {:.0}%", p))
        .unwrap_or_default();
    format!(" _  _  _ \n|_)|_)/   \n|  | \\ \\_ \n[{label}] {time} · {session}{usage_str}")
}

fn current_time_str() -> String {
    let output = Command::new("date").arg("+%H:%M").output();
    if let Ok(out) = output {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    "00:00".to_string()
}

fn payload_hash(p: &WebhookPayload) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    p.state.hash(&mut hasher);
    p.text.hash(&mut hasher);
    p.color.hash(&mut hasher);
    p.workspace.hash(&mut hasher);
    p.session.hash(&mut hasher);
    p.clock.hash(&mut hasher);
    p.usage_bar.hash(&mut hasher);
    p.mode.hash(&mut hasher);
    hasher.finish()
}

pub fn dispatch_if_changed(prefs: &WebhookPrefs, payload: WebhookPayload) {
    if !prefs.enabled || prefs.url.is_empty() {
        return;
    }

    let hash = payload_hash(&payload);
    let prev_hash = LAST_PAYLOAD_HASH.load(Ordering::SeqCst);

    let should_send = if hash != prev_hash {
        true
    } else {
        let guard = LAST_SENT_TIME.lock().unwrap_or_else(|e| e.into_inner());
        match *guard {
            Some(t) => t.elapsed().as_secs() >= 30,
            None => true,
        }
    };

    if !should_send {
        return;
    }

    LAST_PAYLOAD_HASH.store(hash, Ordering::SeqCst);
    {
        let mut guard = LAST_SENT_TIME.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(Instant::now());
    }

    dispatch_async(prefs.url.clone(), prefs.format.clone(), payload);
}

pub fn dispatch_async(url: String, format: String, payload: WebhookPayload) {
    std::thread::spawn(move || {
        send_http_post(&url, &format, &payload);
    });
}

fn send_http_post(url: &str, format: &str, payload: &WebhookPayload) {
    if url.is_empty() {
        return;
    }

    let is_form = format.eq_ignore_ascii_case("form");
    let mut cmd = Command::new("curl");
    cmd.args([
        "-s",
        "-S",
        "--connect-timeout",
        "3",
        "--max-time",
        "5",
        "-X",
        "POST",
    ]);

    if is_form {
        cmd.args([
            "--data-urlencode",
            &format!("text={}", payload.text),
            "--data-urlencode",
            &format!("color={}", payload.color),
            "--data-urlencode",
            &format!("speed={}", payload.speed),
            "--data-urlencode",
            &format!("cmd={}", payload.cmd),
            "--data-urlencode",
            &format!("app={}", payload.app),
            "--data-urlencode",
            &format!("workspace={}", payload.workspace),
            "--data-urlencode",
            &format!("session={}", payload.session),
            "--data-urlencode",
            &format!("clock={}", payload.clock),
            "--data-urlencode",
            &format!("time={}", payload.time),
            "--data-urlencode",
            &format!("usage_bar={}", payload.usage_bar),
            "--data-urlencode",
            &format!("usage={}", payload.usage),
            "--data-urlencode",
            &format!("mode={}", payload.mode),
            "--data-urlencode",
            &format!("scroll={}", if payload.mode == "static" { "0" } else { "1" }),
            "--data-urlencode",
            &format!("usage_pct={}", payload.usage_pct.unwrap_or(0.0) as u32),
            "--data-urlencode",
            &format!("state={}", payload.state),
        ]);
        if let Some(c) = payload.cost {
            cmd.args(["--data-urlencode", &format!("cost=${:.2}", c)]);
        }
        if let Some(tr) = payload.turns {
            cmd.args(["--data-urlencode", &format!("turns={}t", tr)]);
        }
        if let Some(tok) = payload.tokens {
            let tok_str = format_tokens_display(tok);
            cmd.args(["--data-urlencode", &format!("tokens={tok_str}")]);
        }
        if url.contains("matrix.local") {
            cmd.args(["--resolve", "matrix.local:80:192.168.1.31"]);
        }
        cmd.arg(url);
    } else {
        let json_data = serde_json::to_string(payload).unwrap_or_default();
        cmd.args(["-H", "Content-Type: application/json"]);
        cmd.args(["-d", &json_data]);
        if url.contains("matrix.local") {
            cmd.args(["--resolve", "matrix.local:80:192.168.1.31"]);
        }
        cmd.arg(url);
    }

    let res = cmd.output();
    let log_path = "/tmp/cc-sidebar/webhook.log";
    if let Ok(out) = res {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let _ = std::fs::write(
            log_path,
            format!("status: {}\nstdout: {}\nstderr: {}\nurl: {}\npayload: {}\n", out.status, stdout, stderr, url, payload.text),
        );
    } else if let Err(e) = res {
        let _ = std::fs::write(log_path, format!("error: {}\n", e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standby_payload_has_blue_color_and_pause_label() {
        let prefs = WebhookPrefs::default();
        let payload = build_standby_payload(&prefs, "Terminal 1", "cc-dashboard");
        assert_eq!(payload.event, "standby");
        assert_eq!(payload.state, "standby");
        assert_eq!(payload.cmd, "Command");
        assert_eq!(payload.app, "Command");
        assert_eq!(payload.workspace, "cc-dashboard");
        assert_eq!(payload.session, "Terminal 1");
        assert_eq!(payload.color, "#4385BE");
        assert_eq!(payload.speed, 0);
        assert!(payload.text.contains("Command"));
        assert!(payload.text.contains("Terminal 1"));
    }

    #[test]
    fn running_status_payload_has_green_color_and_cost() {
        let prefs = WebhookPrefs::default();
        let payload = build_status_payload(&prefs, "running", "Terminal 1", "cc-dashboard", Some(0.42), Some(14000), Some(12));
        assert_eq!(payload.state, "running");
        assert_eq!(payload.color, "#879A39");
        assert_eq!(payload.cmd, "Command");
        assert_eq!(payload.app, "Command");
        assert_eq!(payload.workspace, "cc-dashboard");
        assert_eq!(payload.session, "Terminal 1");
        assert_eq!(payload.speed, 0);
        assert_eq!(payload.cost, Some(0.42));
    }

    #[test]
    fn waiting_status_payload_has_yellow_color() {
        let prefs = WebhookPrefs::default();
        let payload = build_status_payload(&prefs, "waiting_for_input", "Terminal 1", "cc-dashboard", None, None, None);
        assert_eq!(payload.state, "waiting_for_input");
        assert_eq!(payload.color, "#D0A215");
        assert_eq!(payload.cmd, "Command");
        assert_eq!(payload.session, "Terminal 1");
    }

    #[test]
    fn json_serialization_matches_spec() {
        let prefs = WebhookPrefs::default();
        let payload = build_standby_payload(&prefs, "Terminal 1", "cc-dashboard");
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"event\":\"standby\""));
        assert!(json.contains("\"color\":\"#4385BE\""));
        assert!(json.contains("\"session\":\"Terminal 1\""));
        assert!(json.contains("\"workspace\":\"cc-dashboard\""));
        assert!(json.contains("\"cmd\":\"Command\""));
        assert!(json.contains("\"app\":\"Command\""));
        assert!(json.contains("\"clock\":"));
        assert!(json.contains("\"usage_bar\":"));
        assert!(json.contains("\"speed\":0"));
    }

    #[test]
    fn format_tokens_display_rules() {
        assert_eq!(format_tokens_display(0), "0");
        assert_eq!(format_tokens_display(100), "100");
        assert_eq!(format_tokens_display(999), "999");
        assert_eq!(format_tokens_display(1000), "1k");
        assert_eq!(format_tokens_display(1200), "1,2k");
        assert_eq!(format_tokens_display(55200), "55,2k");
        assert_eq!(format_tokens_display(256300), "256,3k");
        assert_eq!(format_tokens_display(1000000), "1m");
        assert_eq!(format_tokens_display(1500000), "1,5m");
        assert_eq!(format_tokens_display(1200000000), "1,2b");
        assert_eq!(format_tokens_display(10000000000), "10b");
    }
}
