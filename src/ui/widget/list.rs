//! 列表（蓝图「界面块」第 5、6 条）：一条一行，记号、标题、说明，右边的字贴右边不被挤掉；放不下照待办收——做完的收成
//! 一行，接着没做完的，后面放不下的收成 `… 还有 N 项`（「后台命令、子代理和侧边栏」第 4 条）。能点的整行是点击区。

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{Drawn, Hit, Look};
use crate::cells::clip;
use crate::theme;
use crate::widgets::{Item, State, Tone};

/// 排一个列表；`left` 是还剩几行。
pub(super) fn lay(items: &[Item], look: &Look, left: Option<usize>) -> Drawn {
    let mut drawn = Drawn::default();
    let Some(rows) = left.filter(|&l| l > 0 && items.len() > l) else {
        for item in items {
            push(&mut drawn, item, look);
        }
        return drawn;
    };
    drawn.cut = true;
    let done = items
        .iter()
        .filter(|i| i.state == Some(State::Done))
        .count();
    let open: Vec<&Item> = items
        .iter()
        .filter(|i| i.state != Some(State::Done))
        .collect();
    if done > 0 {
        let text = look.texts.folded.replace("{count}", &done.to_string());
        let mark = look.marks.done.clone();
        drawn.lines.push(Line::from(vec![
            Span::styled(mark, theme::dim()),
            Span::styled(text, theme::dim()),
        ]));
    }
    let room = rows.saturating_sub(usize::from(done > 0));
    let shown = if open.len() <= room {
        open.len()
    } else {
        room.saturating_sub(1)
    };
    for item in &open[..shown] {
        push(&mut drawn, item, look);
    }
    if shown < open.len() && room > 0 {
        let more = look
            .texts
            .more
            .replace("{count}", &(open.len() - shown).to_string());
        drawn.lines.push(Line::styled(more, theme::dim()));
    }
    drawn
}

/// 一条一行；带动作的记一个整行的点击区。
fn push(drawn: &mut Drawn, item: &Item, look: &Look) {
    if let Some(action) = &item.action {
        drawn.hits.push(Hit {
            row: drawn.lines.len(),
            from: 0,
            to: look.room(),
            action: action.clone(),
        });
    }
    drawn.lines.push(line(item, look));
}

/// 记号、标题、` · 说明`，右边的字贴右边；放不下先截说明、再截标题。
fn line(item: &Item, look: &Look) -> Line<'static> {
    let room = usize::from(look.room());
    let marks = look.marks;
    let base = look.tone(Tone::Normal);
    let (mark, mark_style, title_style) = match item.state {
        None => (String::new(), base, base),
        Some(State::Todo) => (marks.todo.clone(), theme::dim(), base),
        Some(State::Doing) => {
            let bold = theme::accent().add_modifier(Modifier::BOLD);
            (marks.doing.clone(), bold, bold)
        }
        Some(State::Done) => (
            marks.done.clone(),
            theme::dim(),
            theme::dim().add_modifier(Modifier::CROSSED_OUT),
        ),
        Some(State::Failed) => (marks.failed.clone(), theme::error(), base),
        Some(State::Off) => (marks.off.clone(), theme::dim(), theme::dim()),
    };
    let right = item
        .right
        .as_deref()
        .map(|r| clip(r, room.saturating_sub(mark.width()) / 2));
    let right_w = right.as_ref().map_or(0, |r| r.width() + 1);
    let left_room = room.saturating_sub(mark.width() + right_w);
    let mut spans = vec![Span::styled(mark.clone(), mark_style)];
    let title = clip(&item.title, left_room);
    let mut used = title.width();
    spans.push(Span::styled(title, title_style));
    if let Some(detail) = &item.detail {
        let space = left_room.saturating_sub(used + 3);
        if space >= 2 {
            let detail = format!(" · {}", clip(detail, space));
            used += detail.width();
            spans.push(Span::styled(detail, theme::dim()));
        }
    }
    if let Some(right) = right {
        let pad = room.saturating_sub(mark.width() + used + right.width());
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(right, theme::dim()));
    }
    Line::from(spans)
}
