
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config as TermConfig, Term};

pub use super::pane_tty::{
    is_substantive_output, should_scroll_to_bottom, to_ratatui_color, tty_of_pid, TermSize,
};
pub use super::pane_render::{format_tokens, render_pane};
pub use crate::ui::pane_state::BootInfo;

pub fn poll_metrics(_pane: &mut MuxPane) {}

pub struct MuxPane {

    pub term: Term<VoidListener>,
    pub parser: alacritty_terminal::vte::ansi::Processor,
    pub writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>>,
    input_tx: std::sync::mpsc::SyncSender<Vec<u8>>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    #[allow(dead_code)]
    pub child: Box<dyn portable_pty::Child + Send + Sync>,

    pub child_pid: u32,

    pub state: crate::ui::pane_state::PaneState,
}

const BUSY_CHECK_MS: u64 = 500;

const AGENT_CHECK_MS: u64 = 500;

const LOADING_STABLE_MS: u64 = 800;

const AGENT_STATUS_MAX_AGE_MS: u64 = 30_000;

pub(crate) const SCROLL_SNAP_GRACE_MS: u64 = 2000;

impl MuxPane {

    pub fn prompt_position(&self) -> Option<(i32, usize)> {
        let content = self.term.renderable_content();
        let mut line_buf = String::new();
        let mut line_y = 0i32;
        let mut last_y = None;
        let mut found: Option<(i32, usize)> = None;
        let mut best_score = -1i32;
        let is_prompt_line = |buf: &str| {
            let lower = buf.to_lowercase();
            let trimmed = buf.trim_start();
            lower.contains("ask your question")
                || lower.contains("? for shortcuts")
                || lower.contains("what would you like to do")
                || lower.contains("type \"continue\" to try again")
                || lower.contains("type 'continue' to try again")
                || trimmed.starts_with('❯')
                || trimmed.starts_with('>')
                || trimmed.starts_with('›')
                || trimmed.starts_with('$')
                || trimmed.starts_with('%')
        };
        let score_of = |buf: &str| -> i32 {
            let lower = buf.to_lowercase();
            let trimmed = buf.trim_start();
            if lower.contains("ask your question") {
                3
            } else if trimmed.starts_with('❯') || trimmed.starts_with('>') || trimmed.starts_with('›') {
                2
            } else {
                1
            }
        };
        for item in content.display_iter {
            let y = item.point.line.0;
            if last_y != Some(y) {
                if is_prompt_line(&line_buf) {
                    let score = score_of(&line_buf);
                    if score >= best_score {

                        best_score = score;
                        found = Some((line_y, line_buf.chars().count()));
                    }
                }
                line_buf.clear();
                line_y = y;
                last_y = Some(y);
            }
            line_buf.push(item.cell.c);
        }
        if is_prompt_line(&line_buf) {
            let score = score_of(&line_buf);
            if score >= best_score {
                found = Some((line_y, line_buf.chars().count()));
            }
        }
        found
    }

    pub fn is_at_prompt(&self) -> bool {
        self.prompt_position().is_some()
    }

    pub fn is_busy(&mut self) -> bool {
        if self.state.exited {
            return false;
        }

        if self.state.last_busy_check.elapsed() < std::time::Duration::from_millis(BUSY_CHECK_MS) {
            return self.state.busy_cache;
        }
        self.state.last_busy_check = std::time::Instant::now();
        let busy = if self.is_at_prompt() {
            false
        } else {
            self.state.last_activity.elapsed() < std::time::Duration::from_millis(300)
        };
        self.state.busy_cache = busy;
        busy
    }

    pub fn spinner_frame(&self) -> &'static str {

        const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        FRAMES[(millis / 120 % 4) as usize]
    }

    pub fn spawn(
        command: &str,
        cols: u16,
        rows: u16,
    ) -> io::Result<(Arc<Mutex<Self>>, Box<dyn Read + Send>)> {
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows: rows as u16,
                cols: cols as u16,
                ..Default::default()
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut cmd = portable_pty::CommandBuilder::new("sh");
        cmd.args(["-lc", command]);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let mut tty_name = String::new();
        let tty_pid = child.process_id().unwrap_or(0);
        if tty_pid != 0 {
            crate::orphan_journal::register(tty_pid, command);
            for _ in 0..3 {
                if let Some(t) = tty_of_pid(tty_pid) {
                    tty_name = t;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let master = pair.master;

        let (input_tx, input_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(256);
        {
            let writer = Arc::new(std::sync::Mutex::new(writer));
            let w = writer.clone();
            std::thread::spawn(move || {
                for bytes in input_rx {
                    let mut guard = w.lock().unwrap_or_else(|e| e.into_inner());
                    for chunk in bytes.chunks(16 * 1024) {
                        let _ = guard.write_all(chunk);
                    }
                    let _ = guard.flush();
                }
            });
            let size = TermSize {
                cols: cols as usize,
                rows: rows as usize,
            };
            let term = Term::new(TermConfig::default(), &size, VoidListener);
            let pane = Arc::new(Mutex::new(Self {
                term,
                parser: alacritty_terminal::vte::ansi::Processor::new(),
                writer,
                input_tx,
                master,
                child,
                child_pid: tty_pid,
                state: {
                    let mut st = crate::ui::pane_state::PaneState::new(command.to_string());
                    st.tty_name = tty_name;
                    st
                },
            }));

            Ok((pane, reader))
        }
    }

    pub fn bottom_text(&self, max_lines: usize) -> String {
        let content = self.term.renderable_content();
        let mut lines: Vec<String> = Vec::new();
        let mut line_buf = String::new();
        let mut last_y = None;
        for item in content.display_iter {
            let y = item.point.line.0;
            if last_y != Some(y) {
                if last_y.is_some() {
                    lines.push(std::mem::take(&mut line_buf));
                }
                last_y = Some(y);
            }
            line_buf.push(item.cell.c);
        }
        if last_y.is_some() {
            lines.push(line_buf);
        }
        lines
            .iter()
            .rev()
            .take(max_lines)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn viewport_line_text(&self, vy: usize) -> String {
        let content = self.term.renderable_content();
        let mut line_chars = Vec::new();
        for item in content.display_iter {
            if item.point.line.0 == vy as i32 {
                line_chars.push(item.cell.c);
            }
        }
        line_chars.into_iter().collect()
    }

    pub(crate) fn session_id_from_cmd(cmd: &str) -> Option<String> {
        for flag in ["--session ", "--resume "] {
            if let Some(pos) = cmd.find(flag) {
                let rest = &cmd[pos + flag.len()..];
                let id = rest.split_whitespace().next().unwrap_or("");
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    fn agent_status_running(&self) -> (Option<bool>, Option<String>) {

        let my_session = self
            .state
            .session_id
            .clone()
            .or_else(|| Self::session_id_from_cmd(&self.state.launch_cmd));

        let tty_candidate = (!self.state.tty_name.is_empty()).then(|| {
            let safe: String = self
                .state
                .tty_name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            crate::ipc::ipc_path(&format!("agent_status-{safe}.json"))
        });
        if let Some(p) = tty_candidate.as_deref() {
            if let Some(raw) = std::fs::read_to_string(p).ok() {
                if let Some(vals) = Self::parse_agent_status(&raw) {
                    return vals;
                }
            }
        }

        let candidate = my_session.as_ref().map(|sid| {
            let safe: String = sid
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            crate::ipc::ipc_path(&format!("agent_status-{safe}.json"))
        });
        let raw = candidate
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok());

        if let Some(raw) = raw {
            if let Some(vals) = Self::parse_agent_status(&raw) {
                return vals;
            }
        }

        let raw = std::fs::read_to_string(crate::ipc::ipc_path("agent_status.json")).ok();
        let Some(raw) = raw else {
            return (None, None);
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return (None, None);
        };
        let Some(updated_at) = json.get("updatedAt").and_then(|v| v.as_u64()) else {
            return (None, None);
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if now_ms.saturating_sub(updated_at) > AGENT_STATUS_MAX_AGE_MS {
            return (None, None);
        }
        let running = json.get("running").and_then(|v| v.as_bool());
        let error = json
            .get("error")
            .and_then(|e| e.as_str())
            .map(str::to_string);
        let file_session = json.get("sessionId").and_then(|v| v.as_str());
        match (file_session, my_session) {

            (Some(fs), Some(ms)) if fs == ms => (running, error),

            (None, None) if self.state.pane_count <= 1 => (running, error),
            _ => (None, None),
        }
    }

    #[cfg(test)]
    pub(crate) fn trust_global(
        file_session: Option<&str>,
        my_session: Option<&str>,
        pane_count: usize,
    ) -> bool {
        matches!(
            (file_session, my_session),
            (Some(fs), Some(ms)) if fs == ms
        ) || (file_session.is_none() && my_session.is_none() && pane_count <= 1)
    }

    fn parse_agent_status(raw: &str) -> Option<(Option<bool>, Option<String>)> {
        let json = serde_json::from_str::<serde_json::Value>(raw).ok()?;
        let updated_at = json.get("updatedAt").and_then(|v| v.as_u64())?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if now_ms.saturating_sub(updated_at) > AGENT_STATUS_MAX_AGE_MS {
            return None;
        }
        let running = json.get("running").and_then(|v| v.as_bool());
        let error = json
            .get("error")
            .and_then(|e| e.as_str())
            .map(str::to_string);
        Some((running, error))
    }

    pub fn agent_snapshot(&self) -> crate::agent_state::AgentSnapshot {
        let (agent_running, agent_error) = self.agent_status_running();
        crate::agent_state::AgentSnapshot {
            bottom_text: self.bottom_text(12),
            idle_ms: self.state.last_activity.elapsed().as_millis() as u64,
            user_interacted: self.state.has_user_prompted,
            exited: self.state.exited,
            agent_running,
            agent_error,
        }
    }

    fn discover_session_id(&mut self) {
        if self.state.session_id.is_some() {
            return;
        }

        let mut candidates: Vec<String> = Vec::new();
        if !self.state.tty_name.is_empty() {
            let safe: String = self
                .state
                .tty_name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            candidates.push(crate::ipc::ipc_path(&format!("agent_status-{safe}.json")));
        }
        for path in candidates {
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            let Some(sid) = json.get("sessionId").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(updated_at) = json.get("updatedAt").and_then(|v| v.as_u64()) else {
                continue;
            };
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if now_ms.saturating_sub(updated_at) <= AGENT_STATUS_MAX_AGE_MS && !sid.is_empty() {
                self.state.session_id = Some(sid.to_string());
                return;
            }
        }
    }

    pub fn is_session_live(&self, session_id: &str) -> bool {
        if self
            .state
            .launch_cmd
            .contains(&format!("--session {session_id}"))
            || self
                .state
                .launch_cmd
                .contains(&format!("--resume {session_id}"))
        {
            return true;
        }
        if self.state.session_id.as_deref() != Some(session_id) {
            return false;
        }

        if self.state.tty_name.is_empty() {
            return false;
        }
        let safe: String = self
            .state
            .tty_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let Ok(raw) = std::fs::read_to_string(crate::ipc::ipc_path(&format!("agent_status-{safe}.json")))
        else {
            return false;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return false;
        };
        let Some(updated_at) = json.get("updatedAt").and_then(|v| v.as_u64()) else {
            return false;
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms.saturating_sub(updated_at) <= AGENT_STATUS_MAX_AGE_MS
    }

    pub fn update_agent_state(&mut self) {
        if self.state.last_agent_check.elapsed() < std::time::Duration::from_millis(AGENT_CHECK_MS)
        {
            return;
        }
        self.state.last_agent_check = std::time::Instant::now();

        self.state.prompt_visible = self.is_at_prompt();
        self.discover_session_id();

        if self.state.loading {
            let stable = self.state.last_activity.elapsed()
                >= std::time::Duration::from_millis(LOADING_STABLE_MS);

            if (self.state.prompt_visible && stable) || self.state.has_user_prompted {
                self.state.loading = false;
            }
        }
        let snap = self.agent_snapshot();
        let observed = crate::agent_state::detect_agent_state(&snap);
        let before = self.state.agent_state;
        self.state.agent_state = self.state.agent_tracker.observe(observed);
        if self.state.agent_state != before {
            self.publish_blocked_status();
        }
        self.process_auto_retry();
    }

    pub fn process_auto_retry(&mut self) {
        if self.state.exited {
            return;
        }

        if self.state.agent_state == crate::agent_state::AgentState::Working
            || self.is_busy()
        {
            if self.state.auto_retry.sent_for_current_attempt {
                self.state.auto_retry.waiting_for_response = true;
                self.state.auto_retry.next_retry_at = None;
            }
            return;
        }

        if !self.is_at_prompt() {
            return;
        }

        let prefs = crate::prefs::Prefs::load().auto_retry;
        if !prefs.enabled {
            return;
        }
        let text = self.bottom_text(25);
        if let Some((err_type, sig)) = crate::auto_retry::classify_error(&text, &prefs) {
            let mut should_retry = false;
            {
                let tracker = &mut self.state.auto_retry;
                if tracker.last_error_sig.as_deref() != Some(&sig) {
                    tracker.last_error_sig = Some(sig);
                    tracker.attempt_count = 1;
                    tracker.active_error_label = Some(err_type.label().to_string());
                    tracker.sent_for_current_attempt = false;
                    tracker.waiting_for_response = false;
                    let delay = crate::auto_retry::calculate_backoff(1, &prefs);
                    tracker.next_retry_at = Some(std::time::Instant::now() + delay);
                } else if !tracker.sent_for_current_attempt && !tracker.waiting_for_response {
                    if let Some(target_time) = tracker.next_retry_at {
                        if std::time::Instant::now() >= target_time {
                            if prefs.max_retries == 0 || (tracker.attempt_count as i64) <= prefs.max_retries {
                                should_retry = true;
                                tracker.sent_for_current_attempt = true;
                                tracker.waiting_for_response = true;
                                tracker.next_retry_at = None;
                            } else {
                                tracker.next_retry_at = None;
                            }
                        }
                    }
                }
            }
            if should_retry {
                let prompt = if prefs.prompt.trim().is_empty() {
                    "continue".to_string()
                } else {
                    prefs.prompt.clone()
                };
                let bottom_few = self.bottom_text(3);
                let trimmed_prompt = prompt.trim();
                let already_typed = bottom_few
                    .lines()
                    .any(|l| l.trim().ends_with(trimmed_prompt));

                let tx = self.input_tx.clone();
                let text_to_send = if already_typed {
                    None
                } else {
                    Some(trimmed_prompt.to_string())
                };

                std::thread::spawn(move || {
                    if let Some(t) = text_to_send {
                        let _ = tx.send(t.into_bytes());
                        std::thread::sleep(std::time::Duration::from_millis(80));
                    }
                    let _ = tx.send(vec![b'\r']);
                });
                self.state.dirty = true;
            }
        } else {
            let tracker = &mut self.state.auto_retry;
            if tracker.last_error_sig.is_some() {
                tracker.last_error_sig = None;
                tracker.attempt_count = 0;
                tracker.next_retry_at = None;
                tracker.active_error_label = None;
                tracker.sent_for_current_attempt = false;
                tracker.waiting_for_response = false;
            }
        }
    }

    fn publish_blocked_status(&mut self) {
        use std::collections::BTreeMap;
        let path = crate::ipc::ipc_path("blocked_status.json");
        let mut blocked: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(entries) = json.get("blocked").and_then(|v| v.as_object()) {
                    for (k, v) in entries {
                        blocked.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let is_cc_pane = self.state.launch_cmd.contains("commandcode");
        let my_session = if is_cc_pane {
            self.state
                .session_id
                .clone()
                .or_else(|| Self::session_id_from_cmd(&self.state.launch_cmd))
        } else {
            None
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(sid) = my_session {
            if self.state.agent_state == crate::agent_state::AgentState::Blocked {
                blocked.insert(
                    sid,
                    serde_json::json!({ "blocked": true, "updatedAt": now_ms }),
                );
            } else {
                blocked.remove(&sid);
            }
        }
        let payload = serde_json::json!({
            "blocked": blocked,
            "updatedAt": now_ms,
        });
        let _ = std::fs::write(&path, serde_json::to_string(&payload).unwrap_or_default());
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols > 0 && rows > 0 {
            if self.term.columns() != cols as usize || self.term.screen_lines() != rows as usize {
                crate::ipc::log_append(
                    "resize.log",
                    &format!(
                        "pane_resize: engine=({},{}) -> ({cols},{rows})",
                        self.term.columns(),
                        self.term.screen_lines()
                    ),
                );
                let _ = self.master.resize(portable_pty::PtySize {
                    rows: rows as u16,
                    cols: cols as u16,
                    ..Default::default()
                });
                self.term.resize(TermSize {
                    cols: cols as usize,
                    rows: rows as usize,
                });
            }
        }
    }

    pub fn feed(&mut self, buf: &[u8]) {

        let is_query = buf == b"\x1b[6n";
        if !buf.is_empty() && !is_query {
            self.state.dirty = true;
        }
        if is_substantive_output(buf) {
            self.state.last_activity = std::time::Instant::now();
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.parser.advance(&mut self.term, buf);
        }));
        if result.is_err() {
            self.reset_engine_after_panic();
        }
    }

    fn reset_engine_after_panic(&mut self) {
        let size = TermSize {
            cols: self.term.columns(),
            rows: self.term.screen_lines(),
        };
        self.term = Term::new(TermConfig::default(), &size, VoidListener);
        self.parser = alacritty_terminal::vte::ansi::Processor::new();
        self.state.dirty = true;
    }

    pub fn note_paste(&mut self, text: &str) {
        let newlines = text.chars().filter(|&c| c == '\n').count() as u32;
        if newlines > 0 {
            self.state.paste_zone = Some((std::time::Instant::now(), newlines));
        }
    }

    pub fn write_paste(&mut self, text: &str) {
        self.note_paste(text);
        let bracketed = self
            .term
            .mode()
            .contains(alacritty_terminal::term::TermMode::BRACKETED_PASTE);
        if bracketed {
            let mut payload = Vec::with_capacity(text.len() + 12);
            payload.extend_from_slice(b"\x1b[200~");
            payload.extend_from_slice(text.as_bytes());
            payload.extend_from_slice(b"\x1b[201~");
            self.write_input(&payload);
        } else {
            self.write_input(text.as_bytes());
        }
    }

    pub fn scroll_display(&mut self, delta: i32) {
        self.state.last_manual_scroll = Some(std::time::Instant::now());
        self.term
            .scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
    }

    pub fn scroll_reset(&mut self) {
        self.term
            .scroll_display(alacritty_terminal::grid::Scroll::Bottom);
    }

    pub fn scroll_to_fraction(&mut self, fraction: f64) {
        let m = self.scroll_metrics();
        let max = m.max_offset_from_bottom;
        if max == 0 {
            return;
        }
        let target = (max as f64 * (1.0 - fraction.clamp(0.0, 1.0))).round() as usize;
        let current = m.offset_from_bottom;

        let delta = target as i32 - current as i32;
        if delta != 0 {
            self.scroll_display(delta);
        }
    }

    pub fn scroll_metrics(&self) -> crate::ui::widget::ScrollMetrics {
        let total = self.term.total_lines();
        let viewport = self.term.screen_lines().max(1);
        let offset = self.term.grid().display_offset();
        crate::ui::widget::ScrollMetrics {
            max_offset_from_bottom: total.saturating_sub(viewport),
            offset_from_bottom: offset.min(total.saturating_sub(viewport)),
            viewport_rows: viewport,
        }
    }

    pub fn write_input(&mut self, bytes: &[u8]) {
        let has_newline = bytes.contains(&b'\r') || bytes.contains(&b'\n');
        if has_newline {
            self.state.has_user_prompted = true;
            self.state.turns = self.state.turns.saturating_add(1);
        }

        let snaps = should_scroll_to_bottom(bytes)
            && (has_newline
                || bytes.first() == Some(&b'/')
                || self
                    .state
                    .last_manual_scroll
                    .map(|t| {
                        t.elapsed() >= std::time::Duration::from_millis(SCROLL_SNAP_GRACE_MS)
                    })
                    .unwrap_or(true));
        if snaps {
            if has_newline && self.state.has_user_prompted {
                let cursor = self.term.grid().cursor.point;
                let anchor = cursor.line.0 - self.term.grid().display_offset() as i32;
                self.state.prompt_anchors.push(anchor);
            }
            self.scroll_reset();
        }
        self.state.dirty = true;

        self.state.last_busy_check =
            std::time::Instant::now() - std::time::Duration::from_millis(BUSY_CHECK_MS + 1);

        if !bytes.is_empty() {
            let _ = self.input_tx.send(bytes.to_vec());
        }
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();

        {
            let mut w = self.writer.lock().unwrap_or_else(|e| e.into_inner());
            *w = Box::new(io::sink());
        }
        let _ = self.input_tx.send(Vec::new());
    }
}

pub use super::pane_keys::key_to_bytes;

impl Drop for MuxPane {
    fn drop(&mut self) {
        if let Some(pid) = self.child.process_id() {
            crate::orphan_journal::unregister(pid);
            let _ = self.child.kill();
        }
    }
}

