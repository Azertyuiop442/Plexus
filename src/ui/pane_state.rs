
use crate::agent_state::{AgentState, AgentStateTracker};

#[derive(Clone, Debug)]
pub struct BootInfo {
    pub version: String,
    pub models: Option<String>,
    pub cwd: Option<String>,
    pub native_logo: Vec<String>,

    pub capture_w: u16,
}

pub const BUSY_CHECK_MS: u64 = 500;
pub const AGENT_CHECK_MS: u64 = 500;

#[derive(Debug)]
pub struct PaneState {
    pub title: String,

    pub launch_cmd: String,
    pub exited: bool,

    #[allow(dead_code)]
    pub resumed_session: bool,

    pub tty_name: String,

    pub yolo_mode: bool,

    pub paste_zone: Option<(std::time::Instant, u32)>,
    pub boot_info: Option<BootInfo>,

    pub turns: u32,
    pub has_user_prompted: bool,
    pub selection: Option<crate::selection::Selection>,
    #[allow(dead_code)]
    pub banner_render_cache: Option<crate::ui::banner::BannerCardCache>,

    pub banner_folder_icon: Option<(u16, u16)>,
    pub last_activity: std::time::Instant,

    pub dirty: bool,

    pub(crate) last_busy_check: std::time::Instant,
    pub(crate) busy_cache: bool,

    pub agent_state: AgentState,
    pub(crate) agent_tracker: AgentStateTracker,
    pub(crate) last_agent_check: std::time::Instant,

    pub prompt_visible: bool,

    pub loading: bool,

    pub pending_cwd: Option<String>,

    pub session_id: Option<String>,

    pub last_manual_scroll: Option<std::time::Instant>,

    pub gen: u64,

    pub pane_count: usize,

    pub prompt_anchors: Vec<i32>,
    pub auto_retry: crate::auto_retry::RetryTracker,
    pub working_started_at: Option<std::time::Instant>,
    pub sound_played_for_run: bool,
    pub last_sound_at: Option<std::time::Instant>,
}

impl PaneState {
    pub fn new(launch_cmd: String) -> Self {
        let resumed_session = launch_cmd.contains("--session") || launch_cmd.contains("--resume");
        let yolo_mode = launch_cmd.contains("--yolo");
        Self {
            title: launch_cmd.clone(),
            launch_cmd,
            exited: false,
            resumed_session,
            tty_name: String::new(),
            yolo_mode,
            paste_zone: None,
            boot_info: None,
            turns: 0,
            has_user_prompted: false,
            selection: None,
            banner_render_cache: None,
            banner_folder_icon: None,
            last_activity: std::time::Instant::now(),
            dirty: true,
            last_busy_check: std::time::Instant::now()
                - std::time::Duration::from_millis(BUSY_CHECK_MS + 1),
            busy_cache: false,
            agent_state: AgentState::Idle,
            agent_tracker: AgentStateTracker::new(),
            last_agent_check: std::time::Instant::now()
                - std::time::Duration::from_millis(AGENT_CHECK_MS + 1),
            prompt_visible: false,
            loading: resumed_session,
            pending_cwd: None,
            session_id: None,
            last_manual_scroll: None,
            gen: 0,
            pane_count: 1,
            prompt_anchors: Vec::new(),
            auto_retry: crate::auto_retry::RetryTracker::default(),
            working_started_at: None,
            sound_played_for_run: false,
            last_sound_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_state_is_testable_without_pty() {
        let mut st = PaneState::new("commandcode --yolo".to_string());

        assert_eq!(st.title, "commandcode --yolo");
        assert_eq!(st.launch_cmd, "commandcode --yolo");
        assert!(!st.exited);
        assert!(!st.resumed_session);
        assert_eq!(st.turns, 0);
        assert_eq!(st.agent_state, AgentState::Idle);
        assert!(st.dirty);

        st.turns = 3;
        st.dirty = false;
        st.agent_state = AgentState::Working;
        assert_eq!(st.turns, 3);
        assert!(!st.dirty);
        assert_eq!(st.agent_state, AgentState::Working);
    }

    #[test]
    fn pane_state_detects_resume_and_yolo_flags() {
        let st = PaneState::new("commandcode --session abc-123 --yolo".to_string());
        assert!(st.resumed_session);
        assert!(st.yolo_mode);

        let st = PaneState::new("commandcode --resume xyz".to_string());
        assert!(st.resumed_session);
        assert!(!st.yolo_mode);

        let st = PaneState::new("zsh".to_string());
        assert!(!st.resumed_session);
        assert!(!st.yolo_mode);
    }

    #[test]
    fn pane_state_loading_flag_tracks_resume() {

        let resumed = PaneState::new("commandcode --session abc".to_string());
        assert!(resumed.loading);

        let fresh = PaneState::new("commandcode".to_string());
        assert!(!fresh.loading);

        let shell = PaneState::new("zsh".to_string());
        assert!(!shell.loading);
    }
}

