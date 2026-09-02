
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

use crossterm::event::{self, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::enable_raw_mode;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod agent_state;
mod ipc;
mod mux_core;
mod mux_events;
mod orphan_journal;
mod path_lease;
mod prefs;
mod render_loop;
mod scroll_physics;
mod splash;
mod selection;
mod state;
mod theme;
mod ui;
mod update;
mod usage;
mod auto_retry;
mod skills;

use crate::mux_core::input::{handle_key, handle_mouse};
use crate::mux_core::pane_ops::spawn_pane;
use crate::prefs::Prefs;
use crate::state::AppState;
use crate::ui::sidebar::Sidebar;

use crate::render_loop::{
    should_draw_split, FRAME_MS, IDLE_CADENCE_MS, LOADING_DRAW_MS, MIN_FRAME_MS,
};

fn find_pane_by_gen(
    state: &AppState,
    gen: u64,
) -> Option<std::sync::Arc<std::sync::Mutex<crate::ui::pane::MuxPane>>> {
    state.panes.iter().find(|p| {
        p.lock()
            .map(|g| g.state.gen == gen)
            .unwrap_or(false)
    }).cloned()
}

fn main() -> io::Result<()> {

    std::panic::set_hook(Box::new(|info| {
        let has_backtrace = std::env::var("RUST_BACKTRACE").map(|v| v != "0").unwrap_or(false);
        let msg = if has_backtrace {
            format!(
                "cc-mux panic: {}\nBacktrace:\n{}\n",
                info,
                std::backtrace::Backtrace::capture()
            )
        } else {
            format!("cc-mux panic: {}\n", info)
        };
        let _ = std::fs::create_dir_all(crate::ipc::data_dir());
        let _ = std::fs::write(crate::ipc::ipc_path("panic.log"), &msg);
    }));

    unsafe {
        extern "C" fn on_signal(sig: libc::c_int) {
            crate::ipc::log_append("resize.log", &format!("signal: {sig} (exit)"));
            crate::orphan_journal::kill_all_registered();
            std::process::exit(0);
        }
        extern "C" fn on_reload(_: libc::c_int) {
            RELOAD_REQUESTED.store(true, Ordering::SeqCst);
        }
        libc::signal(libc::SIGTERM, on_signal as libc::sighandler_t);
        libc::signal(libc::SIGHUP, on_signal as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as libc::sighandler_t);
        libc::signal(libc::SIGUSR1, on_reload as libc::sighandler_t);
    }

    let args: Vec<String> = std::env::args().collect();
    let mut command = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "commandcode".to_string());
    let new_tab_command = args.get(2).cloned().unwrap_or_else(|| command.clone());

    fn ensure_cd_prefix(cmd: &str) -> String {
        if cmd.trim_start().starts_with("cd ") {
            return cmd.to_string();
        }
        match std::env::current_dir() {
            Ok(d) => {
                let d = d.to_string_lossy().to_string();
                format!("cd {} && {}", crate::mux_core::input::shell_quote(&d), cmd)
            }
            Err(_) => cmd.to_string(),
        }
    }
    command = ensure_cd_prefix(&command);

    let _ = crate::orphan_journal::cleanup_orphans_on_startup();

    std::fs::write(crate::ipc::ipc_path("dashboard.pid"), std::process::id().to_string()).ok();

    crate::ipc::log_reset("resize.log");

    let prefs = Prefs::load();

    let _ = std::fs::remove_file(crate::ipc::ipc_path("cost.json"));
    if let Ok(entries) = std::fs::read_dir(crate::ipc::data_dir()) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.starts_with("cost-")
                && name.ends_with(".json")
                && entry.path().parent().is_some()
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    enable_raw_mode()?;
    let _ = execute!(io::stdout(), EnableMouseCapture);

    let _ = execute!(io::stdout(), crossterm::event::EnableBracketedPaste);

    let _ = execute!(
        io::stdout(),
        crossterm::event::PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        )
    );

    let _ = execute!(io::stdout(), crossterm::cursor::Hide);
    print!("\x1b]0;Command Code\x07");
    print!("\x1b]11;#000000\x07");
    {
        use std::io::Write as _;
        let _ = io::stdout().flush();
    }
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    if crate::splash::read_show_splash() {
        let _ = crate::splash::run_splash(&mut terminal);
    }

    let size = crate::render_loop::viewport_size(&mut terminal)?;

    let mut sidebar = Sidebar::load();
    sidebar.yolo_mode = prefs.yolo_mode;
    sidebar.taste_learning = prefs.taste_learning;
    sidebar.ide_context = prefs.ide_context;
    sidebar.show_cost_bar = prefs.show_cost_bar;
    sidebar.show_context_btn = prefs.show_context_btn;
    if sidebar.yolo_mode && !command.contains("--yolo") {
        command.push_str(" --yolo");
    }

    let mut state = AppState::new(sidebar);

    let event_rx = state.take_events().expect("event receiver");
    state.sidebar_w = prefs.sidebar_w;
    state.sidebar_open = prefs.sidebar_open;
    let mut new_tab_cmd = ensure_cd_prefix(&new_tab_command);
    if state.sidebar.yolo_mode && !new_tab_cmd.contains("--yolo") {
        new_tab_cmd.push_str(" --yolo");
    }

    let cols = size.width.saturating_sub(state.sidebar_w + 3).max(20);
    let rows = size.height.saturating_sub(4).max(5);
    if spawn_pane(&mut state, &command, cols, rows).is_err() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        if spawn_pane(&mut state, &shell, cols, rows).is_err() {

            let _ = std::fs::write(
                crate::ipc::ipc_path("panic.log"),
                "spawn failed (command and shell)\n",
            );
        }
    }

    crate::update::check_for_updates_background(state.events.clone());
    crate::usage::spawn_usage_checker(state.events.clone());
    crate::skills::check_all_background(state.events.clone());

    let mut last_sidebar_refresh = std::time::Instant::now();
    let mut last_mods_refresh = std::time::Instant::now();
    let mut last_picker_check = std::time::Instant::now();
    let mut last_update_check = std::time::Instant::now();
    let mut last_saved_prefs = prefs.clone();
    let mut last_draw = std::time::Instant::now();

    let mut force_draw = false;

    let mut input_errors: u32 = 0;

    loop {

        if RELOAD_REQUESTED.swap(false, Ordering::SeqCst) {
            crate::mux_core::input::reload_mux();
        }

        let t_drain = std::time::Instant::now();
        while let Ok(ev) = event_rx.try_recv() {
            match ev {
                crate::mux_events::MuxEvent::PtyOutput { gen, bytes } => {

                    if let Some(pane) = find_pane_by_gen(&state, gen) {
                        if let Ok(mut p) = pane.lock() {

                            p.feed(&bytes);
                            state.dirty = true;
                        }
                    }
                }
                crate::mux_events::MuxEvent::PaneExited { gen } => {
                    if let Some(pane) = find_pane_by_gen(&state, gen) {
                        if let Ok(mut p) = pane.lock() {
                            p.state.exited = true;
                        }
                        state.dirty = true;
                    }
                }
                crate::mux_events::MuxEvent::UpdateAvailable { version } => {
                    state.available_update = Some(version.clone());
                    state.sidebar.available_update = Some(version);
                    state.dirty = true;
                }
                crate::mux_events::MuxEvent::UpdateProgress { label, current, total } => {
                    crate::mux_core::modals::open_update_progress_modal(&mut state, &label, current, total);
                    state.dirty = true;
                }
                crate::mux_events::MuxEvent::UpdateCompleted { success, error } => {
                    if success {
                        crate::mux_core::input::reload_mux();
                    } else if let Some(err) = error {
                        crate::mux_core::modals::open_update_modal(&mut state, &format!("✗ Update failed: {err}"));
                        state.dirty = true;
                    }
                }
                crate::mux_events::MuxEvent::UsageUpdated(usage) => {
                    state.sidebar.usage = Some(usage);
                    state.dirty = true;
                }
                crate::mux_events::MuxEvent::SkillsUpdated { vendors } => {
                    let count = vendors.iter().filter(|v| v.is_stale()).count();
                    state.sidebar.set_skills_update_count(count);
                    state.dirty = true;
                }
                crate::mux_events::MuxEvent::SkillsUpdateProgress {
                    done,
                    total,
                    current,
                    last_result,
                } => {
                    if let Some(upd) = state.skills_view.updating.as_mut() {
                        upd.done = done;
                        upd.total = total;
                        upd.current = current;
                        upd.last_result = last_result;
                    } else {
                        state.skills_view.updating = Some(crate::state::SkillsUpdateProgress {
                            done,
                            total,
                            current,
                            last_result,
                            started_at_ms: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis() as u64)
                                .unwrap_or(0),
                        });
                    }
                    if crate::ui::modal::skills::is_skills_modal(&state)
                        && crate::ui::modal::skills::current_step(&state) == 1
                    {
                        crate::ui::modal::open_skills_modal(&mut state);
                    }
                    state.dirty = true;
                }
                crate::mux_events::MuxEvent::SkillsUpdateDone { ok, failed } => {
                    state.skills_view.updating = None;
                    state.skills_view.last_update_summary =
                        Some(format!("{ok} updated, {failed} failed"));
                    crate::skills::check_all_background(state.events.clone());
                    if crate::ui::modal::skills::is_skills_modal(&state) {
                        crate::ui::modal::open_skills_modal(&mut state);
                    }
                    state.dirty = true;
                }
            }
            if t_drain.elapsed().as_millis() > 500 {
                crate::ipc::log_append("resize.log", &format!("hang: event drain {:.0}ms", t_drain.elapsed().as_secs_f64() * 1000.0));
                break;
            }
        }

        let before = state.panes.len();
        state.panes.retain(|p| !p.lock().unwrap_or_else(|e| e.into_inner()).state.exited);
        if state.panes.len() != before {
            state.sync_pane_count();
            state.dirty = true;
        }
        if state.panes.is_empty() {

            let (cols, rows) = (80, 24);
            if spawn_pane(&mut state, &command, cols, rows).is_err() {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
                let _ = spawn_pane(&mut state, &shell, cols, rows);
            }
            state.dirty = true;
        }
        state.clamp_active();

        state.refresh_model_cache();

        if last_picker_check.elapsed() >= std::time::Duration::from_millis(300) {
            last_picker_check = std::time::Instant::now();
            if let Ok(entries) = std::fs::read_dir(crate::ui::mod_bridge::load::mods_data_dir()) {

                if state.active_modal.is_some() {

                    let mut progress_updated = false;
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            continue;
                        }
                        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                            continue;
                        };
                        if let Ok(raw) = std::fs::read_to_string(&path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                                if let Some(modals) = json.get("modals").and_then(|m| m.as_array())
                                {
                                    for modal in modals {
                                        if modal.get("pending").and_then(|p| p.as_bool())
                                            == Some(true)
                                        {
                                            let active = state.active_modal.as_mut().unwrap();
                                            let want_id = format!("list_{}", stem);
                                            let is_progress = modal
                                                .get("progress")
                                                .and_then(|p| p.as_object())
                                                .is_some();
                                            if is_progress && active.id == want_id {
                                                if let Some(prog) = modal.get("progress") {
                                                    if let Some(cur) =
                                                        prog.get("current").and_then(|c| c.as_u64())
                                                    {
                                                        let total = prog
                                                            .get("total")
                                                            .and_then(|t| t.as_u64())
                                                            .unwrap_or(1)
                                                            .max(1);
                                                        let label = prog
                                                            .get("label")
                                                            .and_then(|l| l.as_str())
                                                            .unwrap_or("")
                                                            .to_string();
                                                        for row in &mut active.rows {
                                                            if let crate::ui::modal::ModalRow::Progress { current: c, total: t, label: l, .. } = row {
                                                                *c = cur as usize;
                                                                *t = total as usize;
                                                                *l = label.clone();
                                                                state.dirty = true;
                                                            }
                                                        }
                                                        progress_updated = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if progress_updated {
                        continue;
                    }

                    if let Some(active) = &state.active_modal {
                        let is_progress_modal = active
                            .rows
                            .iter()
                            .any(|r| matches!(r, crate::ui::modal::ModalRow::Progress { .. }));
                        if is_progress_modal {
                            let still_present = std::fs::read_dir(crate::ui::mod_bridge::load::mods_data_dir())
                                .ok()
                                .into_iter()
                                .flatten()
                                .flatten()
                                .filter(|e| {
                                    e.path().extension().and_then(|x| x.to_str()) == Some("json")
                                })
                                .filter_map(|e| std::fs::read_to_string(e.path()).ok())
                                .filter_map(|raw| {
                                    serde_json::from_str::<serde_json::Value>(&raw).ok()
                                })
                                .filter_map(|j| j.get("modals").and_then(|m| m.as_array()).cloned())
                                .flatten()
                                .any(|m| {
                                    m.get("id").and_then(|i| i.as_str())
                                        == Some(active.id.trim_start_matches("list_"))
                                });
                            if !still_present {
                                state.active_modal = None;
                                state.dirty = true;
                            }
                        }
                    }
                } else {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            continue;
                        }
                        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                            continue;
                        };
                        if let Ok(raw) = std::fs::read_to_string(&path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                                if let Some(modals) = json.get("modals").and_then(|m| m.as_array())
                                {
                                    for modal in modals {
                                        if modal.get("pending").and_then(|p| p.as_bool())
                                            == Some(true)
                                        {
                                            if let Ok(m) = serde_json::from_value::<
                                                crate::ui::mod_bridge::ModModal,
                                            >(
                                                modal.clone()
                                            ) {

                                                let consume_modal = |v: &mut serde_json::Value| {
                                                    if let Some(arr) = v
                                                        .get_mut("modals")
                                                        .and_then(|m| m.as_array_mut())
                                                    {
                                                        arr.retain(|mm| {
                                                            !(mm.get("pending")
                                                                .and_then(|p| p.as_bool())
                                                                == Some(true))
                                                        });
                                                    }
                                                };

                                                if m.id == "open-config" {
                                                    let canonical_stem =
                                                        crate::ui::mod_bridge::load::canonical_mod_id(&stem);
                                                    if let Some(idx) = state
                                                        .sidebar
                                                        .mods
                                                        .iter()
                                                        .position(|mi| mi.id == canonical_stem)
                                                    {
                                                        crate::mux_core::modals::open_mod_config_modal(
                                                            &mut state,
                                                            idx,
                                                        );
                                                        state.dirty = true;
                                                        if let Ok(mut v) = serde_json::from_str::<
                                                            serde_json::Value,
                                                        >(
                                                            &raw
                                                        ) {
                                                            consume_modal(&mut v);
                                                            if let Ok(bridge) =
                                                                serde_json::to_string(&v)
                                                            {
                                                                let _ =
                                                                    std::fs::write(&path, bridge);
                                                            }
                                                        }
                                                        break;
                                                    }
                                                    continue;
                                                }

                                                let canonical_id =
                                                    crate::ui::mod_bridge::load::canonical_mod_id(&stem);
                                                crate::mux_core::modals::open_list_modal(
                                                    &mut state, &canonical_id, &m,
                                                );
                                                state.dirty = true;

                                                let is_progress = m.progress.is_some();
                                                if !is_progress {
                                                    if let Ok(mut v) =
                                                        serde_json::from_str::<serde_json::Value>(
                                                            &raw,
                                                        )
                                                    {
                                                        consume_modal(&mut v);
                                                        if let Ok(bridge) =
                                                            serde_json::to_string(&v)
                                                        {
                                                            let _ = std::fs::write(&path, bridge);
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for p in &state.panes {
            if let Ok(mut g) = p.lock() {
                g.update_agent_state();
            }
        }

        if last_sidebar_refresh.elapsed() >= std::time::Duration::from_secs(5) {
            state.sidebar.refresh();
            state.dirty = true;
            let mut current = Prefs::load();
            current.show_banner = crate::ui::banner::is_banner_enabled();
            current.yolo_mode = state.sidebar.yolo_mode;
            current.taste_learning = state.sidebar.taste_learning;
            current.ide_context = state.sidebar.ide_context;
            current.show_cost_bar = state.sidebar.show_cost_bar;
            current.show_context_btn = state.sidebar.show_context_btn;
            current.show_usage = state.sidebar.show_usage;
            current.sidebar_w = state.sidebar_w;
            current.sidebar_open = state.sidebar_open;
            current.auto_retry.enabled = state.sidebar.auto_retry_enabled;
            if current != last_saved_prefs {
                current.save();
                last_saved_prefs = current;
            }
            last_sidebar_refresh = std::time::Instant::now();
        }

        if last_update_check.elapsed() >= std::time::Duration::from_secs(300) {
            crate::update::check_for_updates_background(state.events.clone());
            last_update_check = std::time::Instant::now();
        }

        {
            let populated = !state.mods_data.segments().is_empty()
                || state
                    .mods_data
                    .mods
                    .iter()
                    .any(|m| m.data.context_usage().is_some());
            let cadence = if populated {
                std::time::Duration::from_secs(5)
            } else {
                std::time::Duration::from_millis(600)
            };
            if last_mods_refresh.elapsed() >= cadence {
                let t_mods = std::time::Instant::now();
                let active_tty = state
                    .panes
                    .get(state.active)
                    .and_then(|p| p.lock().ok())
                    .map(|p| p.state.tty_name.clone())
                    .filter(|t| !t.is_empty());
                state.mods_data =
                    crate::ui::mod_bridge::ModsData::load_with_tty(active_tty.as_deref());
                if t_mods.elapsed().as_millis() > 500 {
                    crate::ipc::log_append("resize.log", &format!("hang: mods load {:.0}ms", t_mods.elapsed().as_secs_f64() * 1000.0));
                }

                state.sidebar.live_blocks = state.mods_data.live_blocks();
                state.dirty = true;
                last_mods_refresh = std::time::Instant::now();

                force_draw = true;
            }
        }

        let _ = crate::render_loop::viewport_size(&mut terminal);

        let frame_ms = FRAME_MS;
        let idle_cadence_ms = IDLE_CADENCE_MS;
        let draw_due = last_draw.elapsed().as_millis() as u64;
        let poll_ms = if draw_due >= frame_ms {

            MIN_FRAME_MS.max(2)
        } else {
            let until_draw = frame_ms - draw_due;
            let until_idle = idle_cadence_ms - draw_due.min(idle_cadence_ms);
            until_draw.min(until_idle).max(MIN_FRAME_MS)
        };

        match event::poll(std::time::Duration::from_millis(poll_ms)) {
            Ok(true) => match event::read() {
                Ok(Event::Mouse(mut mouse)) => {
                    input_errors = 0;
                    state.dirty = true;
                    if matches!(
                        mouse.kind,
                        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
                            | crossterm::event::MouseEventKind::Up(
                                crossterm::event::MouseButton::Left
                            )
                    ) {
                        crate::ipc::log_append(
                            "resize.log",
                            &format!("mouse: {:?} at {},{}", mouse.kind, mouse.column, mouse.row),
                        );
                    }
                    let is_scroll = matches!(
                        mouse.kind,
                        crossterm::event::MouseEventKind::ScrollUp
                            | crossterm::event::MouseEventKind::ScrollDown
                    );
                    let mut scroll_accum = if is_scroll {
                        if mouse.kind == crossterm::event::MouseEventKind::ScrollUp {
                            3
                        } else {
                            -3
                        }
                    } else {
                        0
                    };

                    while let Ok(true) = event::poll(std::time::Duration::from_millis(0)) {
                        if let Ok(Event::Mouse(next_mouse)) = event::read() {
                            let next_is_scroll = matches!(
                                next_mouse.kind,
                                crossterm::event::MouseEventKind::ScrollUp
                                    | crossterm::event::MouseEventKind::ScrollDown
                            );
                            if is_scroll && next_is_scroll {
                                let delta = if next_mouse.kind
                                    == crossterm::event::MouseEventKind::ScrollUp
                                {
                                    3
                                } else {
                                    -3
                                };
                                if (scroll_accum > 0 && delta < 0)
                                    || (scroll_accum < 0 && delta > 0)
                                {
                                    scroll_accum = delta;
                                } else {
                                    scroll_accum += delta;
                                }
                                mouse = next_mouse;
                            } else {
                                if !is_scroll {
                                    if let Err(e) = handle_mouse(
                                        &mut state,
                                        &terminal,
                                        mouse,
                                        &command,
                                        &new_tab_cmd,
                                    ) {
                                        crate::ipc::log_append(
                                            "resize.log",
                                            &format!("mouse_error: {e}"),
                                        );
                                    }
                                }
                                mouse = next_mouse;
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if is_scroll && scroll_accum != 0 {
                        let _ = crate::mux_core::input::handle_scroll_accum(
                            &mut state,
                            mouse,
                            scroll_accum,
                        );
                    } else if !is_scroll {
                        if let Err(e) =
                            handle_mouse(&mut state, &terminal, mouse, &command, &new_tab_cmd)
                        {
                            crate::ipc::log_append(
                                "resize.log",
                                &format!("mouse_error: {e}"),
                            );
                        }
                    }
                }
                Ok(Event::Key(key)) => {
                    input_errors = 0;
                    state.dirty = true;

                    if state.active_modal.is_none()
                        && state.picker.is_none()
                        && !state.sidebar_focus
                        && !state.panel_focused
                        && crate::mux_core::dictation::is_plain_char_key(&key)
                    {
                        let mut chunk = String::new();
                        if let crossterm::event::KeyCode::Char(c) = key.code {
                            chunk.push(c);
                        }
                        while crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                            if let Ok(Event::Key(next_key)) = crossterm::event::read() {
                                if crate::mux_core::dictation::is_plain_char_key(&next_key) {
                                    if let crossterm::event::KeyCode::Char(c) = next_key.code {
                                        chunk.push(c);
                                    }
                                } else {
                                    if !chunk.is_empty() {
                                        if let Some(pane) = state.panes.get(state.active) {
                                            let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
                                            p.write_input(chunk.as_bytes());
                                        }
                                        chunk.clear();
                                    }
                                    handle_key(&mut state, next_key, &command, &new_tab_cmd);
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        if !chunk.is_empty() {
                            if let Some(pane) = state.panes.get(state.active) {
                                let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
                                p.write_input(chunk.as_bytes());
                            }
                        }
                    } else {
                        handle_key(&mut state, key, &command, &new_tab_cmd);
                    }
                }

                Ok(Event::Paste(text)) => {
                    input_errors = 0;
                    state.dirty = true;
                    if !text.is_empty() {
                        crate::mux_core::input::handle_paste(&mut state, &text);
                    }
                }
                Ok(Event::Resize(width, height)) => {
                    input_errors = 0;
                    state.dirty = true;
                    crate::ipc::log_append(
                        "resize.log",
                        &format!("event_resize: host={width}x{height}"),
                    );
                }
                Ok(_) => {
                    input_errors = 0;
                }
                Err(e) => {
                    input_errors += 1;
                    crate::ipc::log_append(
                        "resize.log",
                        &format!("read_error[{}]: {e}", input_errors),
                    );
                    if input_errors > 15 {
                        crate::ipc::log_append("resize.log", "EXIT: read errors > 15");
                        return Err(e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            },
            Ok(false) => {
                input_errors = 0;
            }
            Err(e) => {
                input_errors += 1;
                crate::ipc::log_append(
                    "resize.log",
                    &format!("loop_error[{}]: {e}", input_errors),
                );
                if input_errors > 15 {
                    crate::ipc::log_append("resize.log", "EXIT: input_errors > 15 (tty presumed dead)");
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        if force_draw {
            let _ = terminal.draw(|f| crate::ui::render(f, &mut state));
            force_draw = false;
        }

        let input_dirty = state.dirty;
        let output_dirty = state
            .panes
            .iter()
            .any(|p| p.lock().map(|g| g.state.dirty).unwrap_or(false));

        let loading = state
            .panes
            .iter()
            .any(|p| p.lock().map(|g| g.state.loading).unwrap_or(false));

        crate::mux_core::input::flush_dictation(&mut state);

        let draw_ok = if loading {
            last_draw.elapsed() >= std::time::Duration::from_millis(LOADING_DRAW_MS)
        } else {
            should_draw_split(input_dirty, output_dirty, last_draw.elapsed())
        };

        if draw_ok {
            let t_draw = std::time::Instant::now();
            match terminal.draw(|f| {
                crate::ui::render(f, &mut state);
            }) {
                Ok(_) => {
                    if t_draw.elapsed().as_millis() > 500 {
                        crate::ipc::log_append("resize.log", &format!("hang: draw {:.0}ms", t_draw.elapsed().as_secs_f64() * 1000.0));
                    }
                    input_errors = 0;

                    state.dirty = false;
                    for p in &state.panes {
                        if let Ok(mut g) = p.lock() {
                            g.state.dirty = false;
                        }
                    }
                }
                Err(e) => {
                    input_errors += 1;
                    crate::ipc::log_append(
                        "resize.log",
                        &format!("draw_error[{}]: {e}", input_errors),
                    );
                    if input_errors > 15 {
                        crate::ipc::log_append("resize.log", "EXIT: draw errors > 15");
                        return Err(e);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
            last_draw = std::time::Instant::now();
        } else {
            std::thread::sleep(std::time::Duration::from_millis(poll_ms.max(1)));
        }
    }
}

#[cfg(test)]
mod mux_tests;

