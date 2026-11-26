//! 宽字后面那一格清成空白（蓝图 `tui.md`「每一帧」）：画完、交给终端之前过一遍。
//!
//! ratatui 比对两帧时认定宽字后面那格是空的；`Paragraph` 画宽字却不清后面那格，列表、面板盖在别的东西上时，
//! 缓冲区里就成了「宽字 + 横线」。等宽字被盖掉，比对只重写前半格，终端上留下半格空白或残色。
//! 清成的空白带着宽字的样子（底色）：终端上那格是宽字的后半、是宽字的颜色，记成默认的，下一帧那里也是默认的
//! 空白时比对不重写，留下一格残色（2026-10-01 命令列表选中行的蓝底）。

use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

/// 窗、框、提示盖在 `rect` 上之前先清地方：里面的格子清空（同 ratatui 的 `Clear`）；左边一格写着宽字、后半跨进来的，那个字
/// 换成空白（带着它原来的样子）。不换的话比对时宽字后面那一格不写，终端上窗的第一列露着半个字；[`tidy`] 还会把那一格清成
/// 宽字的空白，窗的底色也没了（2026-11-20 主人本机验收第 5 条：软件包页回车弹出的框被左边列表的中文字顶穿）。
pub fn cover(buf: &mut Buffer, rect: Rect) {
    let rect = rect.intersection(buf.area);
    for y in rect.top()..rect.bottom() {
        for x in rect.left()..rect.right() {
            buf[(x, y)].reset();
        }
        if rect.x > buf.area.x {
            let left = &mut buf[(rect.x - 1, y)];
            if left.symbol().width() > 1 && left.diff_option != CellDiffOption::Skip {
                left.set_symbol(" ");
            }
        }
    }
}

/// 一格的字占几列，照 unicode-width 对整个字串算的那样：一个字节的字（ASCII，空格、控制字符在内）它一定算 1，不查表
/// （每帧整屏每一格都要问一次，大多是 ASCII）；别的照查。
fn symbol_width(symbol: &str) -> usize {
    if symbol.len() == 1 { 1 } else { symbol.width() }
}

/// 每个宽字后面那几格不是空白的，照 ratatui 自己写宽字时的做法清掉。图占着的格子不碰。
pub fn tidy(buf: &mut Buffer) {
    let area = buf.area;
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let width = u16::try_from(symbol_width(buf[(x, y)].symbol())).unwrap_or(1);
            let style = buf[(x, y)].style();
            for k in 1..width {
                let Some(next) = buf.cell_mut((x + k, y)) else {
                    break;
                };
                if next.diff_option != CellDiffOption::Skip {
                    // 终端上这格是宽字的后半，带着宽字的颜色：照它记，盖掉宽字时比对才会重写这格。
                    next.reset();
                    next.set_style(style);
                }
            }
            x += width.max(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::text::Line;
    use ratatui::widgets::{Paragraph, Widget};

    use super::{cover, symbol_width, tidy};

    #[test]
    fn covering_a_cell_blanks_the_wide_character_that_reaches_into_it() {
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::empty(area);
        let red = Style::new().fg(Color::Red);
        buf.set_string(0, 0, "游戏破", red);
        // 「戏」在第 2、3 格：从第 3 格起盖，它跨进来。
        cover(&mut buf, Rect::new(3, 0, 3, 1));
        tidy(&mut buf);
        let row: Vec<&str> = (0..6).map(|x| buf[(x, 0)].symbol()).collect();
        assert_eq!(row, ["游", " ", " ", " ", " ", " "]);
        assert_eq!(buf[(2, 0)].fg, Color::Red, "换成的空白带着原来的样子");
        assert_eq!(buf[(3, 0)].fg, Color::Reset, "盖住的格子是空的");
    }

    #[test]
    fn a_wide_character_drawn_over_a_line_owns_the_next_cell() {
        let area = Rect::new(0, 0, 6, 1);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "──────", Style::new().fg(Color::Blue));
        // Paragraph 画宽字不清后面那格：盖在横线上以后，「令」后面还是横线。
        Paragraph::new(Line::raw("命令")).render(Rect::new(1, 0, 4, 1), &mut buf);
        assert_eq!(
            buf[(2, 0)].symbol(),
            "─",
            "先证明 Paragraph 会留下这种缓冲区"
        );
        tidy(&mut buf);
        let row: Vec<&str> = (0..6).map(|x| buf[(x, 0)].symbol()).collect();
        assert_eq!(row, ["─", "命", " ", "令", " ", "─"]);
        // 照这样比对，盖回横线时两格都重写。
        let mut next = Buffer::empty(area);
        next.set_string(0, 0, "──────", Style::new().fg(Color::Blue));
        let written: Vec<u16> = buf.diff(&next).iter().map(|(x, _, _)| *x).collect();
        assert_eq!(written, [1, 2, 3, 4]);
    }

    #[test]
    fn the_cell_after_a_wide_character_keeps_its_background() {
        // 2026-10-01 项目主人报：`/restore` 回车以后通知框里留下一格蓝块，通知消失了还在。命令列表选中那一行
        // 是蓝底，宽字在终端上占两格、两格都是蓝底；后面那格在缓冲区里却记成默认的空白。列表收起、那里换成
        // 窄字和默认的空白，比对认为后面那格没变不重写，终端上就留着蓝的半格。
        let area = Rect::new(0, 0, 4, 1);
        let blue = Style::new().bg(Color::Blue);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "恢 ", blue);
        tidy(&mut buf);
        assert_eq!(buf[(1, 0)].bg, Color::Blue, "后面那格照宽字的底色记");
        let mut next = Buffer::empty(area);
        next.set_string(0, 0, "x", Style::new());
        let written: Vec<u16> = buf.diff(&next).iter().map(|(x, _, _)| *x).collect();
        assert!(
            written.contains(&1),
            "盖掉宽字时后面那格也重写：{written:?}"
        );
    }

    #[test]
    fn the_one_byte_shortcut_gives_what_unicode_width_gives() {
        // BO-T2：一个字节的字不查表。每一个一字节的字（0x00 到 0x7F，控制字符在内）和查表的结果一样。
        use unicode_width::UnicodeWidthStr;
        for byte in 0u8..=0x7f {
            let symbol = char::from(byte).to_string();
            assert_eq!(symbol_width(&symbol), symbol.width(), "{byte:#04x}");
        }
    }

    #[test]
    fn tidying_a_mixed_screen_matches_the_table_lookup_cell_by_cell() {
        // 一屏混着中文、emoji、ASCII、控制字符、组合字符、空格子、图占着的格子：走快路的 tidy 和每格都查表的原来那份
        // 逐格相同（字、样子、比对的选项）。
        use ratatui::buffer::{Cell, CellDiffOption};
        use unicode_width::UnicodeWidthStr;
        fn old_tidy(buf: &mut Buffer) {
            let area = buf.area;
            for y in area.top()..area.bottom() {
                let mut x = area.left();
                while x < area.right() {
                    let width = u16::try_from(buf[(x, y)].symbol().width()).unwrap_or(1);
                    let style = buf[(x, y)].style();
                    for k in 1..width {
                        let Some(next) = buf.cell_mut((x + k, y)) else {
                            break;
                        };
                        if next.diff_option != CellDiffOption::Skip {
                            next.reset();
                            next.set_style(style);
                        }
                    }
                    x += width.max(1);
                }
            }
        }
        let area = Rect::new(0, 0, 23, 6);
        let mut buf = Buffer::empty(area);
        let symbols = [
            "a",
            "中",
            "x",
            "👩‍💻",
            "\u{7}",
            " ",
            "🇨🇳",
            "e\u{301}",
            "\t",
            "\u{1b}",
            "❤️",
            "─",
            "",
            "ｱ",
            "ﾞ",
            "\r",
            "\n",
            "\u{7f}",
            "国",
            "~",
            "⚠️",
            "👍🏽",
            "Z",
        ];
        let colors = [Color::Red, Color::Blue, Color::Reset, Color::Rgb(1, 2, 3)];
        for y in 0..area.height {
            for x in 0..area.width {
                let k = usize::from(x * 7 + y * 3);
                let cell: &mut Cell = &mut buf[(x, y)];
                cell.set_symbol(symbols[k % symbols.len()])
                    .set_style(Style::new().fg(colors[k % 4]).bg(colors[(k + 1) % 4]));
                if k % 11 == 0 {
                    cell.set_diff_option(CellDiffOption::Skip);
                }
            }
        }
        let mut old = buf.clone();
        old_tidy(&mut old);
        tidy(&mut buf);
        for y in 0..area.height {
            for x in 0..area.width {
                assert_eq!(buf[(x, y)], old[(x, y)], "第 {y} 行第 {x} 格");
            }
        }
    }

    #[test]
    fn cells_held_by_a_picture_are_left_alone() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
        buf.set_string(0, 0, "图", Style::new());
        buf[(1, 0)]
            .set_symbol("x")
            .set_diff_option(ratatui::buffer::CellDiffOption::Skip);
        tidy(&mut buf);
        assert_eq!(buf[(1, 0)].symbol(), "x");
    }
}
