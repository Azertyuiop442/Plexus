use std::io;

use alacritty_terminal::grid::Dimensions;
use crossterm::event::MouseEvent;

use super::mouse::content_origin;
use crate::state::AppState;

pub fn handle_scroll_accum(
    state: &mut AppState,
    mouse: MouseEvent,
    scroll_delta: i32,
) -> io::Result<()> {
    let viewport_h = state
        .panes
        .get(state.active)
        .map(|p| p.lock().map(|g| g.term.screen_lines()).unwrap_or(24))
        .unwrap_or(24) as u16;
    let delta = state.scroll_physics.apply(scroll_delta, viewport_h);

    let sidebar_w = state.sidebar_w;
    let over_sidebar = state.sidebar_open && mouse.column < sidebar_w;
    let over_tab_bar = !over_sidebar && mouse.row == 0;

    if over_sidebar {
        if delta > 0 {
            state.sidebar.prev();
        } else {
            state.sidebar.next();
        }
    } else if over_tab_bar {
        if delta > 0 {
            state.active = if state.active == 0 {
                state.panes.len().saturating_sub(1)
            } else {
                state.active - 1
            };
            state.refresh_mods_now();
        } else if !state.panes.is_empty() {
            state.active = (state.active + 1) % state.panes.len();
            state.refresh_mods_now();
        }
        state.sidebar_focus = false;
    } else if let Some(pane) = state.panes.get(state.active) {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let mode = *p.term.mode();
        let mouse_report = mode.intersects(
            alacritty_terminal::term::TermMode::MOUSE_REPORT_CLICK
                | alacritty_terminal::term::TermMode::MOUSE_DRAG
                | alacritty_terminal::term::TermMode::MOUSE_MOTION
                | alacritty_terminal::term::TermMode::SGR_MOUSE,
        );

        if mouse_report {
            let up = scroll_delta < 0;
            let btn = if up { 64 } else { 65 };
            let (content_left, content_top) = content_origin(state, 80);
            let col = mouse.column.saturating_sub(content_left) + 1;
            let row = mouse.row.saturating_sub(content_top) + 1;
            let seq = format!("\x1b[<{btn};{col};{row}M");
            p.write_input(seq.as_bytes());
        } else if mode.contains(alacritty_terminal::term::TermMode::ALT_SCREEN) {
            let up = scroll_delta < 0;
            let seq = if up { b"\x1b[A" } else { b"\x1b[B" };
            for _ in 0..scroll_delta.abs().min(5) {
                p.write_input(seq);
            }
        } else {
            p.scroll_display(delta);
        }
    }
    Ok(())
}
