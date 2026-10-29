//! 形象那一块（蓝图 `tui.md`「空会话的首页」第 10 条，2026-10-11 项目主人）：人格有头像的画头像，图照比例放进这一块、
//! 居中，照画图那一套在后台做，还没做好的先空着；没有头像、画不了的画吉祥物。开关关着的头像、吉祥物都不画（2026-10-29
//! 项目主人：原来头像不受开关管）。

use std::cell::RefCell;
use std::path::Path;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui_image::sliced::{SignedPosition, SlicedImage};

use crate::app::{Avatar, MascotState, Place, Screen, Seen};
use crate::figures::{Figures, Look};
use crate::markdown::FigureKind;

/// 画头像画成了没有。
enum Shown {
    /// 画了。
    Yes,
    /// 还在做：这一块先空着。
    Waiting,
    /// 读不出来、终端显示不了：退回吉祥物。
    Unable,
}

/// 这一处有没有那一块：「显示形象」的开关开着（头像、吉祥物都归它管，2026-10-29 项目主人：关了形象头像也不该显示）。
pub fn slot_shown(seen: &Seen, place: Place) -> bool {
    mascot_on(seen, place)
}

/// 吉祥物在这一处画不画（首页、侧边栏各一个开关）。
fn mascot_on(seen: &Seen, place: Place) -> bool {
    match place {
        Place::Home => seen.config.layout.mascot_home,
        Place::Sidebar => seen.config.layout.mascot_sidebar,
    }
}

/// 在 `rect`（吉祥物那么大的一块）里画：头像，或者吉祥物。`far` 照吉祥物的（在侧边栏）。
pub fn draw(
    frame: &mut Frame,
    rect: Rect,
    (seen, screen, mascot): (&Seen, &Screen, &mut MascotState),
    place: Place,
    far: bool,
) {
    let Some(avatar) = wanted(mascot_on(seen, place), seen.avatar(place)) else {
        return;
    };
    let shown = match avatar {
        Avatar::File(path) => {
            let at = match place {
                Place::Home => "home",
                Place::Sidebar => "sidebar",
            };
            picture(frame, (&seen.media.figures, at), rect, &path, 1.0)
        }
        Avatar::Waiting => Shown::Waiting,
        Avatar::None => Shown::Unable,
    };
    if matches!(shown, Shown::Unable) {
        super::mascot_view::draw(frame, rect, (seen, screen, mascot), far);
    }
}

/// 这一处画什么：开关（`on`）关着的什么都不画，头像也不画；开着的照头像，没有头像的（`Avatar::None`）画吉祥物。
fn wanted(on: bool, avatar: Avatar) -> Option<Avatar> {
    on.then_some(avatar)
}

/// 引导「好了」那一屏（「第一次打开的引导」第 11 条）：把头像照比例放进 `rect`、居中画，从上往下露出 `shown` 成（0 到 1，
/// 2026-10-14 项目主人定的揭幕）。画了、还在做的交回 `true`（那一块不再画吉祥物），读不出来、终端显示不了的交回 `false`。
pub fn draw_file(
    frame: &mut Frame,
    figures: &RefCell<Figures>,
    rect: Rect,
    path: &Path,
    shown: f64,
) -> bool {
    !matches!(
        picture(frame, (figures, "oobe"), rect, path, shown),
        Shown::Unable
    )
}

/// 露出几行：一共 `rows` 行露 `shown` 成，往上取整（刚开始就露一行，到 1 整张）。
fn revealed(rows: u16, shown: f64) -> u16 {
    ((f64::from(rows) * shown.clamp(0.0, 1.0)).ceil() as u16).min(rows)
}

/// 把图照比例放进 `rect`、居中画，从上往下露出 `shown` 成（首页、侧边栏整张：1）。`at` 是画在哪一处：换成另一张、新的
/// 还在做时先照旧画这一处上一张（[`Figures::look_held`]）。
fn picture(
    frame: &mut Frame,
    (figures, at): (&RefCell<Figures>, &'static str),
    rect: Rect,
    path: &Path,
    shown: f64,
) -> Shown {
    let mut figures = figures.borrow_mut();
    let source = path.display().to_string();
    let look = figures.look_held(
        at,
        FigureKind::Preview,
        &source,
        (None, None),
        (rect.width, rect.height),
    );
    let (key, rows) = match look {
        Look::Ready { key, rows } => (key, rows.min(rect.height)),
        Look::Pending => return Shown::Waiting,
        Look::Unsupported | Look::Failed => return Shown::Unable,
    };
    let cols = figures.cols(key).unwrap_or(rect.width).min(rect.width);
    let Some(drawn) = figures.shown(key) else {
        return Shown::Waiting;
    };
    let own = Rect::new(
        rect.x + (rect.width - cols) / 2,
        rect.y + (rect.height - rows) / 2,
        cols,
        rows,
    );
    // 整张露在窗口里才画（侧边栏、首页都不会把它切掉一截）。只画露出来的上面几行：区域比图矮的，下面的不画。
    if frame.area().contains(own.as_position()) && own.bottom() <= frame.area().bottom() {
        let top = Rect {
            height: revealed(own.height, shown),
            ..own
        };
        SlicedImage::new(&drawn.protocol, SignedPosition { x: 0, y: 0 })
            .render(top, frame.buffer_mut());
    }
    Shown::Yes
}

#[cfg(test)]
mod tests {
    use super::{revealed, wanted};
    use crate::app::Avatar;

    #[test]
    fn the_switch_hides_the_avatar_too() {
        // 2026-10-29 项目主人：藏了吉祥物却依旧显示头像不对，关了形象头像也不画。
        let file = || Avatar::File("/a.png".into());
        assert_eq!(wanted(false, file()), None, "关着的头像也不画");
        assert_eq!(wanted(false, Avatar::None), None);
        assert_eq!(wanted(true, file()), Some(file()), "开着的画头像");
        assert_eq!(
            wanted(true, Avatar::None),
            Some(Avatar::None),
            "没头像的画吉祥物"
        );
    }

    #[test]
    fn the_avatar_is_revealed_row_by_row_from_the_top() {
        // 2026-10-14 项目主人定：完成页的头像从上往下一行行露出来。
        assert_eq!(revealed(10, 0.0), 0, "还没开始：一行都不露");
        assert_eq!(revealed(10, 0.01), 1, "刚开始就露一行");
        assert_eq!(revealed(10, 0.5), 5);
        assert_eq!(revealed(10, 1.0), 10, "到头整张");
        assert_eq!(revealed(10, 2.0), 10, "多了的不超过整张");
    }
}
