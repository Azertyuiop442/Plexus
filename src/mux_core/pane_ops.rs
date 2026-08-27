use std::io::{self, Read};

use crate::mux_events::MuxEvent;
use crate::state::AppState;
use crate::ui::pane::MuxPane;
use alacritty_terminal::grid::Dimensions;

pub fn spawn_pane(state: &mut AppState, cmd: &str, cols: u16, rows: u16) -> io::Result<()> {
    let next_num = state.panes.len() + 1;
    crate::ipc::log_append("resize.log", &format!("pane_spawn: #{next_num} at {cols}x{rows}"));
    let (pane, reader) = MuxPane::spawn(cmd, cols, rows)?;

    let gen = state.next_pane_gen;
    state.next_pane_gen += 1;
    crate::ipc::log_append("resize.log", &format!("pane_spawn: #{next_num} gen={gen}"));
    {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        p.state.title = format!("Terminal {}", next_num);
        p.state.gen = gen;

        p.state.pending_cwd = launch_cwd(cmd).or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|d| d.to_string_lossy().to_string())
        });
    }
    let events = state.events.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 16384];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {

                    let _ = events.send(MuxEvent::PtyOutput {
                        gen,
                        bytes: buf[..n].to_vec(),
                    });
                }
                Err(_) => break,
            }
        }
        let _ = events.send(MuxEvent::PaneExited { gen });
        crate::ipc::log_append("resize.log", &format!("pane_exited: gen={gen} (reader EOF)"));
    });

    if let Ok(p) = pane.lock() {
        let pid = p.child_pid;
        if pid != 0 {
            let events = state.events.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    let alive = unsafe { libc::kill(pid as i32, 0) == 0 };
                    if !alive {

                        std::thread::sleep(std::time::Duration::from_millis(250));
                        let _ = events.send(MuxEvent::PaneExited { gen });
                        crate::ipc::log_append("resize.log", &format!("pane_exited: gen={gen} (watchdog)"));
                        return;
                    }
                }
            });
        }
    }
    state.panes.push(pane);

    state.dirty = true;

    for p in &state.panes {
        if let Ok(mut g) = p.lock() {
            g.state.pane_count = state.panes.len();
        }
    }

    if state.panes.len() > 1 {
        let logo = state.panes[0].lock().unwrap_or_else(|e| e.into_inner()).state.boot_info.clone();
        if let Some(mut logo) = logo {
            let mut new_pane = state.panes.last().unwrap().lock().unwrap_or_else(|e| e.into_inner());
            if new_pane.state.boot_info.is_none() {

                logo.cwd = new_pane.state.pending_cwd.clone();
                new_pane.state.boot_info = Some(logo);
            }
        }
    }
    Ok(())
}

pub fn launch_cwd(cmd: &str) -> Option<String> {
    let c = cmd.trim();
    let rest = c.strip_prefix("cd ")?;
    let dir = rest.split("&&").next()?.trim();
    let dir = dir
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(dir)
        .trim();
    if dir.is_empty() {
        None
    } else {
        Some(dir.to_string())
    }
}

pub fn active_pane_size(state: &AppState) -> (u16, u16) {
    match state.panes.get(state.active) {
        Some(pane) => {
            let p = pane.lock().unwrap_or_else(|e| e.into_inner());
            (p.term.columns() as u16, p.term.screen_lines() as u16)
        }
        None => (80, 24),
    }
}

