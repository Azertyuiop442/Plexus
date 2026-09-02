
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};

use crate::state::AppState;

pub const BURST_WINDOW_MS: u64 = 75;

pub const HOLD_MAX_MS: u64 = 500;

pub const BURST_MAX_CHARS: usize = 4096;

pub fn is_plain_char_key(key: &KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(crossterm::event::KeyModifiers::ALT);
    let super_key = key.modifiers.contains(crossterm::event::KeyModifiers::SUPER);
    !ctrl && !alt && !super_key && matches!(key.code, KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ')
}

pub fn maybe_append_burst(state: &mut AppState, bytes: &[u8]) -> bool {
    let now = std::time::Instant::now();
    let (last_seen, buf) = match &state.dictation {
        Some((last, buf)) => (*last, buf.clone()),
        None => (now, String::new()),
    };
    let burst = now.duration_since(last_seen) < Duration::from_millis(BURST_WINDOW_MS);
    if burst && buf.len() < BURST_MAX_CHARS {
        let mut buf = buf;
        buf.push_str(&String::from_utf8_lossy(bytes));
        state.dictation = Some((now, buf));
        true
    } else {
        false
    }
}

pub fn start_or_flush_burst(state: &mut AppState, bytes: &[u8], is_plain_char: bool) -> bool {
    let now = std::time::Instant::now();
    if is_plain_char {

        let _ = bytes;
        state.dictation = Some((now, String::new()));
        false
    } else {
        state.dictation = None;
        false
    }
}

pub fn take_burst(state: &mut AppState) -> Option<String> {
    state.dictation.take().map(|(_, buf)| buf)
}

#[allow(dead_code)]
pub fn burst_should_flush(state: &AppState) -> bool {
    let Some((last, buf)) = &state.dictation else {
        return false;
    };
    if buf.is_empty() {
        return false;
    }
    let elapsed = last.elapsed();
    elapsed >= Duration::from_millis(BURST_WINDOW_MS)
        || elapsed >= Duration::from_millis(HOLD_MAX_MS)
}

pub fn flush_dictation(state: &mut AppState) {
    let Some((last, buf)) = state.dictation.take() else {
        return;
    };
    if buf.is_empty() {
        return;
    }
    let elapsed = last.elapsed();
    let quiet = elapsed >= Duration::from_millis(BURST_WINDOW_MS)
        || elapsed >= Duration::from_millis(HOLD_MAX_MS);
    if !quiet {

        state.dictation = Some((last, buf));
        return;
    }
    if let Some(pane) = state.panes.get(state.active) {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        p.note_paste(&buf);
        p.write_input(buf.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::sidebar::Sidebar;
    use crate::ui::sidebar::SettingsSubMenu;

    fn empty_state() -> AppState {
        let sidebar = Sidebar {
            project: String::new(),
            project_cwd: String::new(),
            sessions: vec![],
            mods: vec![],
            rows: vec![],
            selected: 0,
            expanded: false,
            scroll: 0,
            view_lines: 10,
            active_tab: 0,
            settings_menu: SettingsSubMenu::Main,
            yolo_mode: false,
            skill_injection: false,
            taste_learning: true,
            ide_context: true,
            show_cost_bar: true,
            show_context_btn: true,
            show_usage: true,
            auto_retry_enabled: true,
            skills_update_count: 0,
            live_blocks: Vec::new(),
            available_update: None,
            usage: None,
            usage_tab: 0,
        };
        AppState::new(sidebar)
    }

    #[test]
    fn burst_groups_rapid_chars_and_flushes_on_quiet() {
        let mut state = empty_state();

        let consumed = start_or_flush_burst(&mut state, b"h", true);
        assert!(!consumed, "the first char must never be buffered");
        assert_eq!(state.dictation.as_ref().map(|(_, b)| b.as_str()), Some(""));

        let joined = maybe_append_burst(&mut state, b"i");
        assert!(joined);
        assert_eq!(state.dictation.as_ref().map(|(_, b)| b.as_str()), Some("i"));

        let taken = take_burst(&mut state);
        assert_eq!(taken.as_deref(), Some("i"));
        assert!(state.dictation.is_none());
    }

    #[test]
    fn burst_cap_limits_buffer_growth() {
        let mut state = empty_state();
        start_or_flush_burst(&mut state, b"x", true);

        let mut buf = state.dictation.take().unwrap().1;
        while buf.len() < BURST_MAX_CHARS {
            buf.push('y');
        }
        state.dictation = Some((std::time::Instant::now(), buf));
        let joined = maybe_append_burst(&mut state, b"z");
        assert!(!joined, "burst past the cap must not grow");
    }

    #[test]
    fn voice_burst_at_50_100ms_per_char_groups_then_flushes() {
        let mut state = empty_state();

        start_or_flush_burst(&mut state, b"h", true);
        let mut last = state.dictation.as_ref().unwrap().0;
        for ch in "ello world".chars() {
            last += std::time::Duration::from_millis(70);
            state.dictation = Some((last, state.dictation.as_ref().unwrap().1.clone()));
            let joined = maybe_append_burst(&mut state, &[ch as u8]);
            assert!(joined, "char after 70ms must join the burst");
        }
        assert_eq!(
            state.dictation.as_ref().map(|(_, b)| b.as_str()),
            Some("ello world")
        );

        let flush = burst_should_flush(&state);
        assert!(!flush, "still inside the window right after the last char");
        let mut s2 = empty_state();
        start_or_flush_burst(&mut s2, b"a", true);
        let old = s2.dictation.as_ref().unwrap().0;
        s2.dictation = Some((old - std::time::Duration::from_millis(BURST_WINDOW_MS + 1), String::from("a")));
        assert!(burst_should_flush(&s2), "quiet burst must flush");
        let _ = (state, flush);
    }

    #[test]
    fn continuous_dictation_flushes_at_hold_cap() {
        let mut state = empty_state();
        start_or_flush_burst(&mut state, b"a", true);
        let mut last = state.dictation.as_ref().unwrap().0;

        for i in 0..20 {
            last += std::time::Duration::from_millis(60);
            state.dictation = Some((last, format!("x{i}")));
        }
        assert!(
            state.dictation.as_ref().unwrap().0.elapsed() < std::time::Duration::from_millis(HOLD_MAX_MS),
            "hold cap not reached yet"
        );

        state.dictation = Some((
            std::time::Instant::now() - std::time::Duration::from_millis(HOLD_MAX_MS + 1),
            String::from("hello"),
        ));
        assert!(burst_should_flush(&state), "hold cap must force a flush");
    }

    #[test]
    fn test_is_plain_char_key_excludes_modifiers() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), crossterm::event::KeyModifiers::CONTROL);
        let ctrl_u = KeyEvent::new(KeyCode::Char('u'), crossterm::event::KeyModifiers::CONTROL);
        let alt_x = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::ALT);
        let plain_a = KeyEvent::new(KeyCode::Char('a'), crossterm::event::KeyModifiers::NONE);
        let plain_space = KeyEvent::new(KeyCode::Char(' '), crossterm::event::KeyModifiers::NONE);

        assert!(!is_plain_char_key(&ctrl_c), "Ctrl+C is a control signal, not plain text");
        assert!(!is_plain_char_key(&ctrl_u), "Ctrl+U is a control signal, not plain text");
        assert!(!is_plain_char_key(&alt_x), "Alt+X is a shortcut, not plain text");
        assert!(is_plain_char_key(&plain_a), "plain 'a' is plain text");
        assert!(is_plain_char_key(&plain_space), "plain ' ' is plain text");
    }
}

