//! 引导中间几步左边那一块（蓝图 `tui.md`「第一次打开的引导」第 11a 条，2026-10-15 项目主人）：照 [`Oobe::wanted_figure`]
//! 画吉祥物、人格的头像，或者什么都不画；要画的一变就当场换——吉祥物往左走出画面（走出去的那一截截掉），头像在原位从上往下
//! 一行行露出来，换回吉祥物的从左边走回来。头像不跟着换步跳，也不转头。
//!
//! [`Oobe::wanted_figure`]: crate::oobe::Oobe::wanted_figure

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Position, Rect};

use super::Parts;
use super::figure::{self, Extra};

/// 画左边那一块。`rect` 是吉祥物站的地方（换步时跳起来的，往上挪了 `lift` 行），`area` 是整屏。
pub fn draw(
    frame: &mut Frame,
    (area, rect, lift): (Rect, Rect, u16),
    p: &mut Parts<'_>,
    gaze: (Option<Position>, f64),
    extra: Extra,
    now: Instant,
) {
    let avatar = p
        .oobe
        .persona
        .finale_avatar(|version| p.media.avatars.file(version));
    let wanted = p.oobe.wanted_figure(avatar.is_some());
    p.oobe.figure.want(wanted, now);
    let phase = p.oobe.figure.phase(now, &p.config.oobe.swap);
    if let Some(out) = phase.mascot {
        // 往左走到整个出了画面：走了几列，出了左边的那一截截掉。
        let room = rect.x.saturating_sub(area.x);
        let span = f64::from(room + rect.width);
        let shift = (out.clamp(0.0, 1.0) * span).round() as u16;
        let (x, skip) = if shift <= room {
            (rect.x - shift, 0)
        } else {
            (area.x, shift - room)
        };
        if skip < rect.width {
            let moved = Rect {
                x,
                width: rect.width - skip,
                ..rect
            };
            let look = p.config.mascot.clone();
            figure::draw(frame, moved, &look, p, gaze, Extra { skip, ..extra });
        }
    }
    if let (Some(shown), Some(path)) = (phase.avatar, avatar) {
        let still = Rect {
            y: rect.y + lift,
            ..rect
        };
        crate::ui::avatar_view::draw_file(frame, &p.media.figures, still, &path, shown);
    }
}
