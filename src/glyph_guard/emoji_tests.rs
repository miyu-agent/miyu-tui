//! 宽度认不准的字、带 VS16 的 emoji 混在随机的帧里，活跃和放掉以后醒来两条路，四种模拟终端上都和界面排的一样（模拟终端
//! 见 `tests.rs` 的 `Model`）。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::tests::{H, TERMINALS, W, shown_with};
use crate::screen::Screen;
use crate::screen::wire::{Wire, frame as screen_frame};

/// 随机的几帧：每帧三行，拼的字里有中文、emoji，这种字后面照规矩跟一个空格。
fn random_frames(seed: u64, count: usize) -> Vec<Vec<(u16, String)>> {
    const PIECES: [&str; 11] = [
        "a",
        "b",
        " ",
        "中",
        "😀",
        "\u{f0768} ",
        "-",
        "é",
        "  ",
        "❤️",
        "⚠️",
    ];
    let mut rng = crate::rng::Rng::new(seed);
    let mut rows = vec![String::new(); 3];
    (0..count)
        .map(|_| {
            for row in &mut rows {
                if rng.between([0, 9]) < 5 {
                    // 最多十块、二十格，这种字不落在最后一格（当两格的终端里放不下）。
                    *row = (0..rng.between([0, 10]))
                        .map(|_| PIECES[rng.between([0, 10]) as usize])
                        .collect();
                }
            }
            (1u16..).zip(rows.iter().cloned()).collect()
        })
        .collect()
}

#[test]
fn random_frames_with_and_without_parks_match_on_every_kind_of_terminal() {
    let frames = random_frames(20_261_123, 24);
    let parks = [3, 7, 8, 12, 19];
    for kind in TERMINALS {
        for parks in [&[][..], &parks[..]] {
            for k in 1..=frames.len() {
                let (screen, want) = shown_with(true, parks, kind, &frames[..k]);
                assert_eq!(screen, want, "第 {k} 帧，{kind:?}，放掉 {parks:?}");
            }
        }
    }
}

#[test]
fn active_frames_write_at_most_a_few_cells_more_than_ratatui() {
    // 活跃的帧：写的格子不超过 ratatui 逐格比出来的，加上每一截两头往外扩的（宽字、这种字和它右边那一格，每头最多两格）。
    let frames = random_frames(20_261_124, 40);
    let options = TerminalOptions {
        viewport: Viewport::Fixed(Rect::new(0, 0, W, H)),
    };
    let mut reference = Terminal::with_options(Wire::new(W, H), options).unwrap();
    let mut screen = Screen::new(Wire::new(W, H));
    for (n, lines) in frames.iter().enumerate() {
        let lay = |buf: &mut Buffer| {
            for (y, text) in lines {
                Line::raw(text.clone()).render(Rect::new(0, *y, W, 1), buf);
            }
        };
        reference.draw(|f| lay(f.buffer_mut())).unwrap();
        screen_frame(&mut screen, |buf| {
            lay(buf);
            None
        });
        let diff = std::mem::take(&mut reference.backend_mut().written);
        let ours = std::mem::take(&mut screen.backend_mut().written);
        if n == 0 {
            continue;
        }
        let mut sorted = diff.clone();
        sorted.sort_by_key(|(x, y)| (*y, *x));
        let runs = sorted
            .windows(2)
            .filter(|w| !(w[0].1 == w[1].1 && w[1].0 == w[0].0 + 1))
            .count()
            + usize::from(!sorted.is_empty());
        assert!(
            ours.len() <= diff.len() + 4 * runs,
            "第 {n} 帧：写了 {} 格，ratatui 比出 {} 格、{runs} 截",
            ours.len(),
            diff.len()
        );
    }
}

#[test]
fn an_emoji_with_vs16_does_not_shift_what_follows_it() {
    // 2026-11-23 在 kitty 里看到的：闲着放掉缓冲以后翻页，醒来那一帧补写 ❤️ 盖住的那一格时没挪光标，kitty 把 ❤️ 画成
    // 两格，那个空格落到后一格上，后面的字整截往右错一格，「和」「还」被擦掉。活跃时上一帧那一格有字的，ratatui 补写同样错位。
    let line = "English、❤️ 和 ⚠️ 还有";
    let before = "English, love and warn!";
    // 四种组合都比完再报，看得出哪条路、哪种终端错。
    let mut wrong = Vec::new();
    for kind in [(1, false, 2), (1, false, 1)] {
        for parks in [vec![], vec![1]] {
            let frames = [vec![(2, before.to_string())], vec![(2, line.to_string())]];
            let (screen, want) = shown_with(true, &parks, kind, &frames);
            if screen != want {
                wrong.push(format!("{kind:?}，放掉 {parks:?}：「{}」", screen[2]));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn an_emoji_with_vs16_at_the_end_of_a_row_is_not_followed_by_a_stray_cell() {
    // 盖住的那一格就是行尾：照 ratatui 的先后（emoji 以后不挪光标补写那一格），kitty 上那一格折到下一行开头；不补写，照一格画
    // 的终端上留着旧字。先写那一格、再写 emoji，两种都对。
    let mut line: String = std::iter::repeat_n('a', usize::from(W) - 2).collect();
    line.push_str("❤️");
    let filler: String = std::iter::repeat_n('b', usize::from(W)).collect();
    let mut wrong = Vec::new();
    for kind in [(1, false, 2), (1, false, 1)] {
        for parks in [vec![], vec![1]] {
            let frames = [
                vec![(2, filler.clone()), (3, filler.clone())],
                vec![(2, line.clone()), (3, filler.clone())],
            ];
            let (screen, want) = shown_with(true, &parks, kind, &frames);
            if screen != want {
                wrong.push(format!("{kind:?}，放掉 {parks:?}：{:?}", &screen[2..4]));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
