
pub mod fuzzy;

use std::sync::{Arc, Mutex};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::pane::MuxPane;
use fuzzy::fuzzy_match;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Tabs,
    Files,
    Output,
}

impl Scope {
    pub const ALL: [Self; 4] = [Self::All, Self::Tabs, Self::Files, Self::Output];

    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Tabs,
            Self::Tabs => Self::Files,
            Self::Files => Self::Output,
            Self::Output => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Tabs => "Tabs",
            Self::Files => "Files",
            Self::Output => "Scrollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Tab { idx: usize },
    File { path: String },
    Output { pane_idx: usize, line: String },
}

#[derive(Debug, Clone)]
pub struct FinderItem {
    pub kind: ItemKind,
    pub label: String,
    pub detail: String,
    pub icon: &'static str,
    pub score: i64,
    pub match_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct FinderState {
    pub query: String,
    pub scope: Scope,
    pub selected: usize,
    pub cached_files: Vec<String>,
    pub results: Vec<FinderItem>,
}

impl FinderState {
    pub fn new(cwd: Option<&str>) -> Self {
        let files = load_project_files(cwd);
        Self {
            query: String::new(),
            scope: Scope::All,
            selected: 0,
            cached_files: files,
            results: Vec::new(),
        }
    }

    pub fn update_results(&mut self, panes: &[Arc<Mutex<MuxPane>>]) {
        let mut items: Vec<FinderItem> = Vec::new();
        let q = self.query.trim();

        if self.scope == Scope::All || self.scope == Scope::Tabs {
            for (idx, pane) in panes.iter().enumerate() {
                if let Ok(p) = pane.lock() {
                    let title = p.state.title.clone();
                    let session_tag = p.state.session_id.as_deref().unwrap_or("");
                    let label = format!("{}. {}", idx + 1, title);
                    let detail = if !session_tag.is_empty() {
                        format!("Session: {}", session_tag)
                    } else {
                        "Active Terminal".into()
                    };

                    if let Some(m) = fuzzy_match(q, &label) {
                        items.push(FinderItem {
                            kind: ItemKind::Tab { idx },
                            label,
                            detail,
                            icon: "",
                            score: m.score + 100,
                            match_indices: m.indices,
                        });
                    }
                }
            }
        }

        if self.scope == Scope::All || self.scope == Scope::Files {
            for path in &self.cached_files {
                if let Some(m) = fuzzy_match(q, path) {
                    items.push(FinderItem {
                        kind: ItemKind::File { path: path.clone() },
                        label: path.clone(),
                        detail: "Project file".into(),
                        icon: "",
                        score: m.score,
                        match_indices: m.indices,
                    });
                }
            }
        }

        if self.scope == Scope::All || self.scope == Scope::Output {
            if !q.is_empty() {
                for (pane_idx, pane) in panes.iter().enumerate() {
                    if let Ok(p) = pane.lock() {
                        let bottom = p.bottom_text(40);
                        for line in bottom.lines() {
                            let trimmed = line.trim();
                            if trimmed.is_empty() || trimmed.len() < 3 {
                                continue;
                            }
                            if let Some(m) = fuzzy_match(q, trimmed) {
                                items.push(FinderItem {
                                    kind: ItemKind::Output {
                                        pane_idx,
                                        line: trimmed.to_string(),
                                    },
                                    label: trimmed.to_string(),
                                    detail: format!("Terminal {}", pane_idx + 1),
                                    icon: "",
                                    score: m.score - 20,
                                    match_indices: m.indices,
                                });
                            }
                        }
                    }
                }
            }
        }

        items.sort_by(|a, b| b.score.cmp(&a.score));
        items.truncate(50);

        self.results = items;
        if self.selected >= self.results.len() {
            self.selected = self.results.len().saturating_sub(1);
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
        }
    }

    pub fn next_scope(&mut self, panes: &[Arc<Mutex<MuxPane>>]) {
        self.scope = self.scope.next();
        self.update_results(panes);
    }
}

fn load_project_files(cwd: Option<&str>) -> Vec<String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("ls-files");
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    if let Ok(out) = cmd.output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.lines().take(500).map(|l| l.to_string()).collect();
        }
    }
    Vec::new()
}

pub fn render_finder(frame: &mut Frame, area: Rect, finder: &FinderState, pal: &Palette) {
    let width = (area.width.saturating_sub(12)).min(80).max(40);
    let height = (area.height.saturating_sub(6)).min(20).max(10);

    let popup = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    crate::ui::widget::render_modal_shell(frame, area, width, height, pal);

    let buf = frame.buffer_mut();

    let input_y = popup.y + 1;
    let query_line = Line::from(vec![
        Span::styled("   ", Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)),
        Span::styled(&finder.query, Style::default().fg(pal.text).add_modifier(Modifier::BOLD)),
        Span::styled("▏", Style::default().fg(pal.accent)),
    ]);
    buf.set_line(popup.x + 2, input_y, &query_line, width.saturating_sub(4));

    let scope_y = popup.y + 2;
    let mut scope_spans: Vec<Span> = Vec::new();
    for scope in Scope::ALL {
        let active = scope == finder.scope;
        if active {
            scope_spans.push(Span::styled(
                format!(" {} ", scope.label()),
                Style::default().bg(pal.accent).fg(pal.panel_bg).add_modifier(Modifier::BOLD),
            ));
        } else {
            scope_spans.push(Span::styled(
                format!(" {} ", scope.label()),
                Style::default().fg(pal.overlay1),
            ));
        }
        scope_spans.push(Span::raw(" "));
    }
    let scope_line = Line::from(scope_spans);
    buf.set_line(popup.x + 2, scope_y, &scope_line, width.saturating_sub(4));

    let sep_y = popup.y + 3;
    let sep_str: String = "─".repeat(width.saturating_sub(4) as usize);
    buf.set_line(popup.x + 2, sep_y, &Line::from(Span::styled(sep_str, Style::default().fg(pal.surface0))), width.saturating_sub(4));

    let list_start_y = popup.y + 4;
    let visible_rows = height.saturating_sub(6) as usize;

    if finder.results.is_empty() {
        let empty_msg = Line::from(Span::styled(
            " No matching results",
            Style::default().fg(pal.subtext0),
        ));
        buf.set_line(popup.x + 3, list_start_y + 1, &empty_msg, width.saturating_sub(6));
    } else {
        for (i, item) in finder.results.iter().take(visible_rows).enumerate() {
            let row_y = list_start_y + i as u16;
            let is_sel = i == finder.selected;

            let icon_style = if is_sel {
                Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(pal.subtext0)
            };

            let label_style = if is_sel {
                Style::default().fg(pal.text).bg(pal.surface0).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(pal.text)
            };

            let detail_style = Style::default().fg(pal.subtext0);

            let row_line = Line::from(vec![
                Span::styled(if is_sel { " ❯ " } else { "   " }, icon_style),
                Span::styled(format!("{} ", item.icon), icon_style),
                Span::styled(&item.label, label_style),
                Span::raw("  "),
                Span::styled(&item.detail, detail_style),
            ]);

            buf.set_line(popup.x + 2, row_y, &row_line, width.saturating_sub(4));
        }
    }

    let footer_y = popup.y + height - 2;
    let hints = Line::from(vec![
        Span::styled("Enter", Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" open · ", Style::default().fg(pal.overlay1)),
        Span::styled("Tab", Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" scope · ", Style::default().fg(pal.overlay1)),
        Span::styled("Esc", Style::default().fg(pal.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" close", Style::default().fg(pal.overlay1)),
    ]);
    buf.set_line(popup.x + 2, footer_y, &hints, width.saturating_sub(4));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_scope_cycles_correctly() {
        let s = Scope::All;
        assert_eq!(s.next(), Scope::Tabs);
        assert_eq!(s.next().next(), Scope::Files);
        assert_eq!(s.next().next().next(), Scope::Output);
        assert_eq!(s.next().next().next().next(), Scope::All);
    }

    #[test]
    fn finder_navigation_stays_in_bounds() {
        let mut finder = FinderState {
            query: "".into(),
            scope: Scope::All,
            selected: 0,
            cached_files: vec!["file1.rs".into(), "file2.rs".into()],
            results: vec![
                FinderItem {
                    kind: ItemKind::File { path: "file1.rs".into() },
                    label: "file1.rs".into(),
                    detail: "".into(),
                    icon: "",
                    score: 10,
                    match_indices: vec![],
                },
                FinderItem {
                    kind: ItemKind::File { path: "file2.rs".into() },
                    label: "file2.rs".into(),
                    detail: "".into(),
                    icon: "",
                    score: 5,
                    match_indices: vec![],
                },
            ],
        };

        finder.move_down();
        assert_eq!(finder.selected, 1);
        finder.move_down();
        assert_eq!(finder.selected, 1);
        finder.move_up();
        assert_eq!(finder.selected, 0);
        finder.move_up();
        assert_eq!(finder.selected, 0);
    }
}

