use crate::state::AppState;
use crate::ui::modal::model::{Modal, ModalRow};

pub fn open_sounds_modal(state: &mut AppState) {
    let prefs = crate::prefs::Prefs::load();
    let local_path = crate::prefs::Prefs::local_path();
    open_sounds_modal_with_prefs(state, prefs.sounds, local_path);
}

pub fn open_sounds_modal_with_prefs(
    state: &mut AppState,
    sounds: crate::prefs::SoundPrefs,
    persist_path: Option<std::path::PathBuf>,
) {
    let mut m = Modal::new("sounds_config", "Sound Alerts & Notifications");
    if let Some(path) = persist_path {
        if !path.as_os_str().is_empty() {
            m.persist_config = Some(path);
        }
    }

    let completed_opts: Vec<(String, String, String)> = crate::sound::available_completed_sounds()
        .iter()
        .map(|(v, l, d)| (v.to_string(), l.to_string(), d.to_string()))
        .collect();

    let completed_idx = completed_opts
        .iter()
        .position(|(val, _, _)| val.eq_ignore_ascii_case(&sounds.sound_completed))
        .unwrap_or(0);

    let blocked_opts: Vec<(String, String, String)> = crate::sound::available_blocked_sounds()
        .iter()
        .map(|(v, l, d)| (v.to_string(), l.to_string(), d.to_string()))
        .collect();

    let blocked_idx = blocked_opts
        .iter()
        .position(|(val, _, _)| val.eq_ignore_ascii_case(&sounds.sound_blocked))
        .unwrap_or(0);

    let rows = vec![
        ModalRow::Separator("Audio Alerts Configuration".into()),
        ModalRow::Info("Play audio cues on full task completion or when agent needs user intervention.".into()),
        ModalRow::Toggle {
            key: "sounds.enabled".into(),
            label: "Enable Sound Alerts".into(),
            enabled: sounds.enabled,
        },
        ModalRow::Separator("Sound Effects".into()),
        ModalRow::Choice {
            key: "sounds.sound_completed".into(),
            label: "Task Completed Sound".into(),
            options: completed_opts,
            current: completed_idx,
            searchable: false,
            color: "green".into(),
        },
        ModalRow::Choice {
            key: "sounds.sound_blocked".into(),
            label: "Needs Attention Sound".into(),
            options: blocked_opts,
            current: blocked_idx,
            searchable: false,
            color: "yellow".into(),
        },
        ModalRow::Separator("Test & Preview".into()),
        ModalRow::Nav {
            key: "sounds.test_completed".into(),
            label: "> Test Completion Sound".into(),
            color: "accent".into(),
        },
        ModalRow::Nav {
            key: "sounds.test_blocked".into(),
            label: "> Test Attention Sound".into(),
            color: "accent".into(),
        },
    ];

    m.add_step("1. Settings", rows);
    m.commands.push(("back".into(), "Back".into()));
    m.commands.push(("refresh".into(), "Save".into()));
    state.active_modal = Some(m);
}

pub fn handle_sounds_modal_enter(state: &mut AppState) {
    let Some(modal) = state.active_modal.as_mut() else {
        return;
    };
    let idx = modal.selected;
    let row = match modal.rows.get(idx) {
        Some(r) => r.clone(),
        None => return,
    };

    match row {
        crate::ui::modal::model::ModalRow::Nav { ref key, .. } => {
            let prefs = crate::prefs::Prefs::load();
            if key == "sounds.test_completed" {
                let sound = modal
                    .rows
                    .iter()
                    .find_map(|r| {
                        if let crate::ui::modal::model::ModalRow::Choice { key, options, current, .. } = r {
                            if key == "sounds.sound_completed" {
                                return options.get(*current).map(|(v, _, _)| v.clone());
                            }
                        }
                        None
                    })
                    .unwrap_or(prefs.sounds.sound_completed);
                crate::sound::play_preview(&sound);
            } else if key == "sounds.test_blocked" {
                let sound = modal
                    .rows
                    .iter()
                    .find_map(|r| {
                        if let crate::ui::modal::model::ModalRow::Choice { key, options, current, .. } = r {
                            if key == "sounds.sound_blocked" {
                                return options.get(*current).map(|(v, _, _)| v.clone());
                            }
                        }
                        None
                    })
                    .unwrap_or(prefs.sounds.sound_blocked);
                crate::sound::play_preview(&sound);
            }
        }
        crate::ui::modal::model::ModalRow::Choice { ref options, current, .. } => {
            modal.cycle_selected();
            let next_idx = (current + 1) % options.len();
            if let Some((sound_val, _, _)) = options.get(next_idx) {
                crate::sound::play_preview(sound_val);
            }
            crate::mux_core::modals::sync_modal_toggles(state);
        }
        crate::ui::modal::model::ModalRow::Toggle { .. } => {
            modal.cycle_selected();
            crate::mux_core::modals::sync_modal_toggles(state);
        }
        _ => {
            crate::mux_core::modals::sync_modal_toggles(state);
            state.active_modal = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::SoundPrefs;

    #[test]
    fn sounds_modal_initializes_with_prefs() {
        let prefs = SoundPrefs {
            enabled: false,
            sound_completed: "Ping".into(),
            sound_blocked: "Basso".into(),
        };
        let sidebar = crate::ui::sidebar::Sidebar::load();
        let mut state = AppState::new(sidebar);
        open_sounds_modal_with_prefs(&mut state, prefs, None);

        let modal = state.active_modal.as_ref().unwrap();
        assert_eq!(modal.id, "sounds_config");

        let toggle_enabled = modal.rows.iter().find_map(|r| {
            if let ModalRow::Toggle { key, enabled, .. } = r {
                if key == "sounds.enabled" {
                    return Some(*enabled);
                }
            }
            None
        });
        assert_eq!(toggle_enabled, Some(false));

        let completed_choice = modal.rows.iter().find_map(|r| {
            if let ModalRow::Choice { key, options, current, .. } = r {
                if key == "sounds.sound_completed" {
                    return options.get(*current).map(|(v, _, _)| v.as_str());
                }
            }
            None
        });
        assert_eq!(completed_choice, Some("Ping"));

        let blocked_choice = modal.rows.iter().find_map(|r| {
            if let ModalRow::Choice { key, options, current, .. } = r {
                if key == "sounds.sound_blocked" {
                    return options.get(*current).map(|(v, _, _)| v.as_str());
                }
            }
            None
        });
        assert_eq!(blocked_choice, Some("Basso"));
    }

    #[test]
    fn sounds_modal_enter_toggles_and_cycles() {
        let prefs = SoundPrefs::default();
        let sidebar = crate::ui::sidebar::Sidebar::load();
        let mut state = AppState::new(sidebar);
        open_sounds_modal_with_prefs(&mut state, prefs, None);

        let modal = state.active_modal.as_mut().unwrap();
        let toggle_idx = modal.rows.iter().position(|r| matches!(r, ModalRow::Toggle { key, .. } if key == "sounds.enabled")).unwrap();
        modal.selected = toggle_idx;

        handle_sounds_modal_enter(&mut state);
        let modal = state.active_modal.as_ref().unwrap();
        let toggle_val = modal.rows.iter().find_map(|r| {
            if let ModalRow::Toggle { key, enabled, .. } = r {
                if key == "sounds.enabled" {
                    return Some(*enabled);
                }
            }
            None
        });
        assert_eq!(toggle_val, Some(false));
    }
}
