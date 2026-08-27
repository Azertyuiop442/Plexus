
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let super_key = key.modifiers.contains(KeyModifiers::SUPER);

    let mut out = Vec::new();

    if super_key {
        return out;
    }

    match key.code {
        KeyCode::Char(c) if ctrl => {
            let byte = (c as u8) & 0x1f;
            out.push(byte);
        }
        KeyCode::Char(c) => {
            if alt {
                out.push(0x1b);
            }
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
        KeyCode::Enter => {
            if shift {

                out.extend_from_slice(b"\x1b[13;2u");
            } else if alt {

                out.extend_from_slice(b"\x1b\r");
            } else if ctrl {

                out.push(b'\n');
            } else {
                out.push(b'\r');
            }
        }
        KeyCode::Tab => {
            if shift {
                out.extend_from_slice(b"\x1b[Z");
            } else {
                out.push(b'\t');
            }
        }
        KeyCode::BackTab => out.extend_from_slice(b"\x1b[Z"),
        KeyCode::Backspace => out.push(0x7f),
        KeyCode::Delete => out.extend_from_slice(b"\x1b[3~"),
        KeyCode::Esc => out.push(0x1b),
        KeyCode::Home => out.extend_from_slice(b"\x1b[H"),
        KeyCode::End => out.extend_from_slice(b"\x1b[F"),
        KeyCode::PageUp => out.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => out.extend_from_slice(b"\x1b[6~"),
        KeyCode::Left => {
            if alt {
                out.extend_from_slice(b"\x1bb");
            } else {
                out.extend_from_slice(b"\x1b[D");
            }
        }
        KeyCode::Right => {
            if alt {
                out.extend_from_slice(b"\x1bf");
            } else {
                out.extend_from_slice(b"\x1b[C");
            }
        }
        KeyCode::Up => out.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => out.extend_from_slice(b"\x1b[B"),
        KeyCode::Insert => out.extend_from_slice(b"\x1b[2~"),
        KeyCode::F(n) => {
            let code = match n {
                1 => b'P',
                2 => b'Q',
                3 => b'R',
                4 => b'S',
                _ => b'P',
            };
            out.extend_from_slice(&[0x1b, b'[', code]);
        }
        KeyCode::Null => out.push(0),
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_ctrl_maps_to_control_byte() {
        let k = KeyEvent::new(
            KeyCode::Char('c'),
            crossterm::event::KeyModifiers::CONTROL,
        );
        assert_eq!(key_to_bytes(k), vec![0x03]);
    }

    #[test]
    fn super_cmd_keys_never_reach_the_pty() {

        let cmd_c = KeyEvent::new(
            KeyCode::Char('c'),
            crossterm::event::KeyModifiers::SUPER,
        );
        assert!(key_to_bytes(cmd_c).is_empty(), "Cmd+C must not reach the pty");
        let cmd_v = KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::SUPER,
        );
        assert!(key_to_bytes(cmd_v).is_empty(), "Cmd+V must not reach the pty");
    }

    #[test]
    fn plain_char_utf8() {
        let k = KeyEvent::new(KeyCode::Char('é'), crossterm::event::KeyModifiers::NONE);
        assert_eq!(key_to_bytes(k), "é".as_bytes().to_vec());
    }

    #[test]
    fn arrows_and_page_keys() {
        let up = KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::NONE);
        assert_eq!(key_to_bytes(up), b"\x1b[A");
        let pgup = KeyEvent::new(KeyCode::PageUp, crossterm::event::KeyModifiers::NONE);
        assert_eq!(key_to_bytes(pgup), b"\x1b[5~");
        let bs = KeyEvent::new(KeyCode::Backspace, crossterm::event::KeyModifiers::NONE);
        assert_eq!(key_to_bytes(bs), vec![0x7f]);
    }

    #[test]
    fn test_shift_enter_and_modified_keys() {
        let plain_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        assert_eq!(key_to_bytes(plain_enter), b"\r");

        let shift_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::SHIFT);
        assert_eq!(key_to_bytes(shift_enter), b"\x1b[13;2u");

        let alt_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::ALT);
        assert_eq!(key_to_bytes(alt_enter), b"\x1b\r");

        let ctrl_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::CONTROL);
        assert_eq!(key_to_bytes(ctrl_enter), b"\n");

        let shift_tab = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::SHIFT);
        assert_eq!(key_to_bytes(shift_tab), b"\x1b[Z");
    }
}

