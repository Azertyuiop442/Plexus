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
                    let alive = crate::orphan_journal::is_process_alive(pid);
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

pub fn clean_base_cmd(cmd: &str, yolo: bool) -> String {
    let trimmed = cmd.trim();
    let after_cd = if let Some(rest) = trimmed.strip_prefix("Set-Location ") {
        rest.split_once(';')
            .map(|(_, right)| right.trim())
            .unwrap_or(trimmed)
    } else if let Some(rest) = trimmed.strip_prefix("cd ") {
        rest.split_once("&&")
            .map(|(_, right)| right.trim())
            .unwrap_or(trimmed)
    } else {
        trimmed
    };

    let words: Vec<&str> = after_cd.split_whitespace().collect();
    let mut out: Vec<String> = Vec::new();
    let mut skip_next = false;
    let mut has_yolo = false;

    for &w in &words {
        if skip_next {
            skip_next = false;
            continue;
        }
        if w == "--session" || w == "--resume" {
            skip_next = true;
            continue;
        }
        if w.starts_with("--session=") || w.starts_with("--resume=") {
            continue;
        }
        if w == "--yolo" {
            has_yolo = true;
            out.push(w.to_string());
            continue;
        }
        out.push(w.to_string());
    }

    if out.is_empty() {
        out.push("commandcode".to_string());
    }

    if yolo && !has_yolo && out.first().map(|s| s.contains("commandcode")).unwrap_or(false) {
        out.push("--yolo".to_string());
    }

    out.join(" ")
}

pub fn replace_pane_cwd_by_gen(
    state: &mut AppState,
    target_gen: u64,
    new_cwd: &str,
) -> io::Result<()> {
    let target_idx = match state.panes.iter().position(|p| {
        p.lock()
            .map(|g| g.state.gen == target_gen)
            .unwrap_or(false)
    }) {
        Some(idx) => idx,
        None => return Ok(()),
    };

    let old_pane = state.panes[target_idx].clone();
    let (cols, rows, old_title, old_cmd) = {
        let old = old_pane.lock().unwrap_or_else(|e| e.into_inner());
        let cols = (old.term.columns() as u16).max(20);
        let rows = (old.term.screen_lines() as u16).max(5);
        (
            cols,
            rows,
            old.state.title.clone(),
            old.state.launch_cmd.clone(),
        )
    };

    let base_cmd = clean_base_cmd(&old_cmd, state.sidebar.yolo_mode);
    #[cfg(windows)]
    let new_cmd = format!("Set-Location '{}'; {}", new_cwd, base_cmd);
    #[cfg(not(windows))]
    let new_cmd = format!(
        "cd {} && {}",
        crate::mux_core::nav::shell_quote(new_cwd),
        base_cmd
    );

    let (new_pane, reader) = MuxPane::spawn(&new_cmd, cols, rows)?;

    let gen = state.next_pane_gen;
    state.next_pane_gen += 1;

    {
        let mut np = new_pane.lock().unwrap_or_else(|e| e.into_inner());
        np.state.title = old_title;
        np.state.gen = gen;
        np.state.pending_cwd = Some(new_cwd.to_string());
        np.state.boot_info = None;
        np.state.banner_render_cache = None;
        np.state.pane_count = state.panes.len();
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

    if let Ok(p) = new_pane.lock() {
        let pid = p.child_pid;
        if pid != 0 {
            let events = state.events.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let alive = crate::orphan_journal::is_process_alive(pid);
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

    state.panes[target_idx] = new_pane.clone();
    old_pane.lock().unwrap_or_else(|e| e.into_inner()).kill();

    if let Some(mut logo) = state
        .panes
        .iter()
        .find_map(|p| p.lock().ok().and_then(|g| g.state.boot_info.clone()))
    {
        let mut np = new_pane.lock().unwrap_or_else(|e| e.into_inner());
        logo.cwd = Some(new_cwd.to_string());
        np.state.boot_info = Some(logo);
    }

    state.dirty = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_base_cmd_preserves_plain_command() {
        assert_eq!(clean_base_cmd("commandcode", false), "commandcode");
        assert_eq!(clean_base_cmd("commandcode", true), "commandcode --yolo");
    }

    #[test]
    fn clean_base_cmd_strips_cd_prefix() {
        assert_eq!(
            clean_base_cmd("cd '/Users/test/dir' && commandcode", false),
            "commandcode"
        );
        assert_eq!(
            clean_base_cmd("cd '/Users/test/dir' && commandcode --yolo", true),
            "commandcode --yolo"
        );
    }

    #[test]
    fn clean_base_cmd_strips_session_flags() {
        assert_eq!(
            clean_base_cmd("cd '/old' && commandcode --session sess123 --yolo", true),
            "commandcode --yolo"
        );
        assert_eq!(
            clean_base_cmd("commandcode --resume sess456", false),
            "commandcode"
        );
    }

    #[test]
    fn clean_base_cmd_preserves_other_shells() {
        assert_eq!(clean_base_cmd("/bin/zsh", false), "/bin/zsh");
        assert_eq!(clean_base_cmd("cd '/foo' && /bin/bash", true), "/bin/bash");
    }
}

