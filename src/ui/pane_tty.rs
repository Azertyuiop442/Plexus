
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::vte::ansi::NamedColor;
use ratatui::style::Color as RColor;

pub struct TermSize {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub fn is_substantive_output(buf: &[u8]) -> bool {
    if buf.is_empty() {
        return false;
    }
    if buf.starts_with(b"\x1b[?")
        || buf.starts_with(b"\x1b[6n")
        || buf == b"\x1b[?25h"
        || buf == b"\x1b[?25l"
    {
        return false;
    }

    let mut printable = 0usize;
    let mut in_esc = false;
    let mut osc = false;
    let mut i = 0;
    while i < buf.len() {
        let b = buf[i];
        if in_esc {
            if osc {
                if b == 0x07 || b == 0x1b {
                    in_esc = false;
                    osc = false;
                }
            } else if b == b']' {
                osc = true;
            } else if (0x40..=0x7e).contains(&b) && b != b'[' {
                in_esc = false;
            }
            i += 1;
            continue;
        }
        if b == 0x1b {
            in_esc = true;
            i += 1;
            continue;
        }
        if (b >= 32 && b <= 126) || b > 127 {
            printable += 1;
        }
        i += 1;
    }
    printable > 3 || (buf.len() > 12 && printable > 0)
}

pub fn tty_of_pid(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let comm_end = stat.rfind(')')?;
        let rest = &stat[comm_end + 1..];
        let mut fields = rest.split_whitespace();
        let _ = fields.next()?;
        let _ = fields.next()?;
        let _ = fields.next()?;
        let _ = fields.next()?;
        let tty_nr: u32 = fields.next()?.parse().ok()?;
        return tty_device_name(tty_nr);
    }
    #[cfg(not(target_os = "linux"))]
    {
        let out = std::process::Command::new("ps")
            .args(["-o", "tty=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let tty = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if tty.is_empty() || tty == "??" || tty == "?" {
            return None;
        }
        Some(tty)
    }
}

#[cfg(target_os = "linux")]
fn tty_device_name(tty_nr: u32) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let major = (tty_nr >> 8) & 0xfff;
    let minor = (tty_nr & 0xff) | ((tty_nr >> 12) & 0xfff00);
    if major == 0 {
        return None;
    }
    let dev = libc::makedev(major as u32, minor as u32);
    for dir in ["/dev/pts", "/dev"] {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let Ok(meta) = entry.metadata() else { continue };
                if meta.rdev() == dev {
                    return entry.file_name().to_str().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

pub fn should_scroll_to_bottom(bytes: &[u8]) -> bool {
    match bytes.first() {
        Some(0x1b) => false,
        Some(b) => b.is_ascii_graphic() || *b == b' ' || *b == b'\r' || *b == b'\n',
        None => false,
    }
}

pub fn to_ratatui_color(color: &alacritty_terminal::vte::ansi::Color, default: RColor) -> RColor {
    match color {
        alacritty_terminal::vte::ansi::Color::Spec(rgb) => RColor::Rgb(rgb.r, rgb.g, rgb.b),
        alacritty_terminal::vte::ansi::Color::Indexed(i) => RColor::Indexed(*i),
        alacritty_terminal::vte::ansi::Color::Named(named) => match named {
            NamedColor::Black => RColor::Black,
            NamedColor::Red => RColor::Red,
            NamedColor::Green => RColor::Green,
            NamedColor::Yellow => RColor::Yellow,
            NamedColor::Blue => RColor::Blue,
            NamedColor::Magenta => RColor::Magenta,
            NamedColor::Cyan => RColor::Cyan,
            NamedColor::White => RColor::Gray,
            NamedColor::BrightBlack => RColor::DarkGray,
            NamedColor::BrightRed => RColor::LightRed,
            NamedColor::BrightGreen => RColor::LightGreen,
            NamedColor::BrightYellow => RColor::LightYellow,
            NamedColor::BrightBlue => RColor::LightBlue,
            NamedColor::BrightMagenta => RColor::LightMagenta,
            NamedColor::BrightCyan => RColor::LightCyan,
            NamedColor::BrightWhite => RColor::White,
            _ => default,
        },
    }
}

