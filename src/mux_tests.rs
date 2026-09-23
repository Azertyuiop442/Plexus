
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
            avg < std::time::Duration::from_millis(35),
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
            skill_injection: false,
            taste_learning: true,
            ide_context: true,
            show_cost_bar: true,
            show_context_btn: true,
            show_usage: true,
            sound_notifications: true,
            auto_retry_enabled: true,
            webhook_enabled: false,
            skills_update_count: 0,
            live_blocks: Vec::new(),
            available_update: None,
            usage: None,
            usage_tab: 0,
            panels: Vec::new(),
            session_marks: std::collections::HashMap::new(),
            active_panel: None,
            panel_open: false,
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
                skill_injection: false,
                taste_learning: true,
                ide_context: true,
                show_cost_bar: true,
                show_context_btn: true,
                show_usage: true,
                sound_notifications: true,
                auto_retry_enabled: true,
                webhook_enabled: false,
                skills_update_count: 0,
                live_blocks: Vec::new(),
                available_update: None,
                usage: None,
                usage_tab: 0,
                panels: Vec::new(),
                session_marks: std::collections::HashMap::new(),
                active_panel: None,
                panel_open: false,
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

    #[test]
    fn live_session_model_overrides_bridge_model() {
        let _guard = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let home = std::env::temp_dir().join(format!("cc-live-model-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".commandcode/projects/myproj")).unwrap();
        std::fs::write(
            home.join(".commandcode/projects/myproj/sid-1.meta.json"),
            r#"{ "model": "z-ai/glm-5.3-flash" }"#,
        )
        .unwrap();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &home);

        let restore_home = || match old_home.clone() {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        };

        let mut state = AppState::new(Sidebar {
            project: "myproj".to_string(),
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
            sound_notifications: true,
            auto_retry_enabled: true,
            webhook_enabled: false,
            skills_update_count: 0,
            live_blocks: Vec::new(),
            available_update: None,
            usage: None,
            usage_tab: 0,
            panels: Vec::new(),
            session_marks: std::collections::HashMap::new(),
            active_panel: None,
            panel_open: false,
        });

        let (pane, reader) = match MuxPane::spawn("true", 80, 24) {
            Ok(p) => p,
            Err(_) => {
                restore_home();
                let _ = std::fs::remove_dir_all(&home);
                return;
            }
        };
        drop(reader);
        pane.lock().unwrap_or_else(|e| e.into_inner()).state.session_id =
            Some("sid-1".to_string());
        state.panes.push(pane);
        state.active = 0;

        let mut data = crate::ui::mod_bridge::contract::ModData::default();
        data.model_id = "old/old-model".to_string();
        state.mods_data = crate::ui::mod_bridge::ModsData {
            mods: vec![crate::ui::mod_bridge::contract::ModLiveData {
                id: "bridge".to_string(),
                data,
            }],
            known_mod_ids: Default::default(),
        };

        state.last_model_check = std::time::Instant::now() - std::time::Duration::from_secs(10);
        state.refresh_model_cache();

        let (model, _) = state.model_info();
        restore_home();
        let _ = std::fs::remove_dir_all(&home);

        assert_eq!(
            model.as_deref(),
            Some("z-ai/glm-5.3-flash"),
            "live session meta model must beat the mods bridge"
        );
    }

    #[test]
    fn collapsed_sidebar_renders_grip_symbol_in_center() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = false;

        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| crate::ui::render(f, &mut state))
            .unwrap();

        let buf = terminal.backend().buffer();
        let mid_y = 20;

        assert_eq!(buf[(0, mid_y - 1)].symbol(), "║");
        assert_eq!(buf[(0, mid_y)].symbol(), "║");
        assert_eq!(buf[(0, mid_y + 1)].symbol(), "║");

        assert_eq!(buf[(0, 5)].symbol(), "│");
        assert_eq!(buf[(0, 35)].symbol(), "│");
    }

    #[test]
    fn adaptive_sidebar_renders_abbreviations_on_intermediate_width() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = true;
        state.sidebar_w = 14;

        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| crate::ui::render(f, &mut state))
            .unwrap();

        let buf = terminal.backend().buffer();
        assert_eq!(buf[(13, 0)].symbol(), "╮");
        assert_eq!(buf[(13, 39)].symbol(), "╯");
    }

    #[test]
    fn adaptive_sidebar_renders_icons_only_on_narrow_width() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = true;
        state.sidebar_w = 8;

        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| crate::ui::render(f, &mut state))
            .unwrap();

        let buf = terminal.backend().buffer();
        assert_eq!(buf[(7, 0)].symbol(), "╮");
        assert_eq!(buf[(7, 39)].symbol(), "╯");
    }

    #[test]
    fn drag_sidebar_to_collapse_does_not_reopen_on_mouse_up() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = true;
        state.sidebar_w = 25;

        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let terminal = ratatui::Terminal::new(backend).unwrap();

        let mouse_down = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 25,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_down, "true", "true");
        assert!(state.resizing_sidebar);
        assert!(!state.resizing_sidebar_dragged);

        let mouse_drag = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
            column: 2,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_drag, "true", "true");
        assert!(!state.sidebar_open);
        assert!(state.resizing_sidebar_dragged);

        let mouse_up = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            column: 1,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_up, "true", "true");
        assert!(!state.sidebar_open);
        assert!(!state.resizing_sidebar);
        assert!(!state.resizing_sidebar_dragged);
    }

    #[test]
    fn click_collapsed_sidebar_rail_reopens_sidebar() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = false;
        state.sidebar_w = 25;

        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let terminal = ratatui::Terminal::new(backend).unwrap();

        let mouse_down = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_down, "true", "true");
        assert!(state.resizing_sidebar);
        assert!(!state.resizing_sidebar_dragged);

        let mouse_up = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            column: 0,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_up, "true", "true");
        assert!(state.sidebar_open);
        assert_eq!(state.sidebar_w, 25);
        assert!(state.sidebar_focus);
        assert!(!state.resizing_sidebar);
        assert!(!state.resizing_sidebar_dragged);
    }

    #[test]
    fn drag_collapsed_sidebar_rail_expands_to_target_width() {
        let sidebar = Sidebar::load();
        let mut state = AppState::new(sidebar);
        state.sidebar_open = false;

        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let terminal = ratatui::Terminal::new(backend).unwrap();

        let mouse_down = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_down, "true", "true");

        let mouse_drag = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
            column: 32,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_drag, "true", "true");
        assert!(state.sidebar_open);
        assert_eq!(state.sidebar_w, 32);
        assert!(state.resizing_sidebar_dragged);

        let mouse_up = crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
            column: 32,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let _ = crate::mux_core::input::handle_mouse(&mut state, &terminal, mouse_up, "true", "true");
        assert!(state.sidebar_open);
        assert_eq!(state.sidebar_w, 32);
        assert!(!state.resizing_sidebar);
    }

    #[test]
    fn narrow_sidebar_border_and_icon_alignment() {
        let mut sidebar = Sidebar::load();
        sidebar.sessions.clear();
        sidebar.sessions.push(crate::ui::sidebar::SessionEntry {
            id: "s1".to_string(),
            title: "Test Session".to_string(),
            last_at: 0,
            date: String::new(),
            age: String::new(),
            age_short: "1m".to_string(),
        });
        sidebar.usage = Some(crate::usage::UsageData {
            monthly_remaining: 100.0,
            purchased_remaining: 0.0,
            free_remaining: 0.0,
            monthly_allowance: 100.0,
            plan_name: "Pro".to_string(),
            plan_id: "pro".to_string(),
            current_period_end: None,
            five_hour: None,
            weekly: None,
            last_updated_secs: 0,
        });
        sidebar.show_usage = true;
        sidebar.rebuild_rows();

        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut view = crate::ui::sidebar::models::SidebarView::default();
        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();

        terminal
            .draw(|f| {
                crate::ui::sidebar::render_sidebar(
                    f,
                    ratatui::layout::Rect::new(0, 0, 6, 24),
                    &sidebar,
                    &mut view,
                    false,
                    &panes,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(1, 0)].symbol(), "─");
        assert_eq!(buffer[(2, 0)].symbol(), "─");
        assert_eq!(buffer[(3, 0)].symbol(), "─");
        assert_eq!(buffer[(4, 0)].symbol(), "─");

        for y in 1..=3 {
            assert_eq!(buffer[(0, y)].symbol(), "│");
            assert_ne!(buffer[(2, y)].symbol(), " ");
        }
    }

    #[test]
    fn dock_button_activates_deactivates_and_retracts() {
        let mut sidebar = Sidebar::load();
        sidebar.panels.push(crate::ui::sidebar::models::SidebarPanelItem {
            icon: "nf-dev-git".to_string(),
            title: "git".to_string(),
        });
        sidebar.rebuild_rows();
        let mut state = AppState::new(sidebar);
        assert!(!state.panel_active);
        assert!(!state.panel_sidebar_open);
        assert_eq!(state.sidebar.active_panel, None);

        crate::mux_core::mouse_sidebar::handle_sidebar_click(
            &mut state,
            crate::ui::sidebar::models::SidebarRow::RightSidebar(0),
            "true",
            "true",
        )
        .unwrap();
        assert!(state.panel_active);
        assert!(state.panel_sidebar_open);
        assert_eq!(state.sidebar.active_panel, Some(0));
        assert!(state.sidebar.panel_open);

        state.panel_sidebar_open = false;
        state.sync_sidebar_panel();
        assert!(state.panel_active);
        assert!(!state.panel_sidebar_open);
        assert_eq!(state.sidebar.active_panel, Some(0));
        assert!(!state.sidebar.panel_open);

        crate::mux_core::mouse_sidebar::handle_sidebar_click(
            &mut state,
            crate::ui::sidebar::models::SidebarRow::RightSidebar(0),
            "true",
            "true",
        )
        .unwrap();
        assert!(state.panel_active);
        assert!(state.panel_sidebar_open);
        assert!(state.sidebar.panel_open);

        crate::mux_core::mouse_sidebar::handle_sidebar_click(
            &mut state,
            crate::ui::sidebar::models::SidebarRow::RightSidebar(0),
            "true",
            "true",
        )
        .unwrap();
        assert!(!state.panel_active);
        assert!(!state.panel_sidebar_open);
        assert_eq!(state.sidebar.active_panel, None);
        assert!(!state.sidebar.panel_open);
    }

    #[test]
    fn dock_layout_expanded_collapsed_and_deactivated() {
        let backend = ratatui::backend::TestBackend::new(100, 30);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        let mut sidebar = Sidebar::load();
        sidebar.panels.push(crate::ui::sidebar::models::SidebarPanelItem {
            icon: "nf-dev-git".to_string(),
            title: "git".to_string(),
        });
        sidebar.rebuild_rows();
        let mut state = AppState::new(sidebar);
        let mut live = crate::ui::mod_bridge::contract::ModLiveData::default();
        live.data.panels.push(crate::ui::mod_bridge::contract::ModPanel {
            id: "git".to_string(),
            title: "git".to_string(),
            icon: "nf-dev-git".to_string(),
            ..Default::default()
        });
        state.mods_data.mods.push(live);

        term.draw(|f| crate::ui::render(f, &mut state)).unwrap();
        let b = term.backend().buffer();
        assert_ne!(b[(99, 15)].symbol(), "║");

        state.panel_active = true;
        state.panel_sidebar_open = true;
        state.panel_sidebar_w = 25;
        term.draw(|f| crate::ui::render(f, &mut state)).unwrap();
        let b2 = term.backend().buffer();
        let panel_left = 100 - 25;
        assert_eq!(b2[(panel_left, 15)].symbol(), "║");

        state.panel_sidebar_open = false;
        term.draw(|f| crate::ui::render(f, &mut state)).unwrap();
        let b3 = term.backend().buffer();
        assert_eq!(b3[(99, 15)].symbol(), "║");

        state.panel_active = false;
        term.draw(|f| crate::ui::render(f, &mut state)).unwrap();
        let b4 = term.backend().buffer();
        assert_ne!(b4[(99, 15)].symbol(), "║");
    }

    #[test]
    fn usage_carousel_tiers_without_ascii_bar_when_narrow() {
        let mut sidebar = Sidebar::load();
        sidebar.usage = Some(crate::usage::UsageData {
            monthly_remaining: 100.0,
            purchased_remaining: 0.0,
            free_remaining: 0.0,
            monthly_allowance: 100.0,
            plan_name: "Pro".to_string(),
            plan_id: "pro".to_string(),
            current_period_end: None,
            five_hour: Some(crate::usage::WindowLimit {
                used: 28.0,
                cap: 100.0,
                exceeded: false,
                reset_at: 0,
            }),
            weekly: None,
            last_updated_secs: 0,
        });
        sidebar.show_usage = true;
        sidebar.usage_tab = 0;
        sidebar.rebuild_rows();

        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();

        let backend6 = TestBackend::new(80, 24);
        let mut term6 = ratatui::Terminal::new(backend6).unwrap();
        let mut view6 = crate::ui::sidebar::models::SidebarView::default();
        term6
            .draw(|f| {
                crate::ui::sidebar::render_sidebar(
                    f,
                    ratatui::layout::Rect::new(0, 0, 6, 24),
                    &sidebar,
                    &mut view6,
                    false,
                    &panes,
                );
            })
            .unwrap();
        let b6 = term6.backend().buffer();
        let mut text6 = String::new();
        for y in 0..24 {
            for x in 0..6 {
                text6.push_str(b6[(x, y)].symbol());
            }
        }
        assert!(text6.contains("28%"));
        assert!(!text6.contains("5h"));

        let backend10 = TestBackend::new(80, 24);
        let mut term10 = ratatui::Terminal::new(backend10).unwrap();
        let mut view10 = crate::ui::sidebar::models::SidebarView::default();
        term10
            .draw(|f| {
                crate::ui::sidebar::render_sidebar(
                    f,
                    ratatui::layout::Rect::new(0, 0, 10, 24),
                    &sidebar,
                    &mut view10,
                    false,
                    &panes,
                );
            })
            .unwrap();
        let b10 = term10.backend().buffer();
        let mut text10 = String::new();
        for y in 0..24 {
            for x in 0..10 {
                text10.push_str(b10[(x, y)].symbol());
            }
        }
        assert!(text10.contains("5h"));
        assert!(text10.contains("28%"));
        assert!(!text10.contains("█"));
        assert!(!text10.contains("░"));

        let backend14 = TestBackend::new(80, 24);
        let mut term14 = ratatui::Terminal::new(backend14).unwrap();
        let mut view14 = crate::ui::sidebar::models::SidebarView::default();
        term14
            .draw(|f| {
                crate::ui::sidebar::render_sidebar(
                    f,
                    ratatui::layout::Rect::new(0, 0, 14, 24),
                    &sidebar,
                    &mut view14,
                    false,
                    &panes,
                );
            })
            .unwrap();
        let b14 = term14.backend().buffer();
        let mut text14 = String::new();
        for y in 0..24 {
            for x in 0..14 {
                text14.push_str(b14[(x, y)].symbol());
            }
        }
        assert!(text14.contains("5h"));
        assert!(text14.contains("28%"));
        assert!(text14.contains("‹"));
        assert!(text14.contains("›"));
        assert!(!text14.contains("█"));
        assert!(!text14.contains("░"));

        let backend25 = TestBackend::new(80, 24);
        let mut term25 = ratatui::Terminal::new(backend25).unwrap();
        let mut view25 = crate::ui::sidebar::models::SidebarView::default();
        term25
            .draw(|f| {
                crate::ui::sidebar::render_sidebar(
                    f,
                    ratatui::layout::Rect::new(0, 0, 25, 24),
                    &sidebar,
                    &mut view25,
                    false,
                    &panes,
                );
            })
            .unwrap();
        let b25 = term25.backend().buffer();
        let mut text25 = String::new();
        for y in 0..24 {
            for x in 0..25 {
                text25.push_str(b25[(x, y)].symbol());
            }
        }
        assert!(text25.contains("5h"));
        assert!(text25.contains("28%"));
        assert!(text25.contains("█") || text25.contains("░"));
    }

    #[test]
    fn right_sidebar_section_conditional_rendering() {
        let mut sidebar = Sidebar::load();
        sidebar.panels.clear();
        sidebar.rebuild_rows();

        let backend = TestBackend::new(80, 24);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        let mut view = crate::ui::sidebar::models::SidebarView::default();
        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();

        term.draw(|f| {
            crate::ui::sidebar::render_sidebar(
                f,
                ratatui::layout::Rect::new(0, 0, 25, 24),
                &sidebar,
                &mut view,
                false,
                &panes,
            );
        })
        .unwrap();

        assert!(!view.row_y.iter().any(|(_, r)| matches!(r, crate::ui::sidebar::models::SidebarRow::RightSidebar(_))));

        sidebar.panels.push(crate::ui::sidebar::models::SidebarPanelItem {
            icon: "nf-dev-git".to_string(),
            title: "git".to_string(),
        });
        sidebar.rebuild_rows();

        let mut view_with_panels = crate::ui::sidebar::models::SidebarView::default();
        term.draw(|f| {
            crate::ui::sidebar::render_sidebar(
                f,
                ratatui::layout::Rect::new(0, 0, 25, 24),
                &sidebar,
                &mut view_with_panels,
                false,
                &panes,
            );
        })
        .unwrap();

        assert!(view_with_panels.row_y.iter().any(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::RightSidebar(0)));
        let b = term.backend().buffer();
        let mut text = String::new();
        for y in 0..24 {
            for x in 0..25 {
                text.push_str(b[(x, y)].symbol());
            }
        }
        assert!(text.contains("git"));
        let git_glyph = nf_icons::nf!("nf-dev-git");
        assert!(text.contains(git_glyph));
    }

    #[test]
    fn narrow_sidebar_progressive_degradation_and_bottom_icons() {
        let mut sidebar = Sidebar::load();
        sidebar.panels.clear();
        sidebar.panels.push(crate::ui::sidebar::models::SidebarPanelItem {
            icon: "nf-dev-git".to_string(),
            title: "git".to_string(),
        });
        sidebar.rebuild_rows();

        let backend8 = TestBackend::new(80, 24);
        let mut term8 = ratatui::Terminal::new(backend8).unwrap();
        let mut view8 = crate::ui::sidebar::models::SidebarView::default();
        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();

        term8.draw(|f| {
            crate::ui::sidebar::render_sidebar(
                f,
                ratatui::layout::Rect::new(0, 0, 8, 24),
                &sidebar,
                &mut view8,
                false,
                &panes,
            );
        }).unwrap();

        let b8 = term8.backend().buffer();
        let mut text8 = String::new();
        for y in 0..24 {
            for x in 0..8 {
                text8.push_str(b8[(x, y)].symbol());
            }
        }
        assert!(text8.contains("›"));
        assert!(text8.contains("P"));
        assert!(view8.zones.iter().any(|z| z.row == crate::ui::sidebar::models::SidebarRow::Reload));
        assert!(view8.zones.iter().any(|z| z.row == crate::ui::sidebar::models::SidebarRow::BugReport));
    }

    struct SidebarDirGuard {
        old: Option<std::ffi::OsString>,
        dir: std::path::PathBuf,
    }

    impl SidebarDirGuard {
        fn new(sessions_json: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("cc-sidebar-hit-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("sessions.json"), sessions_json).unwrap();
            let old = std::env::var_os("CC_SIDEBAR_DIR");
            std::env::set_var("CC_SIDEBAR_DIR", &dir);
            Self { old, dir }
        }
    }

    impl Drop for SidebarDirGuard {
        fn drop(&mut self) {
            match &self.old {
                Some(v) => std::env::set_var("CC_SIDEBAR_DIR", v),
                None => std::env::remove_var("CC_SIDEBAR_DIR"),
            }
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    const FOUR_SESSIONS: &str = r#"{
        "project": "proj",
        "cwd": "/tmp/proj",
        "sessions": [
            { "id": "s-one", "title": "Session One", "lastAt": 40, "ageShort": "1m" },
            { "id": "s-two", "title": "Session Two", "lastAt": 30, "ageShort": "2m" },
            { "id": "s-three", "title": "Session Three", "lastAt": 20, "ageShort": "3m" }
        ]
    }"#;

    fn draw_sidebar(sidebar: &Sidebar, view: &mut crate::ui::sidebar::models::SidebarView, panes: &[std::sync::Arc<std::sync::Mutex<MuxPane>>]) -> TestBackend {
        let backend = TestBackend::new(26, 48);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        term.draw(|f| {
            crate::ui::sidebar::render_sidebar(
                f,
                ratatui::layout::Rect::new(0, 0, 26, 48),
                sidebar,
                view,
                false,
                panes,
            );
        })
        .unwrap();
        term.backend().clone()
    }

    fn row_text(backend: &TestBackend, y: u16) -> String {
        let b = backend.buffer();
        let mut text = String::new();
        for x in 0..26 {
            text.push_str(b[(x, y)].symbol());
        }
        text
    }

    #[test]
    fn session_rows_are_clickable_exactly_where_they_are_drawn() {
        let _lock = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = SidebarDirGuard::new(FOUR_SESSIONS);
        let mut sidebar = Sidebar::load();
        sidebar.rebuild_rows();
        let mut view = crate::ui::sidebar::models::SidebarView::default();
        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();
        let backend = draw_sidebar(&sidebar, &mut view, &panes);

        let new_y = view
            .row_y
            .iter()
            .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::NewSession)
            .map(|(y, _)| *y)
            .expect("the New Session row must be clickable");

        for (i, session) in sidebar.sessions.iter().enumerate() {
            let y = view
                .row_y
                .iter()
                .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::Session(i))
                .map(|(y, _)| *y)
                .unwrap_or_else(|| panic!("session {i} must be clickable"));
            assert_eq!(row_text(&backend, y).contains(&session.title), true, "session {i} title must be drawn on the row that receives its clicks");
        }

        let first_y = view
            .row_y
            .iter()
            .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::Session(0))
            .map(|(y, _)| *y)
            .unwrap();
        assert_eq!(first_y, new_y + 1, "the first session sits directly under New Session");
    }

    #[test]
    fn a_session_open_in_a_pane_is_marked_in_the_sidebar() {
        let _lock = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = SidebarDirGuard::new(FOUR_SESSIONS);
        let mut sidebar = Sidebar::load();
        sidebar.rebuild_rows();

        let (pane, reader) = match MuxPane::spawn("true", 80, 24) {
            Ok(p) => p,
            Err(_) => return,
        };
        drop(reader);
        pane.lock().unwrap_or_else(|e| e.into_inner()).state.launch_cmd =
            "command-code --session s-one".to_string();

        let panes = vec![pane];
        let mut view = crate::ui::sidebar::models::SidebarView::default();
        let backend = draw_sidebar(&sidebar, &mut view, &panes);

        let y0 = view
            .row_y
            .iter()
            .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::Session(0))
            .map(|(y, _)| *y)
            .unwrap();
        let y1 = view
            .row_y
            .iter()
            .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::Session(1))
            .map(|(y, _)| *y)
            .unwrap();

        let terminal_glyph = nf_icons::nf!("nf-cod-terminal");
        let dot_glyph = nf_icons::nf!("nf-oct-dot_fill");
        assert!(row_text(&backend, y0).contains(terminal_glyph), "the live session must show the terminal marker");
        assert!(row_text(&backend, y1).contains(dot_glyph), "a closed session keeps the plain dot");
    }

    #[test]
    fn session_cache_mark_colors_the_row_dot() {
        let _lock = crate::ipc::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = SidebarDirGuard::new(FOUR_SESSIONS);
        let mut sidebar = Sidebar::load();
        sidebar.session_marks.insert("s-two".to_string(), "red".to_string());
        sidebar.rebuild_rows();

        let mut view = crate::ui::sidebar::models::SidebarView::default();
        let panes: Vec<std::sync::Arc<std::sync::Mutex<MuxPane>>> = Vec::new();
        let backend = draw_sidebar(&sidebar, &mut view, &panes);

        let palette = crate::theme::Palette::dark();
        let y1 = view
            .row_y
            .iter()
            .find(|(_, r)| *r == crate::ui::sidebar::models::SidebarRow::Session(1))
            .map(|(y, _)| *y)
            .unwrap();
        let b = backend.buffer();
        let dot_x = (0..26).find(|x| b[(*x, y1)].symbol() == nf_icons::nf!("nf-oct-dot_fill")).expect("dot drawn");
        assert_eq!(b[(dot_x, y1)].style().fg, Some(palette.red), "a cache mark must color the bullet");
    }
}



