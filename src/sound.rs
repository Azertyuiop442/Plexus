use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_SOUND_MS: AtomicU64 = AtomicU64::new(0);
const GLOBAL_COOLDOWN_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundAlertKind {
    TaskCompleted,
    AgentBlocked,
}

pub fn available_completed_sounds() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        ("Glass", "Glass", "Clean crystalline chime (default)"),
        ("Ping", "Ping", "Clear notification ping"),
        ("Hero", "Hero", "Triumphant chime"),
        ("Pop", "Pop", "Subtle bubble pop"),
        ("Tink", "Tink", "Minimal sharp tick"),
        ("None", "None (Mute)", "Do not play a sound on completion"),
    ]
}

pub fn available_blocked_sounds() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        ("Sosumi", "Sosumi", "Classic alert tone (default)"),
        ("Basso", "Basso", "Deep resonant warning"),
        ("Submarine", "Submarine", "Sonar ping alert"),
        ("Funk", "Funk", "Snappy attention tap"),
        ("None", "None (Mute)", "Do not play a sound when blocked"),
    ]
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn check_and_update_cooldown() -> bool {
    let now = now_ms();
    let last = LAST_SOUND_MS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < GLOBAL_COOLDOWN_MS {
        return false;
    }
    LAST_SOUND_MS.store(now, Ordering::Relaxed);
    true
}

pub fn play_sound(kind: SoundAlertKind, prefs: &crate::prefs::SoundPrefs) {
    if !prefs.enabled {
        return;
    }

    let sound_name = match kind {
        SoundAlertKind::TaskCompleted => &prefs.sound_completed,
        SoundAlertKind::AgentBlocked => &prefs.sound_blocked,
    };

    if sound_name.eq_ignore_ascii_case("None") || sound_name.is_empty() {
        return;
    }

    if !check_and_update_cooldown() {
        return;
    }

    spawn_play(sound_name.to_string());
}

pub fn play_preview(sound_name: &str) {
    if sound_name.eq_ignore_ascii_case("None") || sound_name.is_empty() {
        return;
    }
    spawn_play(sound_name.to_string());
}

fn spawn_play(sound: String) {
    std::thread::spawn(move || {
        play_platform_sound(&sound);
    });
}

fn play_platform_sound(sound: &str) {
    #[cfg(target_os = "macos")]
    {
        let path = format!("/System/Library/Sounds/{sound}.aiff");
        let _ = Command::new("afplay")
            .arg(&path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = match sound.to_lowercase().as_str() {
            "sosumi" | "basso" | "submarine" | "funk" => {
                "[System.Media.SystemSounds]::Exclamation.Play()"
            }
            _ => "[System.Media.SystemSounds]::Asterisk.Play()",
        };
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", ps_cmd])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let sound_path = match sound.to_lowercase().as_str() {
            "sosumi" | "basso" | "submarine" | "funk" => {
                "/usr/share/sounds/freedesktop/stereo/dialog-warning.oga"
            }
            _ => "/usr/share/sounds/freedesktop/stereo/complete.oga",
        };

        let paplay = Command::new("paplay")
            .arg(sound_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if paplay.is_err() {
            let _ = Command::new("pw-play")
                .arg(sound_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_sounds_list_has_options() {
        let list = available_completed_sounds();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(k, _, _)| *k == "Glass"));
        assert!(list.iter().any(|(k, _, _)| *k == "None"));
    }

    #[test]
    fn blocked_sounds_list_has_options() {
        let list = available_blocked_sounds();
        assert!(!list.is_empty());
        assert!(list.iter().any(|(k, _, _)| *k == "Sosumi"));
        assert!(list.iter().any(|(k, _, _)| *k == "None"));
    }

    #[test]
    fn cooldown_suppresses_rapid_triggers() {
        LAST_SOUND_MS.store(0, Ordering::Relaxed);
        assert!(check_and_update_cooldown());
        assert!(!check_and_update_cooldown());
    }
}
