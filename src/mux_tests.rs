
#[cfg(test)]
mod tests {
    use crate::*;
    use crate::mux_core::modals::open_context_modal;
    use crate::state::AppState;
    use crate::ui::pane::MuxPane;
    use crate::ui::sidebar::{Sidebar, SettingsSubMenu};
    use ratatui::backend::TestBackend;
    use std::io::Write as _;

    #[test]
    fn pane_routing_by_generation_survives_tab_closure_and_index_shifts() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.mods_data = crate::ui::mod_bridge::ModsData::default();

        let (pane0, r0) = match MuxPane::spawn("true", 80, 24) {
            Ok(p) => p,
            Err(_) => return,
        };
        let (pane1, r1) = match MuxPane::spawn("true", 80, 24) {
            Ok(p) => p,
            Err(_) => return,
        };
        drop(r0);
        drop(r1);
        pane0.lock().unwrap_or_else(|e| e.into_inner()).state.gen = 10;
        pane1.lock().unwrap_or_else(|e| e.into_inner()).state.gen = 20;
        state.panes.push(pane0);
        state.panes.push(pane1);

        assert!(find_pane_by_gen(&state, 10).is_some());
        assert!(find_pane_by_gen(&state, 20).is_some());
        assert!(find_pane_by_gen(&state, 99).is_none());

        state.close_pane(0);
        assert_eq!(state.panes.len(), 1);

        let found = find_pane_by_gen(&state, 20);
        assert!(found.is_some(), "Pane 1 must still receive events after pane 0 is closed");
        assert_eq!(found.unwrap().lock().unwrap().state.gen, 20);

        assert!(find_pane_by_gen(&state, 10).is_none(), "Old pane 0 events must be dropped");
    }

    #[test]
    fn startup_render_does_not_panic() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.mods_data = crate::ui::mod_bridge::ModsData::load();

        let (pane, reader) = match MuxPane::spawn("true", 100, 30) {
            Ok(p) => p,
            Err(_) => return,
        };
        drop(reader);
        state.panes.push(pane);
        state.active = 0;

        open_context_modal(&mut state);

        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| crate::ui::render(f, &mut state))
            .expect("first draw must not panic");
    }

    #[test]
    fn render_stays_fast_with_full_pane_content() {

        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.mods_data = crate::ui::mod_bridge::ModsData::default();

        let (pane, reader) = match MuxPane::spawn("true", 120, 40) {
            Ok(p) => p,
            Err(_) => return,
        };
        drop(reader);
        {
            let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
            let mut out = Vec::new();
            for i in 0..60 {
                writeln!(
                    &mut out,
                    "line {:03} abcdefghijklmnopqrstuvwxyz0123456789",
                    i
                )
                .unwrap();
            }
            p.feed(&out);
        }
        state.panes.push(pane);
        state.active = 0;

        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| crate::ui::render(f, &mut state)).unwrap();
        let mut total = std::time::Duration::ZERO;
        let n = 20;
        for _ in 0..n {
            let t0 = std::time::Instant::now();
            terminal.draw(|f| crate::ui::render(f, &mut state)).unwrap();
            total += t0.elapsed();
        }
        let avg = total / n;
        assert!(
            avg < std::time::Duration::from_millis(20),
            "render too slow: avg {avg:?}"
        );
    }

    #[test]
    fn render_with_empty_data_does_not_panic() {
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
            taste_learning: true,
            ide_context: true,
            show_cost_bar: true,
            show_context_btn: true,
            show_usage: true,
            live_blocks: Vec::new(),
            available_update: None,
            usage: None,
            usage_tab: 0,
        };
        let mut state = AppState::new(sidebar);
        state.mods_data = crate::ui::mod_bridge::ModsData::default();

        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| crate::ui::render(f, &mut state))
            .expect("draw with empty state must not panic");
    }

    #[test]
    fn render_does_not_touch_disk_caches() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.mods_data = crate::ui::mod_bridge::ModsData::load();

        let (pane, reader) = match MuxPane::spawn("true", 100, 30) {
            Ok(p) => p,
            Err(_) => return,
        };
        drop(reader);
        state.panes.push(pane);
        state.active = 0;

        state.refresh_model_cache();

        let model_before = state.model_info();
        let model_tick_before = state.last_model_check;

        let backend = TestBackend::new(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        for _ in 0..3 {
            terminal
                .draw(|f| crate::ui::render(f, &mut state))
                .expect("draw must not panic");
        }

        assert_eq!(state.model_info(), model_before);
        assert_eq!(state.last_model_check, model_tick_before);
    }

    #[test]
    fn pane_click_path_does_not_relock_the_pane() {

        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/mux_core/mouse.rs"
        ))
        .unwrap();
        assert!(
            !src.contains("let (pane_w, pane_h) = active_pane_size(state)"),
            "pane-click path must read the size from the held guard, not relock"
        );
    }

    #[test]
    fn render_survives_tiny_terminal_sizes() {
        for (w, h) in [(20u16, 5u16), (30, 8), (50, 12), (1, 1)] {
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
                taste_learning: true,
                ide_context: true,
                show_cost_bar: true,
                show_context_btn: true,
                show_usage: true,
                live_blocks: Vec::new(),
                available_update: None,
                usage: None,
                usage_tab: 0,
            };
            let mut state = AppState::new(sidebar);
            let (pane, reader) = match MuxPane::spawn(
                "true",
                w.saturating_sub(3).max(5),
                h.saturating_sub(3).max(3),
            ) {
                Ok(p) => p,
                Err(_) => continue,
            };
            drop(reader);
            state.panes.push(pane);
            state.active = 0;

            let backend = TestBackend::new(w, h);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            terminal
                .draw(|f| crate::ui::render(f, &mut state))
                .unwrap_or_else(|e| panic!("draw at {}x{} must not panic: {}", w, h, e));
        }
    }
}

