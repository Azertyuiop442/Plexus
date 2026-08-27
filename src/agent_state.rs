
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {

    Idle,

    Working,

    Blocked,
}

#[derive(Debug, Clone)]
pub struct AgentSnapshot {

    pub bottom_text: String,

    pub idle_ms: u64,

    pub user_interacted: bool,

    pub exited: bool,

    pub agent_running: Option<bool>,

    pub agent_error: Option<String>,
}

#[derive(Debug, Clone)]
struct Rule {
    state: AgentState,
    priority: i32,

    all: &'static [&'static str],

    any: &'static [&'static str],

    high_confidence: bool,
}

const API_ERROR_PRIORITY: i32 = 1100;

const COMMAND_CODE_MANIFEST: &[Rule] = &[

    Rule {
        state: AgentState::Blocked,
        priority: API_ERROR_PRIORITY,
        all: &["Error:"],
        any: &[
            "Input length",
            "exceeds the maximum",
            "Trace ID",
            "contact support",
            "rate limit",
            "timeout",
            "503",
        ],
        high_confidence: true,
    },

    Rule {
        state: AgentState::Blocked,
        priority: 1000,
        all: &["Esc to cancel"],
        any: &[
            "1. Yes, proceed",
            "do you want to proceed",
            "Enter to select",
        ],
        high_confidence: false,
    },
    Rule {
        state: AgentState::Blocked,
        priority: 990,
        all: &["↑↓ navigate", "Enter to select"],
        any: &["Esc to close", "Esc to cancel"],
        high_confidence: false,
    },
    Rule {
        state: AgentState::Blocked,
        priority: 980,
        all: &[],
        any: &["Inherit follows /model"],
        high_confidence: false,
    },

    Rule {
        state: AgentState::Blocked,
        priority: 970,
        all: &[],
        any: &[
            "bypass all permissions",
            "2. Yes, don't ask again",
            "2. Yes, don’t ask again",
        ],
        high_confidence: true,
    },

    Rule {
        state: AgentState::Blocked,
        priority: 960,
        all: &[],
        any: &[
            "What should Command Code do instead?",
            "what should command code do instead?",
        ],
        high_confidence: true,
    },

    Rule {
        state: AgentState::Blocked,
        priority: 955,
        all: &[],
        any: &[
            "User answered questions",
            "Answer the question",
            "QUESTION",
            "Select an option",
            "Type your answer",
        ],
        high_confidence: true,
    },

    Rule {
        state: AgentState::Working,
        priority: 975,
        all: &[],
        any: &[
            "Thinking...",
            "thinking...",
            "Planning...",
            "planning...",
            "Running...",
            "running...",
            "Searching...",
            "searching...",
            "Reading...",
            "reading...",
            "Executing...",
            "executing...",
            "Generating...",
            "generating...",
            "Working...",
            "working...",
            "esc to interrupt",
            "Esc to interrupt",
            "[ctrl+o to expand]",
        ],
        high_confidence: true,
    },

    Rule {
        state: AgentState::Idle,
        priority: 950,
        all: &[],
        any: &["Ask your question...", "? for shortcuts"],
        high_confidence: false,
    },
];

pub fn detect_agent_state(snapshot: &AgentSnapshot) -> AgentState {
    if snapshot.exited {
        return AgentState::Idle;
    }

    if snapshot.agent_error.is_some() {
        return AgentState::Blocked;
    }

    let blocked_tail = tail_lines(&snapshot.bottom_text, BLOCKED_TAIL_LINES);
    for rule in COMMAND_CODE_MANIFEST
        .iter()
        .filter(|r| r.state == AgentState::Blocked)
    {
        let hay = if rule.priority == API_ERROR_PRIORITY {
            &snapshot.bottom_text
        } else {
            &blocked_tail
        };
        let all_ok = rule.all.iter().all(|n| hay.contains(n));
        let any_ok =
            rule.any.is_empty() || rule.any.iter().any(|n| hay.contains(n));
        if all_ok && any_ok {

            if !rule.high_confidence && !snapshot.user_interacted {
                return AgentState::Idle;
            }
            return AgentState::Blocked;
        }
    }

    for rule in COMMAND_CODE_MANIFEST
        .iter()
        .filter(|r| r.state == AgentState::Working)
    {
        let all_ok = rule.all.iter().all(|n| snapshot.bottom_text.contains(n));
        let any_ok =
            rule.any.is_empty() || rule.any.iter().any(|n| snapshot.bottom_text.contains(n));
        if all_ok && any_ok {
            return AgentState::Working;
        }
    }

    if snapshot.agent_running == Some(true) {
        return AgentState::Working;
    }

    {
        let last_line = snapshot.bottom_text.rsplit('\n').next().unwrap_or("");
        let footer_visible = last_line.contains("✻ Worked for") || last_line.contains("Worked for");
        if footer_visible && snapshot.idle_ms >= WORKING_MS {
            return AgentState::Idle;
        }
    }

    for rule in COMMAND_CODE_MANIFEST
        .iter()
        .filter(|r| r.state == AgentState::Idle)
    {
        let all_ok = rule.all.iter().all(|n| snapshot.bottom_text.contains(n));
        let any_ok =
            rule.any.is_empty() || rule.any.iter().any(|n| snapshot.bottom_text.contains(n));
        if all_ok && any_ok {
            return AgentState::Idle;
        }
    }

    if snapshot.agent_running == Some(false) {
        return AgentState::Idle;
    }

    if snapshot.idle_ms < WORKING_MS {
        return AgentState::Working;
    }
    AgentState::Idle
}

pub const WORKING_MS: u64 = 300;

const BLOCKED_TAIL_LINES: usize = 3;

fn tail_lines(text: &str, n: usize) -> String {
    let mut lines: Vec<&str> = text.rsplit('\n').take(n).collect();
    lines.reverse();
    lines.join("\n")
}

const CONFIRMATIONS_TO_IDLE: u8 = 2;
const CONFIRMATIONS_FROM_IDLE: u8 = 3;

const STARTUP_GRACE: Duration = Duration::from_millis(1500);

#[derive(Debug)]
pub struct AgentStateTracker {
    state: AgentState,

    pending: Option<(AgentState, u8)>,
    started: Instant,
}

impl Default for AgentStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentStateTracker {
    pub fn new() -> Self {
        Self {
            state: AgentState::Idle,
            pending: None,
            started: Instant::now(),
        }
    }

    pub fn observe(&mut self, observed: AgentState) -> AgentState {
        if self.started.elapsed() < STARTUP_GRACE {

            self.pending = None;
            return AgentState::Idle;
        }
        if observed == self.state {
            self.pending = None;
            return self.state;
        }
        let confirmations = if observed == AgentState::Idle {
            CONFIRMATIONS_TO_IDLE
        } else {
            CONFIRMATIONS_FROM_IDLE
        };
        match &mut self.pending {
            Some((pending_state, count)) if *pending_state == observed => {
                *count += 1;
                if *count >= confirmations {
                    self.state = observed;
                    self.pending = None;
                }
            }
            _ => {
                self.pending = Some((observed, 1));
            }
        }
        self.state
    }

    #[allow(dead_code)]
    pub fn state(&self) -> AgentState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(
        bottom: &str,
        idle_ms: u64,
        interacted: bool,
        exited: bool,
        running: Option<bool>,
    ) -> AgentSnapshot {
        AgentSnapshot {
            bottom_text: bottom.to_string(),
            idle_ms,
            user_interacted: interacted,
            exited,
            agent_running: running,
            agent_error: None,
        }
    }

    fn snap_err(error: &str, running: Option<bool>) -> AgentSnapshot {
        AgentSnapshot {
            bottom_text: String::new(),
            idle_ms: 0,
            user_interacted: true,
            exited: false,
            agent_running: running,
            agent_error: Some(error.to_string()),
        }
    }

    #[test]
    fn interrupted_prompt_is_blocked() {

        let screen = "Interrupted · What should Command Code do instead?";
        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, None)),
            AgentState::Blocked
        );
        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, Some(true))),
            AgentState::Blocked
        );

        let full = "The agent was working on your request.\nInterrupted · What should Command Code do instead?\nType 'continue' to resume, or describe the changes you want.";
        assert_eq!(
            detect_agent_state(&snap(full, 5_000, true, false, Some(true))),
            AgentState::Blocked
        );
    }

    #[test]
    fn model_error_is_blocked() {

        assert_eq!(
            detect_agent_state(&snap_err("503 Service Unavailable", Some(true))),
            AgentState::Blocked
        );
        assert_eq!(
            detect_agent_state(&snap_err("rate limit exceeded", Some(false))),
            AgentState::Blocked
        );

        let mut fresh = snap_err("api error", None);
        fresh.user_interacted = false;
        assert_eq!(detect_agent_state(&fresh), AgentState::Blocked);
    }

    #[test]
    fn hybrid_prefers_structured_running_signal() {

        assert_eq!(
            detect_agent_state(&snap("static screen", 10_000, true, false, Some(true))),
            AgentState::Working
        );

        assert_eq!(
            detect_agent_state(&snap("output", 50, true, false, Some(false))),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap("output", 50, true, false, None)),
            AgentState::Working
        );
        assert_eq!(
            detect_agent_state(&snap("quiet", 5_000, true, false, None)),
            AgentState::Idle
        );
    }

    #[test]
    fn manifest_detects_model_api_errors() {

        let err_screen = "⚠ Error: 400 Input length 280460 exceeds the maximum allowed input length of 262112 tokens.\n  Type \"continue\" to try again. Trace ID: abc";
        assert_eq!(
            detect_agent_state(&snap(err_screen, 200, true, false, Some(true))),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap("Error: rate limit exceeded", 200, true, false, None)),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap("normal output", 200, true, false, None)),
            AgentState::Working
        );
    }

    #[test]
    fn thinking_and_planning_prompt_is_working() {

        let screen = "❯ mod-run\n* Thinking... (186 lines) [ctrl+o to expand]\n◇ Planning... esc to interrupt • 1m 34s • ↓ 12.8k\n❯ Ask your question...";
        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, true, false, None)),
            AgentState::Working
        );
    }

    #[test]
    fn running_signal_wins_over_idle_looking_prompt() {

        let screen = "Welcome to Command Code\nAsk your question...\n? for shortcuts";
        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, false, false, Some(true))),
            AgentState::Working
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, false, false, Some(false))),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, false, false, None)),
            AgentState::Idle
        );
    }

    #[test]
    fn blocked_rules_only_match_the_bottom_lines() {

        let stale = "1. Yes, proceed\nEsc to cancel\nstreaming more output\nand more";
        assert_eq!(
            detect_agent_state(&snap(stale, 200, true, false, None)),
            AgentState::Working
        );

        let live = "streaming more output\n1. Yes, proceed\nEsc to cancel";
        assert_eq!(
            detect_agent_state(&snap(live, 200, true, false, None)),
            AgentState::Blocked
        );
    }

    #[test]
    fn worked_for_footer_means_clean_turn_end() {

        let footer = "✻ Worked for 30s";
        assert_eq!(
            detect_agent_state(&snap(footer, 5_000, true, false, None)),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap("Worked for 12s", 5_000, true, false, None)),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap("some streamed text", 200, true, false, None)),
            AgentState::Working
        );

        assert_eq!(
            detect_agent_state(&snap(
                "✻ Worked for 30s\nnew stream text",
                200,
                true,
                false,
                None
            )),
            AgentState::Working
        );

        assert_eq!(
            detect_agent_state(&snap(footer, 5_000, true, false, Some(true))),
            AgentState::Working
        );
    }

    #[test]
    fn hybrid_manifest_blocked_wins_over_running() {

        let screen = "The agent wants to run:\n  bash(command)\n1. Yes, proceed\nEsc to cancel";
        assert_eq!(
            detect_agent_state(&snap(screen, 200, true, false, Some(true))),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, Some(true))),
            AgentState::Idle
        );
    }

    #[test]
    fn manifest_detects_idle_prompt() {
        let screen = "Welcome to Command Code\nAsk your question...\n? for shortcuts";

        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, false, false, None)),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, true, false, None)),
            AgentState::Idle
        );
    }

    #[test]
    fn manifest_detects_blocked_permission() {
        let screen = "The agent wants to run:\n  bash(command)\n1. Yes, proceed\nEsc to cancel";
        assert_eq!(
            detect_agent_state(&snap(screen, 200, true, false, None)),
            AgentState::Blocked
        );

        let menu = "Select a model\n↑↓ navigate · Enter to select · Esc to close";
        assert_eq!(
            detect_agent_state(&snap(menu, 200, true, false, None)),
            AgentState::Blocked
        );
    }

    #[test]
    fn manifest_blocked_requires_interaction() {

        let screen = "1. Yes, proceed\nEsc to cancel";
        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, None)),
            AgentState::Idle
        );
    }

    #[test]
    fn new_permission_prompt_is_blocked_even_without_interaction() {

        let screen = "Execute Shell Command\nCommand Code needs to execute echo \"PWD: $(pwd)\"; echo \"---\"; ls -la.\n\nPress [ctrl+e] to explain this command\n\n❯ 1. Yes\n  2. Yes, don't ask again for this exact command in this project\n  3. No, tell Command Code what to do differently\n\n↑/↓ navigate · enter select · ctrl+e explain · Run cmd --yolo to bypass all permissions (Docs ↗)";
        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, None)),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 200, false, false, Some(true))),
            AgentState::Blocked
        );

        let curly = screen.replace("don't", "don’t");
        assert_eq!(
            detect_agent_state(&snap(&curly, 200, false, false, None)),
            AgentState::Blocked
        );
    }

    #[test]
    fn activity_heuristics() {

        assert_eq!(
            detect_agent_state(&snap("streaming response...", 100, true, false, None)),
            AgentState::Working
        );

        assert_eq!(
            detect_agent_state(&snap("stable screen", 5_000, true, false, None)),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap("", 0, false, true, None)),
            AgentState::Idle
        );

        assert_eq!(
            detect_agent_state(&snap("x", WORKING_MS - 1, true, false, None)),
            AgentState::Working
        );
        assert_eq!(
            detect_agent_state(&snap("x", WORKING_MS, true, false, None)),
            AgentState::Idle
        );
    }

    #[test]
    fn api_error_blocked_even_without_user_interaction() {

        let err_screen = "╝\n⚠ Error: 400 Input length 336030 exceeds the maximum allowed input length of 262112 tokens.\n  Type \"continue\" to try again. If the issue persists, contact support: https://commandcode.ai/discord\n  Trace ID: 1af8d0a0c28c2724ed489c6a5a6a0aeb\n\n ✻ Worked for 7m 22s";
        let snap = AgentSnapshot {
            bottom_text: err_screen.to_string(),
            idle_ms: 5_000,
            user_interacted: false,
            exited: false,
            agent_running: Some(true),
            agent_error: None,
        };
        assert_eq!(detect_agent_state(&snap), AgentState::Blocked);

        let menu = AgentSnapshot {
            bottom_text: "1. Yes, proceed\nEsc to cancel".to_string(),
            idle_ms: 200,
            user_interacted: false,
            exited: false,
            agent_running: Some(true),
            agent_error: None,
        };
        assert_eq!(detect_agent_state(&menu), AgentState::Idle);
    }

    #[test]
    fn ask_user_question_prompt_is_blocked() {

        let screen = "Question: which approach should I take?\n  Option A\n  Option B\n[User answered questions]";
        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, true, false, Some(true))),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 5_000, false, false, None)),
            AgentState::Blocked
        );

        assert_eq!(
            detect_agent_state(&snap(screen, 200, true, false, Some(true))),
            AgentState::Blocked
        );
    }

    #[test]
    fn tracker_requires_confirmations_before_publishing() {
        let mut t = AgentStateTracker::new();

        t.started = Instant::now() - STARTUP_GRACE - Duration::from_secs(1);

        assert_eq!(t.observe(AgentState::Working), AgentState::Idle);
        assert_eq!(t.state(), AgentState::Idle);

        assert_eq!(t.observe(AgentState::Working), AgentState::Idle);

        assert_eq!(t.observe(AgentState::Working), AgentState::Working);
        assert_eq!(t.state(), AgentState::Working);

        assert_eq!(t.observe(AgentState::Idle), AgentState::Working);
        assert_eq!(t.state(), AgentState::Working);

        assert_eq!(t.observe(AgentState::Idle), AgentState::Idle);
        assert_eq!(t.state(), AgentState::Idle);
    }

    #[test]
    fn tracker_confirmations_are_asymmetric() {
        let mut t = AgentStateTracker::new();
        t.started = Instant::now() - STARTUP_GRACE - Duration::from_secs(1);

        assert_eq!(t.observe(AgentState::Working), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Working), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Working), AgentState::Working);

        assert_eq!(t.observe(AgentState::Idle), AgentState::Working);
        assert_eq!(t.observe(AgentState::Idle), AgentState::Idle);
        assert_eq!(t.state(), AgentState::Idle);

        assert_eq!(t.observe(AgentState::Blocked), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Blocked), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Blocked), AgentState::Blocked);
    }

    #[test]
    fn tracker_grace_window_swallows_transitions() {
        let mut t = AgentStateTracker::new();

        assert_eq!(t.observe(AgentState::Blocked), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Working), AgentState::Idle);
        assert_eq!(t.state(), AgentState::Idle);
    }

    #[test]
    fn tracker_pending_switches_when_observation_changes() {
        let mut t = AgentStateTracker::new();
        t.started = Instant::now() - STARTUP_GRACE - Duration::from_secs(1);

        t.observe(AgentState::Working);

        assert_eq!(t.observe(AgentState::Idle), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Idle), AgentState::Idle);
        assert_eq!(t.observe(AgentState::Idle), AgentState::Idle);
        assert_eq!(t.state(), AgentState::Idle);
    }
}

