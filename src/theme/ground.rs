//! 铺底色（蓝图 `tui.md`「主题」第 4 条，2026-10-13 项目主人定）：手动选了深色、浅色的，整个界面铺这一套的底色，没上色的
//! 字用这一套的字色（终端是深底又选了浅色也看得清）；跟随终端的、关了的不铺。画完一帧、换色深之前做。

use std::sync::RwLock;

use ratatui::buffer::Buffer;
use ratatui::style::Color;

/// kitty 的图占着的格子写这个字，前景色是图的编号（同 `depth.rs`）：不碰。
const KITTY_PLACEHOLDER: char = '\u{10EEEE}';

/// 铺的底色、字色：手动选了深浅才有。
static GROUND: RwLock<Option<(Color, Color)>> = RwLock::new(None);

/// 换要铺的底色、字色；`None` 是不铺。
pub fn set_ground(ground: Option<(Color, Color)>) {
    match GROUND.write() {
        Ok(mut current) => *current = ground,
        Err(poisoned) => *poisoned.into_inner() = ground,
    }
}

/// 画完的一帧铺底色：没上底色的格子铺底色，没上色的字用字色。不铺的什么都不做；kitty 的图占着的格子不碰。
pub fn paint(buf: &mut Buffer) {
    let ground = match GROUND.read() {
        Ok(g) => *g,
        Err(poisoned) => *poisoned.into_inner(),
    };
    let Some((bg, fg)) = ground else {
        return;
    };
    for cell in &mut buf.content {
        if cell.symbol().starts_with(KITTY_PLACEHOLDER) {
            continue;
        }
        if cell.bg == Color::Reset {
            cell.bg = bg;
        }
        if cell.fg == Color::Reset {
            cell.fg = fg;
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use super::{paint, set_ground};

    #[test]
    fn a_painted_frame_fills_only_what_has_no_color_and_leaves_pictures_alone() {
        let _theme = crate::theme::hold();
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
        buf[(1, 0)].set_fg(Color::Red).set_bg(Color::Blue);
        buf[(2, 0)].set_symbol("\u{10EEEE}");
        set_ground(None);
        paint(&mut buf);
        assert_eq!(buf[(0, 0)].bg, Color::Reset, "不铺的不动");
        let (bg, fg) = (Color::Rgb(0xe1, 0xe2, 0xe7), Color::Rgb(0x37, 0x60, 0xbf));
        set_ground(Some((bg, fg)));
        paint(&mut buf);
        set_ground(None);
        assert_eq!((buf[(0, 0)].bg, buf[(0, 0)].fg), (bg, fg), "没上色的铺上");
        assert_eq!(
            (buf[(1, 0)].bg, buf[(1, 0)].fg),
            (Color::Blue, Color::Red),
            "上了色的不动"
        );
        assert_eq!(buf[(2, 0)].bg, Color::Reset, "图占着的格子不碰");
    }
}
