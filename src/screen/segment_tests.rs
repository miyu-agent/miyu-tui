//! 画面的两条路（活跃时逐格比、放掉以后醒来那一帧照段比）：宽字跨段、隔开的两段、重格子（图的整份数据塞在一格里），再加
//! 一段随机的帧、随机放掉，模型和缓冲逐格对得上。带 `park` 的测试两条路都走：`park` 为真时每一帧以前先放掉。

use image::{DynamicImage, Rgb, RgbImage};
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::{Rect, Size};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use ratatui_image::Resize;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::sliced::{SignedPosition, SlicedImage, SlicedProtocol};

use super::rig::{ATOM, Rig, Scenes, has, moves};
use super::{HEAVY, SEGMENT};

const W: u16 = 48;
const H: u16 = 6;

/// 第一段末尾一格起是一个中文（前半在第一段的最后一格，后半在第二段的第一格），两边各是一串字母。
fn across(left: char, right: char) -> String {
    let head: String = std::iter::repeat_n(left, SEGMENT - 1).collect();
    let tail: String = std::iter::repeat_n(right, 20).collect();
    format!("{head}中{tail}")
}

impl Rig {
    /// `park` 为真时先放掉缓冲（下一帧照段比），再画几行字。
    fn after(&mut self, park: bool, lines: &[(u16, &str)]) -> (Buffer, Vec<u8>) {
        if park {
            self.screen.park();
        }
        self.lines(lines, None)
    }
}

#[test]
fn a_wide_character_across_a_segment_boundary_survives_either_side_changing() {
    for park in [false, true] {
        wide_across(park);
    }
}

fn wide_across(park: bool) {
    let mut rig = Rig::new(W, H);
    rig.lines(&[(1, &across('a', 'b'))], None);
    // 只改前一段（宽字的前半在这一段的最后一格）。
    let (buf, bytes) = rig.after(park, &[(1, &across('c', 'b'))]);
    rig.matches(&buf);
    // 醒来那一帧照段写：截的最后一格是宽字的前半，宽字照写。活跃时只写变了的格子，宽字没变不写。
    assert_eq!(has(&bytes, "中".as_bytes()), park, "park {park}");
    // 只改后一段（宽字的后半在这一段的第一格）：截往左扩到宽字，不然后半写成空格会把宽字劈掉。
    let (buf, bytes) = rig.after(park, &[(1, &across('c', 'd'))]);
    rig.matches(&buf);
    // 醒来那一帧从宽字写起（截往左扩）；活跃时宽字没变，从它后面第一个变了的格子写起。
    let first = if park { SEGMENT - 1 } else { SEGMENT + 1 };
    assert_eq!(
        moves(&bytes)[0],
        (first as u16, 1),
        "从宽字写起（park {park}）"
    );
    // 两段都改。
    let (buf, _) = rig.after(park, &[(1, &across('e', 'f'))]);
    rig.matches(&buf);
    // 宽字换成窄字、再换回来。
    let narrow = across('e', 'f').replace('中', "xy");
    let (buf, _) = rig.after(park, &[(1, &narrow)]);
    rig.matches(&buf);
    let (buf, _) = rig.after(park, &[(1, &across('e', 'f'))]);
    rig.matches(&buf);
}

#[test]
fn two_separated_changes_send_only_their_own_segments() {
    for park in [false, true] {
        let mut rig = Rig::new(W, H);
        two_separated(&mut rig, park);
    }
}

fn two_separated(rig: &mut Rig, park: bool) {
    // 第一段第 0 格会变、第二段写着 mi、第三段没变、第四段第一格会变。
    let row = |a: char, b: char| {
        format!(
            "{a}{:p1$}mi{:p2$}{b}",
            "",
            "",
            p1 = SEGMENT - 1,
            p2 = 2 * SEGMENT - 2
        )
    };
    rig.lines(&[(2, &row('1', '2'))], None);
    let (buf, bytes) = rig.after(park, &[(2, &row('3', '4'))]);
    assert_eq!(
        moves(&bytes),
        vec![(0, 2), (3 * SEGMENT as u16, 2)],
        "只写第一段和第四段（park {park}）"
    );
    assert!(!has(&bytes, b"mi"), "中间没变的段不发");
    rig.matches(&buf);
}

/// 重格子（图的左上角）放在第二段的最后一格：它左边两格和它同一段，图盖着的格子在下一段。
const ANCHOR: u16 = 2 * SEGMENT as u16 - 1;

/// 假的重格子：一个字母后面接一串组合符号（一格宽、字节很长），像 ratatui-image 那样强定一格宽，右边三格是图占着的。
fn heavy(buf: &mut Buffer, letter: char) {
    let symbol: String = std::iter::once(letter)
        .chain(std::iter::repeat_n('\u{301}', 40))
        .collect();
    assert!(symbol.len() > HEAVY);
    buf[(ANCHOR, 1)]
        .set_symbol(&symbol)
        .set_diff_option(CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN));
    for x in ANCHOR + 1..ANCHOR + 4 {
        buf[(x, 1)].set_diff_option(CellDiffOption::Skip);
    }
}

/// 这段字节里重格子（`letter` 起头的那一串）写了几回。
fn heavies(bytes: &[u8], letter: char) -> usize {
    let mark = format!("{letter}\u{301}\u{301}");
    String::from_utf8_lossy(bytes).matches(&mark).count()
}

#[test]
fn a_heavy_cell_is_sent_once_until_it_changes_or_the_screen_is_cleared() {
    for park in [false, true] {
        heavy_once(park);
    }
}

fn heavy_once(park: bool) {
    let mut rig = Rig::new(W, H);
    let mut sent = 0;
    for n in 0..6 {
        if park {
            rig.screen.park();
        }
        let (buf, bytes) = rig.paint(
            |buf| {
                // 和重格子在同一段：它左边两格是会变的字。
                Line::raw(format!("t{n}")).render(Rect::new(ANCHOR - 2, 1, 2, 1), buf);
                heavy(buf, 'H');
            },
            None,
        );
        sent += heavies(&bytes, 'H');
        rig.matches(&buf);
    }
    assert_eq!(
        sent, 1,
        "同一段里别的格子变了，重格子只在第一帧写（park {park}）"
    );
    if park {
        rig.screen.park();
    }
    let (_, bytes) = rig.paint(|buf| heavy(buf, 'G'), None);
    assert_eq!(heavies(&bytes, 'G'), 1, "内容变了重写");
    rig.screen.clear().unwrap();
    rig.model.process(&rig.screen.backend_mut().take());
    let (_, bytes) = rig.paint(|buf| heavy(buf, 'G'), None);
    assert_eq!(heavies(&bytes, 'G'), 1, "清过屏重写");
}

#[test]
fn an_active_frame_widening_into_a_heavy_cell_does_not_resend_it() {
    // 重格子左边一格是宽度认不准的字，每一帧在它和普通字之间换：截往右扩到右边那一格（重格子），重格子没变，不重写。
    let mut rig = Rig::new(W, H);
    let mut sent = 0;
    for n in 0..6 {
        let (buf, bytes) = rig.paint(
            |buf| {
                let left = if n % 2 == 0 { ATOM } else { "c" };
                Line::raw(left).render(Rect::new(ANCHOR - 1, 1, 1, 1), buf);
                heavy(buf, 'H');
            },
            None,
        );
        sent += heavies(&bytes, 'H');
        rig.matches(&buf);
    }
    assert_eq!(sent, 1, "重格子只在第一帧写");
}

/// 一张 4×2 格的小图，照 `kind` 的协议编好（同 `theme/depth/tests.rs` 造图的办法）。
fn picture(kind: ProtocolType) -> SlicedProtocol {
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(kind);
    let image = RgbImage::from_fn(40, 40, |x, y| Rgb([(x * 6) as u8, (y * 6) as u8, 128]));
    SlicedProtocol::new_with_resize(
        &picker,
        DynamicImage::ImageRgb8(image),
        Size::new(4, 2),
        Resize::Fit(None),
    )
    .unwrap()
}

#[test]
fn sixel_and_iterm2_pictures_are_sent_once_while_their_row_keeps_changing() {
    // sixel：整张图在左上角那一格（一份）；iTerm2：每行左边那一格放这一行（两份）。
    for (kind, mark, copies) in [
        (ProtocolType::Sixel, "\x1bP", 1),
        (ProtocolType::Iterm2, "\x1b]1337;File=", 2),
    ] {
        for park in [false, true] {
            let protocol = picture(kind);
            let mut rig = Rig::new(W, H);
            let mut sent = 0;
            for n in 0..5 {
                if park {
                    rig.screen.park();
                }
                let (_, bytes) = rig.paint(
                    |buf| {
                        // 图的左上角在第二段的最后一格，它左边那格（同一段）每一帧都变，图右边的也变。
                        SlicedImage::new(&protocol, SignedPosition { x: 0, y: 0 })
                            .render(Rect::new(ANCHOR, 1, 4, 2), buf);
                        for y in [1, 2] {
                            Line::raw(format!("{n}")).render(Rect::new(ANCHOR - 1, y, 1, 1), buf);
                            Line::raw(format!("frame {n}"))
                                .render(Rect::new(ANCHOR + 5, y, 10, 1), buf);
                        }
                    },
                    None,
                );
                sent += String::from_utf8_lossy(&bytes).matches(mark).count();
            }
            assert_eq!(sent, copies, "{kind:?}：图的数据只发一次（park {park}）");
        }
    }
}

#[test]
fn random_frames_with_random_parks_always_match_the_buffer() {
    const PIECES: &[&str] = &[
        "a", "b", "x", " ", " ", "中", "文", "😀", ATOM, "é", "--", "ok", "❤️", "⚠️",
    ];
    let mut scenes = Scenes::new(20_261_122, PIECES, W, H);
    let mut rig = Rig::new(W, H);
    let mut last: Option<Buffer> = None;
    let mut parks = 0;
    for frame in 0..60 {
        let scene = scenes.next();
        let park = frame > 0 && scenes.below(5) == 0;
        if park {
            rig.screen.park();
            parks += 1;
            assert!(!rig.screen.active(), "放掉了");
        }
        let (buf, bytes) = rig.paint(|buf| scene.paint(buf), None);
        assert!(rig.screen.active(), "画完留着缓冲");
        let check = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rig.matches(&buf)));
        assert!(check.is_ok(), "第 {frame} 帧对不上；各行：{:?}", scene.rows);
        if park && let Some(before) = &last {
            // 醒来那一帧只写变了的段（截往外扩最多带上左右各两格）。
            let changed = |x: usize, y: u16| {
                let k = x / SEGMENT;
                let range = k * SEGMENT..((k + 1) * SEGMENT).min(usize::from(W));
                range
                    .into_iter()
                    .any(|c| buf[(c as u16, y)] != before[(c as u16, y)])
            };
            for &(x, y) in &rig.screen.backend_mut().written {
                let x = usize::from(x);
                let near = [x.saturating_sub(2), x, (x + 2).min(usize::from(W) - 1)];
                assert!(
                    near.iter().any(|&c| changed(c, y)),
                    "第 {frame} 帧醒来写了没变的段：({x}, {y})"
                );
            }
        }
        rig.screen.backend_mut().written.clear();
        drop(bytes);
        last = Some(buf);
    }
    assert!((8..=16).contains(&parks), "随机放掉了 {parks} 次");
}
