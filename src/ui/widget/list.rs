//! 列表（蓝图「界面块」第 5、6 条）：一条一行，记号、标题、说明，右边的字贴右边不被挤掉；放不下照待办收——没做完的在前，
//! 放不下的收成 `… 还有 N 项`，做完的收成最底下一行（「后台命令、子代理和侧边栏」第 4 条；2026-10-18 项目主人定和待办一样挪到
//! 最底下）。能点的整行是点击区。

use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{Drawn, Hit, Look};
use crate::cells::clip;
use crate::theme;
use crate::widgets::{Item, Run, State, Tone};

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
    if done > 0 {
        let text = look.texts.folded.replace("{count}", &done.to_string());
        let mark = look.marks.done.clone();
        drawn.lines.push(Line::from(vec![
            Span::styled(mark, theme::dim()),
            Span::styled(text, theme::dim()),
        ]));
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
    let right = right_spans(&item.right, room.saturating_sub(mark.width()) / 2, look);
    let right_width: usize = right.iter().map(Span::width).sum();
    let right_w = if right.is_empty() { 0 } else { right_width + 1 };
    let left_room = room.saturating_sub(mark.width() + right_w);
    let mut spans = vec![Span::styled(mark.clone(), mark_style)];
    let title = clip(&item.title, left_room);
    let mut used = title.width();
    spans.push(Span::styled(title, title_style));
    if let Some(detail) = &item.detail {
        let space = left_room.saturating_sub(used + 3);
        if space >= 2 {
            let detail = format!("{}{}", crate::sheet::sep(), clip(detail, space));
            used += detail.width();
            spans.push(Span::styled(detail, theme::dim()));
        }
    }
    if !right.is_empty() {
        let pad = room.saturating_sub(mark.width() + used + right_width);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.extend(right);
    }
    Line::from(spans)
}

/// 右边的字：几段之间空一格，每段照自己的语气（只写了一句字的照暗，核心 X-15 补）；最多 `cap` 列，放不下的从后往前
/// 丢段，一段都放不下的截掉加 `…`。
fn right_spans(runs: &[Run], cap: usize, look: &Look) -> Vec<Span<'static>> {
    let style = |run: &Run| run.tone.map_or_else(theme::dim, |t| look.tone(t));
    let mut out = Vec::new();
    let mut used = 0;
    for run in runs {
        let gap = usize::from(!out.is_empty());
        let w = run.text.width();
        if used + gap + w > cap {
            if out.is_empty() {
                out.push(Span::styled(clip(&run.text, cap), style(run)));
            }
            break;
        }
        if gap > 0 {
            out.push(Span::raw(" "));
        }
        out.push(Span::styled(run.text.clone(), style(run)));
        used += gap + w;
    }
    out
}
