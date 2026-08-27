
pub mod widget;
pub mod widgets;

use ratatui::{layout::Rect, Frame};

use crate::state::AppState;
use crate::theme::Palette;

pub mod banner;
pub mod borders;
pub mod cmdinfo;
pub mod context_menu;
pub mod glyph;
pub mod links;
pub mod mod_bridge;
pub mod mod_panel;
pub mod modal;
pub mod pane;
pub mod pane_keys;
pub mod pane_render;
pub mod pane_state;
pub mod pane_tests;
pub mod pane_tty;
pub mod sidebar;
pub mod search;
pub mod switcher;
pub mod tab_bar;
pub mod text;

pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();
    let pal = Palette::dark();

    frame.buffer_mut().set_style(
        area,
        ratatui::style::Style::default().bg(crate::theme::effective_bg()),
    );

    let sidebar_w = state.sidebar_w;

    let (sidebar_area, main_area) = if state.sidebar_open && area.width > sidebar_w + 8 {
        (
            Rect::new(area.x, area.y, sidebar_w, area.height),
            Rect::new(
                area.x + sidebar_w,
                area.y,
                area.width.saturating_sub(sidebar_w),
                area.height,
            ),
        )
    } else {
        (Rect::default(), area)
    };

    if state.sidebar_open && area.width > sidebar_w + 8 && area.height > 0 {
        sidebar::render_sidebar(
            frame,
            sidebar_area,
            &state.sidebar,
            &mut state.sidebar_view,
            state.sidebar_focus,
            &state.panes,
        );
    }

    let (right_panel, main_area) = if state.panel_maximized {
        (Rect::default(), main_area)
    } else if state.panel_sidebar_open && main_area.width > state.panel_sidebar_w + 8 {
        let w = state.panel_sidebar_w.min(main_area.width / 2);
        let main = Rect::new(
            main_area.x,
            main_area.y,
            main_area.width.saturating_sub(w),
            main_area.height,
        );
        let right = Rect::new(
            main_area.x + main.width,
            main_area.y,
            w,
            main_area.height,
        );
        (right, main)
    } else {
        (Rect::default(), main_area)
    };

    let tab_h = 1u16;
    let tab_bar_area = Rect::new(main_area.x, main_area.y, main_area.width, tab_h);
    tab_bar::render_tab_bar(frame, tab_bar_area, &state.panes, state.active);

    let pane_area = Rect::new(
        main_area.x,
        main_area.y + tab_h,
        main_area.width,
        main_area.height.saturating_sub(tab_h),
    );

    let panel = state
        .mods_data
        .mods
        .iter()
        .find_map(|m| m.data.panels.first().cloned());

    if state.panel_maximized {
        if let Some(panel) = panel {
            state.panel_state.reconcile(&panel);
            mod_panel::render_panel(frame, pane_area, &panel, &state.panel_state, true, &mut state.panel_view);
        } else {
            let p = Palette::dark();
            let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                " panel…",
                ratatui::style::Style::default()
                    .fg(p.yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )]);
            frame.buffer_mut().set_line(pane_area.left(), pane_area.top(), &line, pane_area.width);
        }

        if let Some(ref modal) = state.active_modal {
            modal::render_modal(frame, area, modal, &pal);
        }
        return;
    }

    if state.panel_sidebar_open && !right_panel.is_empty() {
        if let Some(panel) = panel {
            state.panel_state.reconcile(&panel);
            mod_panel::render_panel(
                frame,
                right_panel,
                &panel,
                &state.panel_state,
                state.panel_focused,
                &mut state.panel_view,
            );
        } else {
            let p = Palette::dark();
            let line = ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                " panel…",
                ratatui::style::Style::default()
                    .fg(p.yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )]);
            frame.buffer_mut().set_line(right_panel.left(), right_panel.top(), &line, right_panel.width);
        }
    }

    let (model, effort) = state.model_info();

    let gauge_mod = state
        .mods_data
        .mods
        .iter()
        .find(|m| m.data.context_usage().is_some());
    let context_usage = gauge_mod.and_then(|m| m.data.context_usage());

    let mod_known = !state.mods_data.known_mod_ids.is_empty();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let bridge_stale = match gauge_mod {
        Some(gauge) => crate::ui::mod_bridge::bridge_is_stale(
            gauge.data.updated_at,
            state.started_at_ms,
            now_ms,
        ),
        None => true,
    };
    if let Some(pane) = state.panes.get(state.active) {
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let segments = state.mods_data.segments();
        pane::render_pane(
            frame,
            pane_area,
            &mut p,
            &segments,
            context_usage,
            bridge_stale,
            state.sidebar.show_cost_bar,
            mod_known,
        );

        let banner_h = banner::banner_height(&mut p, pane_area);
        if banner_h > 0 {
            let banner_area = Rect::new(pane_area.x, pane_area.y, pane_area.width, banner_h);
            let yolo = p.state.yolo_mode;
            banner::maybe_render(
                frame,
                banner_area,
                &mut p,
                state.sidebar.sessions.first().map(|s| s.title.as_str()),
                yolo,
                model.as_deref(),
                effort.as_deref(),
            );
        }
    }

    if let Some(ref modal) = state.active_modal {
        modal::render_modal(frame, area, modal, &pal);
    }

    if let Some(ref picker_state) = state.picker {

        let visible = picker_state.picker.filtered_indices().len().min(14);
        let popup = modal::modal_rect(area, visible + 4, 0, 56).unwrap_or(Rect::new(
            area.x + area.width / 4,
            area.y + area.height / 4,
            area.width / 2,
            area.height / 2,
        ));
        let inner =
            crate::ui::widget::render_modal_shell(frame, area, popup.width, popup.height, &pal);
        if let Some(inner) = inner {
            picker_state.picker.render(frame, inner, &pal);
        }
    }

    if let Some(ref finder) = state.finder {
        search::render_finder(frame, area, finder, &pal);
    }

    if let Some(ref inspect) = state.cmd_inspect {
        inspect.render(frame, area, &pal);
    }

    if let Some(ref switcher) = state.switcher {
        switcher.render(frame, area, &pal);
    }

    if let Some(ref mut menu) = state.context_menu {
        menu.render(frame, area, None, &pal);
    }
}

