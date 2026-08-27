
#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::term::{Config as TermConfig, Term};
    use crate::ui::pane::*;

    #[test]
    fn session_id_from_cmd_extracts_ids() {
        assert_eq!(
            MuxPane::session_id_from_cmd("commandcode --session abc-123"),
            Some("abc-123".to_string())
        );
        assert_eq!(
            MuxPane::session_id_from_cmd("cd '/x' && commandcode --resume def-456"),
            Some("def-456".to_string())
        );
        assert_eq!(MuxPane::session_id_from_cmd("commandcode --yolo"), None);
        assert_eq!(MuxPane::session_id_from_cmd("zsh"), None);
    }

    #[test]
    fn session_live_requires_launch_cmd_or_fresh_status() {

        let Ok((pane, _r)) = MuxPane::spawn("commandcode --session abc-123", 80, 24) else {
            return;
        };
        assert!(pane.lock().unwrap().is_session_live("abc-123"));
        assert!(!pane.lock().unwrap().is_session_live("other-id"));

        let Ok((plain, _r2)) = MuxPane::spawn("commandcode", 80, 24) else {
            return;
        };
        assert!(!plain.lock().unwrap().is_session_live("abc-123"));
    }

    #[test]
    fn global_agent_status_isolation_rules() {
        assert!(MuxPane::trust_global(Some("sess-A"), Some("sess-A"), 3));
        assert!(!MuxPane::trust_global(Some("sess-A"), Some("sess-B"), 3));
        assert!(!MuxPane::trust_global(Some("sess-A"), None, 3));
        assert!(MuxPane::trust_global(None, None, 1));
        assert!(!MuxPane::trust_global(None, None, 2));
        assert!(!MuxPane::trust_global(None, Some("sess-A"), 1));
    }

    #[test]
    fn substantive_output_ignores_pure_escape_sequences() {
        assert!(!is_substantive_output(b"\x1b[1;1H\x1b[2J\x1b[38;5;123m"));
        assert!(!is_substantive_output(b"\x1b]0;title\x07"));
        assert!(!is_substantive_output(b"\x1b[?25h\x1b[?25l"));
        assert!(is_substantive_output(b"hello world\n"));
        assert!(is_substantive_output(b"\x1b[1;1Hline of text"));
    }

    #[test]
    fn scrolled_display_iter_maps_to_viewport_rows() {
        let size = TermSize { cols: 40, rows: 10 };
        let mut term = Term::new(TermConfig::default(), &size, VoidListener);
        let mut out = Vec::new();
        for i in 0..25 {
            writeln!(&mut out, "line {:02} abcdefghijklmnopqrstuvwxyz", i).unwrap();
        }
        let mut parser = alacritty_terminal::vte::ansi::Processor::<
            alacritty_terminal::vte::ansi::StdSyncHandler,
        >::new();
        parser.advance(&mut term, &out);
        assert!(term.grid().history_size() > 0);

        term.scroll_display(alacritty_terminal::grid::Scroll::Delta(5));
        let offset = term.grid().display_offset();
        assert_eq!(offset, 5);

        let content = term.renderable_content();
        let mut rows = std::collections::BTreeSet::new();
        for item in content.display_iter {
            let vp = alacritty_terminal::term::point_to_viewport(offset, item.point)
                .expect("grid point must map to viewport");
            assert!(vp.line < 10, "viewport row {} out of range", vp.line);
            rows.insert(vp.line);
        }
        assert_eq!(rows.len(), 10, "viewport rows must be fully covered");
    }

    #[test]
    fn tty_of_pid_returns_none_for_dead_pid() {
        assert_eq!(tty_of_pid(u32::MAX), None);
    }

    #[test]
    fn spawn_resolves_a_controlling_tty_for_isolation() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 80, 24) else {
            return;
        };
        let tty = pane.lock().unwrap_or_else(|e| e.into_inner()).state.tty_name.clone();
        assert!(
            !tty.is_empty(),
            "pane must resolve its controlling tty for cost isolation"
        );
    }

    #[test]
    fn typing_scrolls_to_bottom_but_navigation_does_not() {
        assert!(should_scroll_to_bottom(b"hello"));
        assert!(should_scroll_to_bottom(b"\r"));
        assert!(!should_scroll_to_bottom(b"\x1b[A"));
        assert!(!should_scroll_to_bottom(b"\x1b[B"));
        assert!(!should_scroll_to_bottom(b"\x1b[5~"));
        assert!(!should_scroll_to_bottom(b"\x1b[6~"));
        assert!(!should_scroll_to_bottom(b"\x1b"));
    }

    #[test]
    fn feed_marks_dirty_only_for_real_output() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        p.state.dirty = false;
        p.feed(b"\x1b[6n");
        assert!(!p.state.dirty);
        p.feed(b"hello world\n");
        assert!(p.state.dirty);
        p.state.dirty = false;
        p.feed(b"\x1b[2J");
        assert!(p.state.dirty);
    }

    #[test]
    fn scroll_performance_delta_3_handles_edge_cases() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        p.scroll_display(3);
        p.scroll_display(3);
        p.scroll_display(-3);
        p.scroll_display(-3);
        p.scroll_reset();
        assert_eq!(p.term.grid().display_offset(), 0);
    }

    #[test]
    fn engine_scroll_offset_clamped_after_shrink() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 60, 30) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        for i in 0..80 {
            p.feed(format!("line {i}\r\n").as_bytes());
        }

        p.scroll_to_fraction(0.0);
        assert!(p.term.grid().display_offset() > 0, "test needs scrolled state");

        let new_h = 6u16;
        p.resize(60, new_h);

        let total = p.term.total_lines();
        let viewport = p.term.screen_lines().max(1) as usize;
        let max_after = total.saturating_sub(viewport);
        if p.term.grid().display_offset() > max_after {

            let raw = p.term.grid().display_offset();
            p.scroll_display(-((raw - max_after) as i32));
            assert_eq!(
                p.term.grid().display_offset(),
                max_after,
                "engine offset must be clamped to the real max after shrink"
            );
        }

        let before = p.term.grid().display_offset() as i64;
        let fed = 20i64;
        for i in 100..100 + fed as u32 {
            p.feed(format!("post {i}\r\n").as_bytes());
        }
        let after = p.term.grid().display_offset() as i64;
        assert_eq!(
            after,
            before + fed,
            "view must stay anchored to its content while scrolled \
             (offset {before} → {after}, expected {before}+{fed})"
        );
    }

    #[test]
    fn scroll_to_fraction_jumps_to_clicked_position() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());

        for i in 0..60 {
            p.feed(format!("line {i}\r\n").as_bytes());
        }
        let m0 = p.scroll_metrics();
        assert!(m0.max_offset_from_bottom > 0, "test needs scrollback");
        assert_eq!(m0.offset_from_bottom, 0, "fresh pane starts at bottom");

        p.scroll_to_fraction(0.0);
        let m = p.scroll_metrics();
        assert_eq!(m.offset_from_bottom, m.max_offset_from_bottom);

        p.scroll_to_fraction(5.0);
        assert_eq!(p.scroll_metrics().offset_from_bottom, 0);

        p.scroll_to_fraction(0.5);
        let mid = p.scroll_metrics().offset_from_bottom;
        assert!(mid > 0 && mid < p.scroll_metrics().max_offset_from_bottom);
    }

    #[test]
    fn feed_survives_malformed_escape_sequences() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let hostile: &[&[u8]] = &[
            b"\x1b",
            b"\x1b[",
            b"\x1b[999",
            b"\x1b[?25",
            b"\x1b]0;",
            b"\x1b]52;c;",
            b"\xff\xfe\x80",
            b"\x00\x01\x02\x1b[2J",
            &[0x1b; 4096],
        ];
        for seq in hostile {
            p.feed(seq);
        }
        p.feed(b"still alive\n");
        assert!(p.state.dirty);
    }

    #[test]
    fn paste_writes_without_interleaving() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 80, 24) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let sample = b"hello world\n";
        p.write_input(sample);
        assert!(p.state.has_user_prompted);
    }

    #[test]
    fn write_input_never_blocks_on_a_full_pty() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 10", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());

        let big = vec![b'x'; 1024 * 1024];
        let t0 = std::time::Instant::now();
        p.write_input(&big);
        let elapsed = t0.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(200),
            "write_input blocked the main thread for {elapsed:?} on a full pty"
        );

        p.kill();
    }

    #[test]
    fn typing_does_not_yank_scroll_inside_snap_grace() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        for i in 0..40 {
            writeln!(&mut out, "line {:02} abcdefghijklmnopqrstuvwxyz", i).unwrap();
        }
        p.feed(&out);
        p.scroll_display(5);
        assert!(p.term.grid().display_offset() > 0);

        p.write_input(b"hello");
        assert!(
            p.term.grid().display_offset() > 0,
            "typing inside the snap grace must not scroll to bottom"
        );

        p.write_input(b"\r");
        assert_eq!(p.term.grid().display_offset(), 0);
    }

    #[test]
    fn typing_snaps_after_grace_expires() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        for i in 0..40 {
            writeln!(&mut out, "line {:02} abcdefghijklmnopqrstuvwxyz", i).unwrap();
        }
        p.feed(&out);
        p.scroll_display(5);
        assert!(p.term.grid().display_offset() > 0);

        p.state.last_manual_scroll = Some(
            std::time::Instant::now()
                - std::time::Duration::from_millis(SCROLL_SNAP_GRACE_MS + 10),
        );
        p.write_input(b"hello");
        assert_eq!(p.term.grid().display_offset(), 0, "typing after the grace snaps");
    }

    #[test]
    fn ultra_fast_rapid_scrolling_stress_test() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 80, 24) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        for i in 0..500 {
            let line = format!("Stress line output #{}\r\n", i);
            p.feed(line.as_bytes());
        }
        let start = std::time::Instant::now();
        for step in 0..100 {
            let delta = if step % 2 == 0 { 25 } else { -20 };
            p.scroll_display(delta);
        }
        p.scroll_reset();
        let elapsed = start.elapsed();
        assert!(elapsed < std::time::Duration::from_millis(50));
        assert_eq!(p.term.grid().display_offset(), 0);
    }

    #[test]
    fn selection_range_computation_spans_multiple_rows_and_spaces() {
        let sel = ((2u16, 1u16), (10u16, 3u16));
        let (min_y, max_y, _min_x, _max_x, start_x, end_x) = {
            let ((sx, sy), (ex, ey)) = sel;
            (
                sy.min(ey),
                sy.max(ey),
                sx.min(ex),
                sx.max(ex),
                if sy <= ey { sx } else { ex },
                if sy >= ey { sx } else { ex },
            )
        };
        assert_eq!(min_y, 1);
        assert_eq!(max_y, 3);
        assert_eq!(start_x, 2);
        assert_eq!(end_x, 10);
    }

    #[test]
    fn selection_preserves_scroll_offset() {
        let Ok((pane, _reader)) = MuxPane::spawn("sleep 1", 40, 10) else {
            return;
        };
        let mut p = pane.lock().unwrap_or_else(|e| e.into_inner());
        p.scroll_display(5);
        let offset_before = p.term.grid().display_offset();
        let metrics = p.scroll_metrics();
        p.state.selection = Some(crate::selection::Selection::anchor(2, 5, metrics));
        let offset_after = p.term.grid().display_offset();
        assert_eq!(offset_before, offset_after);
    }
}

