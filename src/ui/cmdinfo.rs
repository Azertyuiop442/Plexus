
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::Palette;
use crate::ui::text::width as display_width;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub depth: usize,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct CmdInspectState {
    pub pane_idx: usize,
    pub title: String,
    pub cwd: String,
    pub procs: Vec<ProcessInfo>,
    pub scroll: usize,
}

impl CmdInspectState {
    pub fn collect_for_pid(pane_idx: usize, title: String, cwd: String, root_pid: u32) -> Self {
        let procs = collect_process_tree(root_pid);
        Self {
            pane_idx,
            title,
            cwd,
            procs,
            scroll: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, p: &Palette) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let dim_block = Block::new().style(Style::new().bg(crate::theme::BG));
        frame.render_widget(dim_block, area);

        let w = area.width.saturating_sub(8).clamp(50, 100).min(area.width);
        let h = (self.procs.len() as u16 + 6)
            .clamp(10, area.height.saturating_sub(4))
            .min(area.height);

        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let popup = Rect::new(x, y, w, h);

        frame.render_widget(Clear, popup);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(p.accent).bg(p.surface0))
            .style(Style::new().bg(p.surface0));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        if inner.height < 4 {
            return;
        }

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" Running Now", Style::new().fg(p.accent).bold()),
                Span::styled(
                    format!("   Terminal #{} ({})", self.pane_idx + 1, self.title),
                    Style::new().fg(p.subtext0),
                ),
            ])),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );

        let sep_line = "─".repeat(inner.width as usize);
        frame.render_widget(
            Paragraph::new(Span::styled(sep_line, Style::new().fg(p.surface1))),
            Rect::new(inner.x, inner.y + 1, inner.width, 1),
        );

        let body_y = inner.y + 2;
        let body_h = inner.height.saturating_sub(3);
        let body = Rect::new(inner.x, body_y, inner.width, body_h);
        let cap = body.height as usize;

        if self.procs.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " No child process running (idle shell)",
                    Style::new().fg(p.overlay0),
                )),
                Rect::new(body.x, body.y, body.width, 1),
            );
        } else {
            let max_scroll = self.procs.len().saturating_sub(cap);
            let scroll = self.scroll.min(max_scroll);
            for (i, proc) in self.procs.iter().skip(scroll).take(cap).enumerate() {
                let cur_y = body.y + i as u16;
                let (marker, fg, tag_bg) = if proc.depth == 0 {
                    ("shell", p.overlay0, p.surface0)
                } else if proc.depth == 1 {
                    ("agent", p.accent, p.surface1)
                } else {
                    ("tool", p.green, p.surface1)
                };

                let indent = "  ".repeat(proc.depth.min(4));
                let head = format!(" {indent}[{marker}]");
                let pid_str = format!(" {:>6} ", proc.pid);
                let used = (display_width(&head) + display_width(&pid_str)) as u16;
                let cmd_w = body.width.saturating_sub(used) as usize;

                let cmd_display = if display_width(&proc.command) > cmd_w && cmd_w > 2 {
                    let mut s = String::new();
                    for c in proc.command.chars() {
                        if display_width(&s) + 1 >= cmd_w {
                            break;
                        }
                        s.push(c);
                    }
                    format!("{s}…")
                } else {
                    proc.command.clone()
                };

                let line = Line::from(vec![
                    Span::styled(head, Style::new().fg(fg).bg(tag_bg).bold()),
                    Span::styled(pid_str, Style::new().fg(p.overlay1)),
                    Span::styled(
                        cmd_display,
                        Style::new().fg(if proc.depth == 0 { p.subtext0 } else { p.text }),
                    ),
                ]);

                frame.render_widget(Paragraph::new(line), Rect::new(body.x, cur_y, body.width, 1));
            }
        }

        let footer_y = inner.bottom().saturating_sub(1);
        let short_cwd = if self.cwd.len() > (inner.width as usize).saturating_sub(30) {
            let mut s = self.cwd.clone();
            if let Some(home) = std::env::var("HOME").ok() {
                if s.starts_with(&home) {
                    s = s.replacen(&home, "~", 1);
                }
            }
            s
        } else {
            self.cwd.clone()
        };

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!(" 📁 {short_cwd}"), Style::new().fg(p.overlay1)),
                Span::styled(
                    "   ↑↓/jk Scroll · r Refresh · Esc Close",
                    Style::new().fg(p.subtext0),
                ),
            ])),
            Rect::new(inner.x, footer_y, inner.width, 1),
        );
    }
}

fn collect_process_tree(root_pid: u32) -> Vec<ProcessInfo> {
    if root_pid == 0 {
        return Vec::new();
    }

    #[cfg(target_os = "macos")]
    {

        let Ok(out) = std::process::Command::new("ps")
            .args(["-ax", "-o", "pid=,ppid=,command="])
            .output()
        else {
            return Vec::new();
        };
        let raw = String::from_utf8_lossy(&out.stdout);
        parse_ps_tree(&raw, root_pid)
    }

    #[cfg(target_os = "linux")]
    {
        let Ok(out) = std::process::Command::new("ps")
            .args(["-ax", "-o", "pid=,ppid=,command="])
            .output()
        else {
            return Vec::new();
        };
        let raw = String::from_utf8_lossy(&out.stdout);
        parse_ps_tree(&raw, root_pid)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}

fn parse_ps_tree(ps_output: &str, root_pid: u32) -> Vec<ProcessInfo> {

    let mut all_procs: std::collections::HashMap<u32, (u32, String)> = std::collections::HashMap::new();

    for line in ps_output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(pid_str) = parts.next() else { continue };
        let Some(ppid_str) = parts.next() else { continue };
        let Ok(pid) = pid_str.parse::<u32>() else { continue };
        let Ok(ppid) = ppid_str.parse::<u32>() else { continue };
        let command = parts.collect::<Vec<&str>>().join(" ");
        all_procs.insert(pid, (ppid, command));
    }

    let mut result = Vec::new();
    let mut stack = vec![(root_pid, 0usize)];

    while let Some((pid, depth)) = stack.pop() {
        if let Some((_, cmd)) = all_procs.get(&pid) {
            result.push(ProcessInfo {
                pid,
                depth,
                command: cmd.clone(),
            });

            let mut children: Vec<u32> = all_procs
                .iter()
                .filter(|(_, (ppid, _))| *ppid == pid)
                .map(|(&c_pid, _)| c_pid)
                .collect();
            children.sort();
            for child in children.into_iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ps_tree_extracts_hierarchy() {
        let sample = "
          100     1 /bin/zsh -l
          200   100 node /bin/commandcode
          300   200 cargo test --all
          400     1 /bin/launchd
        ";

        let tree = parse_ps_tree(sample, 100);
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0].pid, 100);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[1].pid, 200);
        assert_eq!(tree[1].depth, 1);
        assert_eq!(tree[2].pid, 300);
        assert_eq!(tree[2].depth, 2);
    }
}

