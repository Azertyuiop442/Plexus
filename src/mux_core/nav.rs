
use std::io;
use std::io::Write;

use crossterm::event::DisableMouseCapture;
use crossterm::execute;

use crate::state::AppState;
use crate::ui::pane::MuxPane;

use super::pane_ops::active_pane_size;

pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn valid_session_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub fn session_launch_cmd(cwd: &str, session_id: &str) -> Option<String> {
    if !valid_session_id(session_id) {
        return None;
    }
    if cwd.is_empty() {
        Some(format!("commandcode --session {session_id}"))
    } else {
        Some(format!(
            "cd {} && commandcode --session {session_id}",
            shell_quote(cwd)
        ))
    }
}

pub fn send_slash_command(pane: &mut MuxPane, command: &str) {
    let mut bytes = Vec::new();
    bytes.push(0x15);
    bytes.extend_from_slice(command.as_bytes());
    bytes.push(b'\r');
    pane.write_input(&bytes);
}

pub fn sanitize_clipboard(s: &str) -> String {
    s.chars()
        .filter(|&c| c == '\t' || c == '\n' || c == '\r' || !c.is_control())
        .collect()
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(T[(n >> 6) as usize & 63] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[n as usize & 63] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn copy_to_clipboard(text: &str) {
    let cleaned = sanitize_clipboard(text);
    if cleaned.is_empty() {
        return;
    }

    let osc52 = format!("\x1b]52;c;{}\x07", base64_encode(cleaned.as_bytes()));
    use std::io::Write as _;
    let _ = io::stdout().write_all(osc52.as_bytes());
    let _ = io::stdout().flush();

    let text = cleaned;
    std::thread::spawn(move || {
        let mut child = match std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = child
            .stdin
            .take()
            .map(|mut s| s.write_all(text.as_bytes()));
        let _ = child.wait();
    });
}

pub fn read_clipboard() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("pbpaste")
            .output()
            .ok()
            .and_then(|out| if out.status.success() { String::from_utf8(out.stdout).ok() } else { None })
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("wl-paste")
            .output()
            .or_else(|_| {
                std::process::Command::new("xclip")
                    .args(["-selection", "clipboard", "-o"])
                    .output()
            })
            .ok()
            .and_then(|out| if out.status.success() { String::from_utf8(out.stdout).ok() } else { None })
    }
}

pub fn reload_mux() {
    use std::os::unix::process::CommandExt;

    crate::ipc::log_append("resize.log", "reload: SIGUSR1 exec");
    crate::orphan_journal::kill_all_registered();
    let mut stdout = io::stdout();
    let _ = execute!(stdout, DisableMouseCapture);
    let _ = execute!(stdout, crossterm::event::DisableBracketedPaste);
    let _ = execute!(stdout, crossterm::event::PopKeyboardEnhancementFlags);
    let _ = crossterm::terminal::disable_raw_mode();
    print!("\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?25h\x1b[2J\x1b[1;1H");
    let _ = stdout.flush();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let bin = std::path::PathBuf::from(format!("{}/.commandcode/bin/cc-mux", home));
    let args: Vec<String> = std::env::args().collect();
    let target = if bin.exists() {
        bin
    } else {
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("cc-mux"))
    };

    let err = std::process::Command::new(&target)
        .args(&args[1..])
        .exec();

    eprintln!("Failed to reload cc-mux: {}", err);
    std::process::exit(1);
}

pub fn change_pane_cwd(state: &mut AppState) {

    let script = r#"POSIX path of (choose folder with prompt "Choose working directory")"#;
    let out = std::process::Command::new("osascript")
        .args(["-e", script])
        .output();
    let Ok(out) = out else {
        return;
    };
    if !out.status.success() {
        return;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() || !std::path::Path::new(&path).is_dir() {
        return;
    }

    let Some(pane) = state.panes.get(state.active) else {
        return;
    };
    let (cols, rows) = active_pane_size(state);
    let cmd = format!("cd {} && commandcode", shell_quote(&path));
    if let Ok((new_pane, reader)) = MuxPane::spawn(&cmd, cols, rows) {

        let keep_title = pane.lock().unwrap_or_else(|e| e.into_inner()).state.title.clone();
        pane.lock().unwrap_or_else(|e| e.into_inner()).kill();
        {
            let mut np = new_pane.lock().unwrap_or_else(|e| e.into_inner());
            np.state.title = keep_title;
        }
        state.panes[state.active] = new_pane.clone();
        std::thread::spawn(move || {
            use std::io::Read as _;
            let mut reader = reader;
            let mut buf = [0u8; 16384];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = new_pane.lock() {
                            p.feed(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut p) = new_pane.lock() {
                p.state.exited = true;
            }
        });
        state.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_strictly_validated() {

        assert!(valid_session_id("abc-123"));
        assert!(valid_session_id("1af8d0a0-c28c-2724-ed48-9c6a5a6a0aeb"));
        assert!(valid_session_id("abc_123.def"));

        assert!(!valid_session_id(""));
        assert!(!valid_session_id("foo; rm -rf ~"));
        assert!(!valid_session_id("$(touch /tmp/pwned)"));
        assert!(!valid_session_id("abc def"));
        assert!(!valid_session_id("abc'def"));
        assert!(!valid_session_id("abc|def"));
        assert!(!valid_session_id("abc&&def"));
    }

    #[test]
    fn hostile_session_id_never_reaches_a_shell_command() {

        let evil = "abc; touch /tmp/cc-injection-test";
        let cmd = session_launch_cmd("/tmp", evil);
        assert!(cmd.is_none(), "hostile id must never build a command");

        let cmd = session_launch_cmd("/tmp", "abc-123").expect("valid id builds");
        assert_eq!(cmd, "cd '/tmp' && commandcode --session abc-123");
        let cmd = session_launch_cmd("", "abc-123").expect("valid id builds");
        assert_eq!(cmd, "commandcode --session abc-123");
    }

    #[test]
    fn shell_quote_handles_quotes_and_spaces() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("/tmp/o'brien"), "'/tmp/o'\\''brien'");
    }

    #[test]
    fn sanitize_clipboard_strips_control_chars_keeps_formatting() {
        assert_eq!(sanitize_clipboard("hello"), "hello");
        assert_eq!(sanitize_clipboard("a\tb\nc\rd"), "a\tb\nc\rd");

        assert_eq!(sanitize_clipboard("\x1b[2Jabc\x00def\x7f"), "[2Jabcdef");
        assert_eq!(sanitize_clipboard("héllo → ok"), "héllo → ok");
    }

    #[test]
    fn base64_encoder_matches_rfc4648() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn copy_to_clipboard_sanitizes_before_write() {

        let cleaned = sanitize_clipboard("\x1b]0;evil\x07safe");
        assert_eq!(cleaned, "]0;evilsafe");
    }
}

