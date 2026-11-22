//! 画面那一层：写出去的字节喂给一个终端模型（vt100），屏上每一格的字和 ratatui 画好的缓冲对得上；没变的段一个字节都不发。

use ratatui::buffer::{CellDiffOption, CellWidth};
use ratatui::layout::{Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::Widget;

use super::SEGMENT;
use super::rig::{ATOM, HIDE, Rig, SHOW, has, intended, moves};

const W: u16 = 40;
const H: u16 = 6;

#[test]
fn the_same_frame_twice_sends_no_cell_at_all() {
    let mut rig = Rig::new(W, H);
    let lines = [(0, "hello"), (2, "中文 and more")];
    rig.lines(&lines, None);
    let (buf, bytes) = rig.lines(&lines, None);
    assert_eq!(bytes, HIDE, "只该有藏光标那一句");
    rig.matches(&buf);
}

#[test]
fn a_few_changed_characters_are_all_that_is_written() {
    let mut rig = Rig::new(W, H);
    // 第 1 行：头一段写着「head」，第二段的第一格是会变的计数（放掉以后醒来那一帧照段写，见 segment_tests）。
    let row = |n: u32| format!("{:w$}{n} of 9", "head", w = SEGMENT);
    rig.lines(&[(0, "first line"), (1, &row(1)), (3, "last")], None);
    let (buf, bytes) = rig.lines(&[(0, "first line"), (1, &row(2)), (3, "last")], None);
    assert_eq!(
        moves(&bytes),
        vec![(SEGMENT as u16, 1)],
        "活跃的帧只写变了的那一格（第 1 行第二段的第一格）"
    );
    assert!(!has(&bytes, b"head") && !has(&bytes, b"first") && !has(&bytes, b"last"));
    rig.matches(&buf);
}

#[test]
fn a_shorter_row_covers_the_old_characters() {
    let mut rig = Rig::new(W, H);
    rig.lines(&[(2, "a rather long line!")], None);
    let (buf, _) = rig.lines(&[(2, "short")], None);
    assert_eq!(
        rig.shown(2),
        format!("short{}", " ".repeat(usize::from(W) - 5))
    );
    rig.matches(&buf);
}

#[test]
fn wide_characters_keep_their_right_half_and_trade_places_with_narrow_ones() {
    let mut rig = Rig::new(W, H);
    let rows = [
        "中文abc😀x",
        "ab中文abc😀",
        "中文中文中文中文中文",
        "narrow only here!!",
        "a😀b中c文d",
        "中文abc😀x",
    ];
    for text in rows {
        let (buf, _) = rig.lines(&[(1, text), (4, "固定的一行 fixed")], None);
        rig.matches(&buf);
        let screen = rig.model.screen();
        for x in 0..W {
            let wide = buf[(x, 1)].cell_width() > 1;
            assert_eq!(
                screen.cell(1, x).unwrap().is_wide(),
                wide,
                "「{text}」第 {x} 格"
            );
        }
    }
}

#[test]
fn skipped_cells_are_never_written_even_when_their_neighbours_change() {
    let mut rig = Rig::new(W, H);
    rig.lines(&[(1, "abcdefgh")], None);
    // 第二帧：第 5 格是图占着的（写着别的字也不写），它左右两格都变了。
    let (buf, bytes) = rig.paint(
        |buf| {
            Line::raw("abcdXZYh").render(Rect::new(0, 1, W, 1), buf);
            buf[(5, 1)].set_diff_option(CellDiffOption::Skip);
        },
        None,
    );
    assert!(!has(&bytes, b"Z"), "图占着的那格不写");
    assert_eq!(&rig.shown(1)[..8], "abcdXfYh", "那一格留着终端上原来的");
    // 别的格子照缓冲。
    let mut want = intended(&buf, 1);
    want.replace_range(5..6, "f");
    assert_eq!(rig.shown(1), want);
    // 旧的 `skip` 字段同样不写。
    let (_, bytes) = rig.paint(
        |buf| {
            Line::raw("abcdQZYh").render(Rect::new(0, 1, W, 1), buf);
            #[allow(deprecated)]
            {
                buf[(5, 1)].skip = true;
            }
        },
        None,
    );
    assert!(!has(&bytes, b"Z"), "旧的 skip 也不写");
}

#[test]
fn shaky_glyphs_go_last_each_after_an_explicit_move() {
    let mut rig = Rig::new(W, H);
    let text = format!("ab{ATOM}cd {ATOM}e");
    let (buf, bytes) = rig.lines(&[(2, &text)], None);
    rig.matches(&buf);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let atoms: Vec<usize> = text.match_indices(ATOM).map(|(i, _)| i).collect();
    assert_eq!(atoms.len(), 2);
    let last_plain = text.rfind('e').unwrap();
    assert!(
        atoms.iter().all(|&i| i > last_plain),
        "普通的格子（含右边那一格）先写完：{text:?}"
    );
    // 每个字前面紧挨着挪到它自己那一格。
    for (&i, x) in atoms.iter().zip([2u16, 6]) {
        let before = &text[..i];
        let at = before.rfind("\x1b[").unwrap();
        assert_eq!(moves(&before.as_bytes()[at..]), vec![(x, 2)], "{text:?}");
    }
}

#[test]
fn after_a_resize_or_a_clear_the_frame_writes_what_is_not_blank() {
    // 换了大小、清了屏，终端是空的：同 ratatui，下一帧和空的上一份比，只写有字的行（空着的行不写）。
    let mut rig = Rig::new(W, H);
    let lines = [(0, "top 顶上"), (3, "middle"), (5, "bottom")];
    rig.lines(&lines, None);
    for (width, height) in [(30, 10), (12, 4), (W, H)] {
        rig.resize(width, height);
        let shown = &lines[..usize::from(height.min(3))];
        let (buf, bytes) = rig.lines(shown, None);
        assert!(has(&bytes, b"\x1b[2J"), "换了大小先清屏");
        // 宽字后面那一格不写，crossterm 的后端接着写下一格前会再挪一次光标：同一行可能挪好几次，照行去重。
        let mut rows: Vec<u16> = moves(&bytes).into_iter().map(|(_, y)| y).collect();
        rows.dedup();
        let want: Vec<u16> = shown
            .iter()
            .map(|(y, _)| *y)
            .filter(|y| *y < height)
            .collect();
        assert_eq!(rows, want, "只写有字的行");
        rig.matches(&buf);
    }
    rig.screen.clear().unwrap();
    rig.model.process(&rig.screen.backend_mut().take());
    let (buf, bytes) = rig.lines(&lines, None);
    let mut rows: Vec<u16> = moves(&bytes).into_iter().map(|(_, y)| y).collect();
    rows.dedup();
    assert_eq!(rows, vec![0, 3, 5], "清过屏只写有字的行");
    rig.matches(&buf);
}

#[test]
fn the_cursor_shows_where_asked_and_hides_otherwise() {
    let mut rig = Rig::new(W, H);
    let (_, bytes) = rig.lines(&[(0, "x")], Some(Position::new(3, 2)));
    assert!(!has(&bytes, HIDE));
    let show = bytes.windows(SHOW.len()).position(|w| w == SHOW).unwrap();
    assert_eq!(moves(&bytes[show..]), vec![(3, 2)], "先显示再挪过去");
    assert!(!rig.model.screen().hide_cursor());
    assert_eq!(rig.model.screen().cursor_position(), (2, 3));
    // 连着两帧显示：不藏。
    let (_, bytes) = rig.lines(&[(0, "y")], Some(Position::new(4, 2)));
    assert!(!has(&bytes, HIDE), "显示着的不先藏");
    assert_eq!(rig.model.screen().cursor_position(), (2, 4));
    let (_, bytes) = rig.lines(&[(0, "y")], None);
    assert_eq!(bytes, HIDE);
    assert!(rig.model.screen().hide_cursor());
}
