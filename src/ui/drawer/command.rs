//! 确认要跑的命令（蓝图「确认和提问的抽屉」第 3 条）：问题行、提示下面空一行，命令自成一块，每行前面一根暗色竖线（主题的
//! `quote` 记号），`$ ` 打头，折行缩进两格对齐在 `$ ` 后面。超过窗口（最多 8 行，抽屉放不下时少露几行，
//! `Drawer::command_window`）的只露一段，块下面暗色写还有几行；
//! 滚轮、`Ctrl+J` `Ctrl+K`、`Ctrl+↑` `Ctrl+↓` 滚（2026-10-22 项目主人：命令太长把问题顶没了，照选「全文放在可以滚的一块
//! 里」；同一天看了样子又定「标题和提示在上、命令一块缩进」，原来紧贴着没有空行）。

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{View, clip};
use crate::drawer::{Drawer, Texts};
use crate::input::pieces;
use crate::theme;

/// 命令排成的一行行（不带竖线）：每一行原文的第一截 `$ ` 打头，折下来的缩进两格。`width` 是整个抽屉的宽。
pub fn rows(command: &str, width: u16) -> Vec<String> {
    let bar = u16::try_from(crate::sheet::mark(|m| &m.quote).width()).unwrap_or(2);
    let room = width.saturating_sub(bar + 2).max(1);
    let mut out = Vec::new();
    // 屏上要和真正要跑的一样：看不见的字换成记号再排（发布前审查 T2-2，`tabs::shown`）。
    let command = crate::tabs::shown(command);
    for line in command.lines() {
        let mut lead = "$ ";
        let parts = pieces(line, room);
        if parts.is_empty() {
            out.push(lead.trim_end().to_string());
        }
        for (piece, _) in parts {
            out.push(format!("{lead}{piece}"));
            lead = "  ";
        }
    }
    out
}

/// 接到抽屉的问题下面：空一行，放得下的全写，放不下的照滚到的地方露一窗，块下面写上面、下面还有几行。
pub fn push(d: &Drawer, texts: &Texts, width: u16, v: &mut View) {
    let Some((_, command)) = &d.command else {
        return;
    };
    let rows = rows(command, width);
    let total = rows.len();
    let window = d.command_window;
    let top = d.command_top.min(total.saturating_sub(window));
    let bar = crate::sheet::mark(|m| &m.quote);
    v.push(Line::default(), None);
    for row in rows.iter().skip(top).take(window) {
        let spans = vec![
            Span::styled(bar.clone(), theme::faint()),
            Span::styled(row.clone(), theme::dim()),
        ];
        v.push(Line::from(spans), None);
    }
    let below = total.saturating_sub(top + window);
    let mut more = Vec::new();
    if top > 0 {
        more.push(texts.command_above.replace("{n}", &top.to_string()));
    }
    if below > 0 {
        more.push(texts.command_below.replace("{n}", &below.to_string()));
    }
    if !more.is_empty() {
        let more = more.join(&crate::sheet::sep());
        v.push(Line::styled(clip(&more, width), theme::faint()), None);
    }
}
