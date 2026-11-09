//! 挂载位（蓝图 `tui.md`「挂载位」第 1～5 条，反馈 1102 第 13 条）：一处地方挂几件部件，照配置排先后、藏、截行数，件和件
//! 之间空几行。挂载位不知道挂的是哪几件，部件也不知道挂在哪：加部件不改这里。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::config::slots::{Slot, arrange};
use crate::theme;

/// 一件部件给定宽度排出来的样子。
pub enum Laid {
    /// 照字画：挂载位画、截（截了的最后一行写 `…`）。
    Lines(Vec<Line<'static>>),
    /// 自己画（头像、图），占几行；截了的给它矮一点的那一块。
    Rows(u16),
}

impl Laid {
    /// 占几行；0 是这一回没东西画，不占行、也不留空行。
    fn height(&self) -> usize {
        match self {
            Laid::Lines(lines) => lines.len(),
            Laid::Rows(rows) => usize::from(*rows),
        }
    }
}

/// 一件部件。`C` 是这一处画的时候要用的东西（侧边栏是 `sidebar::parts::Side`）。
pub trait Part<C> {
    /// 部件名：配置里写的那个（蓝图「挂载位」第 2、4 条）。
    fn name(&self) -> &'static str;
    /// 照宽度排。
    fn lay(&mut self, ctx: &C, width: u16) -> Laid;
    /// 画好了：`area` 是给它露出来的那一块，`first` 是这一块的第一行是它自己的第几行（往下滚过、上面几行滚出去的不是 0），
    /// 照字画的 `shown` 是它自己那几行露出来几行（截了的 `…` 那一行不算）。自己画的只在整块露出来时叫到（滚出去一截的
    /// 不画），在这里画；点击区照这里的位置记。
    fn placed(&mut self, _frame: &mut Frame, _area: Rect, _rows: (usize, usize), _ctx: &mut C) {}
    /// 照字画的第 `row` 行是不是上一行放不下折下来的（软折行）：侧边栏复制时接回上一行，不换行（发版前 check 29）。
    fn continues(&self, _row: usize) -> bool {
        false
    }
}

/// 照字画出来的一行：（行、列、字、是不是上一行折下来的），侧边栏选字用。
pub type Written = (u16, u16, String, bool);

/// 一处挂载位画完了：排到的下一行（最后一件下面空 `gap` 行以后）、照字画出来的那几行、限过的偏移、一共多高。
pub struct Stacked {
    /// 接着往下画从这一行起（照滚过的算）；一件都没画的是 `area` 的顶上。
    pub next: u16,
    /// 照字画出来的每一行。
    pub written: Vec<Written>,
    /// 从第几行露起：交进来的限在 0 到「一共多高减一窗」之间。
    pub top: u16,
    /// 排好的一共多高（侧边栏照它画滚动条）。
    pub total: usize,
}

/// 照 `config` 的先后把 `parts` 从 `area` 顶上往下排，件和件之间空 `gap` 行，从第 `top` 行露起（排期第 14 条：侧边栏
/// 像正文那样滚）；露不出来的截掉，配置里不认识的名字不管（读配置时提示过）。先把每件排好、算出一共多高、限住 `top`，
/// 再画露出来的那一截。
pub fn stack<C>(
    frame: &mut Frame,
    area: Rect,
    parts: &mut [&mut dyn Part<C>],
    (config, gap, top): (&[Slot], u16, u16),
    ctx: &mut C,
) -> Stacked {
    let names: Vec<&'static str> = parts.iter().map(|p| p.name()).collect();
    let (order, _) = arrange(&names, config);
    // 先排：（第几件、排好截好的、截没截）。
    let mut laid = Vec::new();
    for (name, cap) in order {
        let Some(at) = parts.iter().position(|p| p.name() == name) else {
            continue;
        };
        let (one, cut) = clip(parts[at].lay(ctx, area.width), usize::from(cap));
        if one.height() > 0 {
            laid.push((at, one, cut));
        }
    }
    let gap = usize::from(gap);
    let total: usize = laid.iter().map(|(_, one, _)| one.height()).sum::<usize>()
        + gap * laid.len().saturating_sub(1);
    let room = usize::from(area.height);
    let top = usize::from(top).min(total.saturating_sub(room));
    let mut written = Vec::new();
    // 照排好的算的那一行（不看滚了多少）。
    let mut y = 0;
    for (k, (at, one, cut)) in laid.into_iter().enumerate() {
        if k > 0 {
            y += gap;
        }
        let height = one.height();
        // 这一件露出来的是它自己的第 `from` 到 `to` 行。
        let from = top.saturating_sub(y).min(height);
        let to = (top + room).saturating_sub(y).min(height);
        if from < to {
            let row = area.y + u16::try_from(y + from - top).unwrap_or(u16::MAX);
            let tall = u16::try_from(to - from).unwrap_or(u16::MAX);
            let rect = Rect::new(area.x, row, area.width, tall);
            match one {
                Laid::Lines(lines) => {
                    let own = if cut && to == height {
                        to - from - 1
                    } else {
                        to - from
                    };
                    let shown: Vec<Line<'static>> = lines[from..to].to_vec();
                    let part = &parts[at];
                    written.extend(shown.iter().zip(rect.y..).zip(from..).map(
                        |((line, row), own)| (row, rect.x, line.to_string(), part.continues(own)),
                    ));
                    frame.render_widget(Paragraph::new(shown), rect);
                    parts[at].placed(frame, rect, (from, own), ctx);
                }
                Laid::Rows(_) if from == 0 && to == height => {
                    parts[at].placed(frame, rect, (0, height), ctx);
                }
                Laid::Rows(_) => {}
            }
        }
        y += height;
    }
    let end = if total > 0 { total + gap } else { 0 };
    let next = area.y + u16::try_from(end.saturating_sub(top)).unwrap_or(u16::MAX);
    let top = u16::try_from(top).unwrap_or(u16::MAX);
    Stacked {
        next,
        written,
        top,
        total,
    }
}

/// 只排字的挂载位（输入框上面那一块：高度要先算出来再定位，「挂载位」第 4 条 SL-2）：照配置排先后、藏、截行数，件和件
/// 之间空 `gap` 行，交回排好的行；没东西的件不占行。
pub fn column(
    mut parts: Vec<(&'static str, Vec<Line<'static>>)>,
    (config, gap): (&[Slot], u16),
) -> Vec<Line<'static>> {
    let names: Vec<&'static str> = parts.iter().map(|(name, _)| *name).collect();
    let (order, _) = arrange(&names, config);
    let mut out = Vec::new();
    for (name, cap) in order {
        let Some((_, lines)) = parts.iter_mut().find(|(n, _)| *n == name) else {
            continue;
        };
        let (Laid::Lines(lines), _) = clip(Laid::Lines(std::mem::take(lines)), usize::from(cap))
        else {
            continue;
        };
        if lines.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.extend((0..gap).map(|_| Line::default()));
        }
        out.extend(lines);
    }
    out
}

/// 照 `cap` 截（0 是不限）：照字画的留 `cap - 1` 行，最后一行写暗色的 `…`（主题的 `marks.ellipsis`，交回的 `true`）；自己画的给 `cap` 行。
fn clip(laid: Laid, cap: usize) -> (Laid, bool) {
    match laid {
        Laid::Lines(mut lines) if cap > 0 && lines.len() > cap => {
            lines.truncate(cap - 1);
            lines.push(Line::styled(crate::sheet::ellipsis(), theme::dim()));
            (Laid::Lines(lines), true)
        }
        Laid::Rows(rows) if cap > 0 => (
            Laid::Rows(rows.min(u16::try_from(cap).unwrap_or(u16::MAX))),
            false,
        ),
        laid => (laid, false),
    }
}

#[cfg(test)]
mod tests;
