//! 没放掉过缓冲的帧（活跃的、终端是空的）和 ratatui 原来的 `Terminal` 写一样的字节；放掉以后清屏、换大小、带 VS16 的
//! emoji 都对。

use ratatui::Terminal;

use super::Screen;
use super::rig::{HIDE, Rig, Scenes, moves};
use super::wire::{Wire, frame};

const W: u16 = 48;
const H: u16 = 6;
/// crossterm 的后端每交来一批格子，末尾都还原颜色和样式；ratatui 每帧都交一批（没变的也交一批空的）。
const RESET: &[u8] = b"\x1b[39m\x1b[49m\x1b[59m\x1b[0m";

/// ratatui 这一帧写的去掉「什么都没变也交一批空的」那一串还原（这里不交）。
fn without_empty_batch(theirs: Vec<u8>) -> Vec<u8> {
    match theirs.strip_prefix(RESET) {
        Some(rest) if rest == HIDE => rest.to_vec(),
        _ => theirs,
    }
}

#[test]
fn frames_write_exactly_what_ratatui_writes_from_the_first_one() {
    // 不放宽度认不准的字：那种字原来另有一层保护（`glyph_guard`），不是 ratatui 自己的写法。
    const PLAIN: &[&str] = &["a", "b", "x", " ", " ", "中", "文", "😀", "é", "--", "ok"];
    let (same, differed) = against_ratatui(PLAIN);
    assert_eq!(
        (same, differed),
        (80, 0),
        "没有带 VS16 的 emoji：每一帧逐字节一样"
    );
    // 带 VS16 的 emoji 放进来：ratatui 补写它盖住的那一格的帧逐格比，别的帧逐字节比。
    const EMOJI: &[&str] = &[
        "a", "b", "x", " ", " ", "中", "文", "😀", "❤️", "⚠️", "é", "--", "ok",
    ];
    let (same, differed) = against_ratatui(EMOJI);
    assert!(
        differed > 0 && same + differed == 80,
        "逐字节比了 {same} 帧，补写的 {differed} 帧"
    );
}

/// 用 `pieces` 拼随机的八十帧，一边给这一层画，一边给 ratatui 原来的 `Terminal` 画，逐帧比；交回逐字节一样的帧数和
/// ratatui 补写了带 VS16 的 emoji 盖住的格子、改成逐格比的帧数。
fn against_ratatui(pieces: &'static [&'static str]) -> (usize, usize) {
    let mut scenes = Scenes::new(20_261_123, pieces, W, H);
    let mut screen = Screen::new(Wire::new(W, H));
    // 原来的样子：全屏视口的 ratatui，自己照后端报的大小换大小、清屏。
    let mut reference = Terminal::new(Wire::new(W, H)).unwrap();
    // 第 20 帧以前换小、第 40 帧以前换大、第 60 帧以前清屏。
    let mut sizes = 0;
    let (mut same, mut differed) = (0, 0);
    for n in 0..80 {
        let size = match n {
            20 => Some((W - 6, H - 1)),
            40 => Some((W, H + 2)),
            _ => None,
        };
        if let Some((width, height)) = size {
            screen.backend_mut().size = ratatui::layout::Size::new(width, height);
            reference.backend_mut().size = ratatui::layout::Size::new(width, height);
            sizes += 1;
        }
        if n == 60 {
            screen.clear().unwrap();
            // ratatui 的 `clear` 先问光标在哪（测试的终端不回话），这里直接清它的后端、作废它的上一份：和它的
            // `clear_viewport` 一样。清屏的那几个字节不算这一帧的，两边都扔掉。
            reference.resize(reference.size().unwrap().into()).unwrap();
            screen.backend_mut().take();
            reference.backend_mut().take();
        }
        let scene = scenes.next();
        let (buf, ours) = frame(&mut screen, |buf| {
            scene.paint(buf);
            None
        });
        reference.draw(|f| scene.paint(f.buffer_mut())).unwrap();
        let theirs = without_empty_batch(reference.backend_mut().take());
        let ours_cells = std::mem::take(&mut screen.backend_mut().written);
        let their_cells = std::mem::take(&mut reference.backend_mut().written);
        // ratatui 补写了带 VS16 的 emoji 盖住的那一格：这里故意不一样（`glyph_guard::shown`）。写的格子一样，先后和挪光标
        // 不一样（先写盖住的那一格、再写 emoji，各自挪过光标），所以这种帧逐格比写了哪些格子；别的帧逐字节比。
        let covered_by_emoji =
            |&(x, y): &(u16, u16)| x > 0 && crate::glyph_guard::emoji(&buf[(x - 1, y)]);
        if their_cells.iter().any(covered_by_emoji) {
            let mut want = their_cells.clone();
            let mut got = ours_cells.clone();
            got.sort_by_key(|(x, y)| (*y, *x));
            got.dedup();
            want.sort_by_key(|(x, y)| (*y, *x));
            want.dedup();
            assert_eq!(
                got, want,
                "第 {n} 帧（补写了 VS16 后面那一格）；各行：{:?}",
                scene.rows
            );
            differed += 1;
            continue;
        }
        assert_eq!(
            String::from_utf8_lossy(&ours),
            String::from_utf8_lossy(&theirs),
            "第 {n} 帧；各行：{:?}",
            scene.rows
        );
        same += 1;
    }
    assert_eq!(sizes, 2);
    (same, differed)
}

#[test]
fn an_emoji_with_vs16_clears_the_cell_it_covers_on_either_path() {
    // 模型（vt100）照一格画带 VS16 的 emoji：它盖住的那一格原来有字的，得写成空的（ratatui 多写的那一格）。
    for park in [false, true] {
        let mut rig = Rig::new(W, H);
        let (buf, _) = rig.lines(&[(1, "aXbc"), (2, "ok ❤️ fine")], None);
        rig.matches(&buf);
        if park {
            rig.screen.park();
        }
        let (buf, _) = rig.lines(&[(1, "❤️bc"), (2, "ok ⚠️ fine")], None);
        rig.matches(&buf);
        if park {
            rig.screen.park();
        }
        let (buf, _) = rig.lines(&[(1, "aXbc"), (2, "ok ❤️ fine")], None);
        rig.matches(&buf);
    }
}

#[test]
fn after_parking_a_clear_or_a_resize_writes_like_a_fresh_screen() {
    // 放掉以后清屏、换大小：终端是空的，和刚起来一样只写有字的行；不走指纹。
    let mut rig = Rig::new(W, H);
    let lines = [(0, "top 顶上"), (3, "middle"), (5, "bottom")];
    rig.lines(&lines, None);
    rig.screen.park();
    rig.screen.clear().unwrap();
    rig.model.process(&rig.screen.backend_mut().take());
    let (buf, bytes) = rig.lines(&lines, None);
    let mut rows: Vec<u16> = moves(&bytes).into_iter().map(|(_, y)| y).collect();
    rows.dedup();
    assert_eq!(rows, vec![0, 3, 5], "放掉以后清屏：只写有字的行");
    rig.matches(&buf);
    rig.screen.park();
    rig.resize(W + 4, H + 1);
    let (buf, bytes) = rig.lines(&lines, None);
    let mut rows: Vec<u16> = moves(&bytes).into_iter().map(|(_, y)| y).collect();
    rows.dedup();
    assert_eq!(rows, vec![0, 3, 5], "放掉以后换大小：只写有字的行");
    rig.matches(&buf);
    // 再画一帧一样的：活跃了，什么都不写。
    let (_, bytes) = rig.lines(&lines, None);
    assert_eq!(bytes, HIDE);
}

#[test]
fn parking_twice_is_harmless_and_waking_with_nothing_changed_writes_nothing() {
    let mut rig = Rig::new(W, H);
    let lines = [(1, "hello 你好"), (4, "😀 ok")];
    rig.lines(&lines, None);
    rig.screen.park();
    rig.screen.park();
    assert!(!rig.screen.active());
    let (buf, bytes) = rig.lines(&lines, None);
    assert_eq!(bytes, HIDE, "醒来那一帧和放掉时一样：只藏光标");
    rig.matches(&buf);
}
