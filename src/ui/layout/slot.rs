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
    /// 画好了：`area` 是给它的那一块，照字画的 `shown` 是它自己那几行露出来几行（截了的 `…` 那一行不算）。自己画的在这里
    /// 画，点击区照这里的位置记。
    fn placed(&mut self, _frame: &mut Frame, _area: Rect, _shown: usize, _ctx: &mut C) {}
}

/// 照字画出来的一行：（行、列、字），侧边栏选字用。
pub type Written = (u16, u16, String);

/// 一处挂载位画完了：排到的下一行（最后一件下面空 `gap` 行以后）、照字画出来的那几行。
pub struct Stacked {
    /// 接着往下画从这一行起；一件都没画的是 `area` 的顶上。
    pub next: u16,
    /// 照字画出来的每一行。
    pub written: Vec<Written>,
}

/// 照 `config` 的先后把 `parts` 从 `area` 顶上往下排，件和件之间空 `gap` 行；放不下的截掉，配置里不认识的名字不管
/// （读配置时提示过）。
pub fn stack<C>(
    frame: &mut Frame,
    area: Rect,
    parts: &mut [&mut dyn Part<C>],
    (config, gap): (&[Slot], u16),
    ctx: &mut C,
) -> Stacked {
    let names: Vec<&'static str> = parts.iter().map(|p| p.name()).collect();
    let (order, _) = arrange(&names, config);
    let mut y = area.y;
    let mut drawn = false;
    let mut written = Vec::new();
    for (name, cap) in order {
        let Some(part) = parts.iter_mut().find(|p| p.name() == name) else {
            continue;
        };
        let (laid, cut) = clip(part.lay(ctx, area.width), usize::from(cap));
        let height = laid.height();
        if height == 0 {
            continue;
        }
        if drawn {
            y = y.saturating_add(gap);
        }
        drawn = true;
        let tall = u16::try_from(height).unwrap_or(u16::MAX);
        let rect = Rect::new(area.x, y, area.width, tall).intersection(area);
        let shown = match laid {
            Laid::Lines(lines) => {
                let shown = lines.len().min(usize::from(rect.height));
                let own = if cut && shown == lines.len() {
                    shown - 1
                } else {
                    shown
                };
                written.extend(
                    lines
                        .iter()
                        .take(shown)
                        .zip(rect.y..)
                        .map(|(line, row)| (row, rect.x, line.to_string())),
                );
                frame.render_widget(Paragraph::new(lines), rect);
                own
            }
            Laid::Rows(_) => usize::from(rect.height),
        };
        part.placed(frame, rect, shown, ctx);
        y = y.saturating_add(tall);
    }
    let next = if drawn { y.saturating_add(gap) } else { area.y };
    Stacked { next, written }
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
