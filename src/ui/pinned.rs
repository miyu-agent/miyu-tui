//! 正文顶上钉着的那一行（排期第 16 条，2026-11-02 项目主人：「照 Claude Code」；挂载位 `top` 的出厂部件 `prompt`，蓝图
//! 「挂载位」第 4 条）：往上翻过（没跟着最新的）、视口最上面那一段对应的你说的话已经滚出去了，顶上盖一行写那句话（贴左边的
//! 竖线，竖线和字都是暗色，铺浅一档的底；发版前 check 第 6 条，2026-11-09 项目主人：原来缩进照正文、竖线照权限级别的颜色、
//! 字是亮的，缩进太大、太抢眼），点一下跳回那句话（同 `/jump`）。跟着最新的、那句话还露着的不画。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::row_cache::Rows;
use crate::app::{Screen, Seen};
use crate::config::slots::{TOP_PARTS, arrange};
use crate::theme;
use crate::transcript::Kind;

/// 钉着的那一行连它下面空的一行一共盖住正文最上面几行（2026-11-11 项目主人：「这里应该空一行，也就是从第三行开始」，
/// 原来正文紧贴在那句话下面）。
pub(super) const ROWS: usize = 2;

/// 画在正文区 `area` 的第一行上，下面再空一行；`first` 是这一帧露出来的第一行。
pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    (rows, first): (&Rows, usize),
    seen: &Seen,
    screen: &mut Screen,
) {
    screen.areas.pinned = Rect::default();
    screen.pinned = None;
    screen.current_said = current(rows, first, seen, screen.view.top.is_none());
    let shown = arrange(&TOP_PARTS, &seen.config.layout.top).0;
    if screen.view.top.is_none()
        || area.height < 4
        || !shown.iter().any(|(n, _)| *n == TOP_PARTS[0])
    {
        return;
    }
    let t = &seen.talk.transcript;
    let Some(at) = rows.entry_at(first) else {
        return;
    };
    let Some(k) = (0..=at.min(t.entries.len().saturating_sub(1)))
        .rev()
        .find(|&i| t.entries[i].kind == Kind::User && !t.entries[i].hidden)
    else {
        return;
    };
    // 那句话还露着（还有一行在视口里）：不钉，免得同一句写两遍。
    if rows.end_of(k).is_none_or(|end| end > first) {
        return;
    }
    let entry = &t.entries[k];
    let row = Rect::new(area.x, area.y, area.width, 1);
    // 先把这一行清空再写：只盖字的话，句子后面露出底下正文那一行剩下的字（2026-11-05 验收）。
    frame.render_widget(ratatui::widgets::Clear, row);
    frame.buffer_mut().set_style(row, theme::panel());
    let bar = &seen.config.layout.user_bar;
    let lead = format!("{} ", bar.trim_end());
    let room = area
        .width
        .saturating_sub(u16::try_from(lead.width()).unwrap_or(0));
    let said = entry
        .text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();
    let line = Line::from(vec![
        Span::styled(lead, theme::dim()),
        Span::styled(super::rows::clip(said.trim(), room), theme::dim()),
    ]);
    frame
        .buffer_mut()
        .set_line(area.x, area.y, &line, area.width);
    // 下面空一行（照正文的底色），正文从第三行露起。
    let gap = Rect::new(area.x, area.y + 1, area.width, 1);
    frame.render_widget(ratatui::widgets::Clear, gap);
    screen.areas.pinned = row;
    screen.pinned = Some(entry.id);
}

/// 当前这一句（排期第 44 条）：跟着最新的（`following`）是最后一句你说的话；往上翻着的是视口第一行那一条往前最近的一句。
fn current(rows: &Rows, first: usize, seen: &Seen, following: bool) -> Option<u64> {
    let entries = &seen.talk.transcript.entries;
    let said =
        |i: &usize| entries[*i].kind == Kind::User && !entries[*i].hidden && !entries[*i].queued;
    let last = if following {
        entries.len().checked_sub(1)?
    } else {
        rows.entry_at(first)?.min(entries.len().checked_sub(1)?)
    };
    let k = (0..=last).rev().find(said)?;
    Some(entries[k].id)
}
