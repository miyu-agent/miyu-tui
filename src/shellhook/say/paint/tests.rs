use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::Painter;

fn painter() -> Painter<Vec<u8>> {
    Painter::new(Vec::new(), crate::theme::Depth::True)
}

fn plain(text: &str) -> String {
    // 去掉颜色的控制序列，只看字。
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn wide_characters_are_written_once_and_trailing_blanks_are_dropped() {
    let lines = painter()
        .lines(20, &[Line::from("你好 ok"), Line::default()])
        .unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        plain(&lines[0]),
        "你好 ok",
        "宽字后面那格不写，行尾的空格不写"
    );
    assert_eq!(lines[1], "", "空行什么都不写");
}

#[test]
fn colours_are_kept_per_span() {
    let line = Line::from(vec![
        Span::styled("红", Style::default().fg(Color::Rgb(255, 0, 0))),
        Span::raw("字"),
    ]);
    let lines = painter().lines(10, &[line]).unwrap();
    assert!(
        lines[0].contains("38;2;255;0;0"),
        "真彩色照原样：{:?}",
        lines[0]
    );
    assert!(lines[0].ends_with("\u{1b}[0m"), "行尾复位");
}

#[test]
fn a_frame_replaces_the_live_area_and_settled_lines_stay() {
    let mut painter = painter();
    painter
        .frame(&[], &["a".into(), "b".into(), "c".into()])
        .unwrap();
    painter.frame(&["done".into()], &["x".into()]).unwrap();
    let out = String::from_utf8(std::mem::take(&mut painter.out)).unwrap();
    // 第二帧：回到活动区第一行（往上两行）、往下全清，定下的一行接着换行，再画新的活动区。
    let second = &out[out.find("c").unwrap()..];
    assert!(second.contains("\u{1b}[2A"), "往上回两行：{second:?}");
    assert!(second.contains("\u{1b}[J"), "往下全清：{second:?}");
    assert!(second.contains("done\r\nx"), "{second:?}");
    assert!(
        out.contains("\u{1b}[?2026h") && out.contains("\u{1b}[?2026l"),
        "包在同步输出里"
    );
}
