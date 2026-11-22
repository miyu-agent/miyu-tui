//! 拿一个把第 15、16 平面私用区的字当两格的模拟终端，看思考那一行往上推以后屏上和界面排的对不对得上（2026-10-16
//! 项目主人手机上经 ssh 用时留下半个字形）。kitty 照一格画的那种也走一遍：两种终端都得对。

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;
use ratatui::{TerminalOptions, Viewport};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::shaky;
use crate::screen::wire::{Wire, frame as screen_frame};
use crate::screen::{SEGMENT, Screen};

pub(super) const W: u16 = 24;
pub(super) const H: u16 = 6;
/// 思考的图标（md-atom）。
pub(super) const ATOM: &str = "\u{f0768}";

/// 一个只认挪光标、打字的终端：`wide` 是它把这种字当几格。宽字盖掉一半的时候整个字清掉（常见终端的做法）。
/// `overflow`：当一格算、字形却画出两格宽（手机上那个终端）：右边那一格留着半个字形（残影），那一格重写了才没。
/// `emoji`：带 VS16 的 emoji 当几格：kitty 当两格（`U+FE0F` 来了把前一个字撑宽、光标再往右一格），有的终端当一格（不清
/// 后面那一格）。写到行尾以后再写字，折到下一行开头（同真的终端）。
struct Model {
    wide: usize,
    overflow: bool,
    emoji: usize,
    cells: Vec<Vec<String>>,
    ghosts: std::collections::HashSet<(usize, usize)>,
    at: (usize, usize),
}

/// 屏上残影那一格写成这个。
const GHOST: &str = "◐";

impl Model {
    fn new((wide, overflow, emoji): Kind) -> Self {
        Self {
            wide,
            overflow,
            emoji,
            cells: vec![vec![" ".to_string(); usize::from(W)]; usize::from(H)],
            ghosts: std::collections::HashSet::new(),
            at: (0, 0),
        }
    }

    fn width(&self, c: char) -> usize {
        if shaky(&c.to_string()) {
            self.wide
        } else {
            c.width().unwrap_or(1)
        }
    }

    /// 把第 `y` 行第 `x` 格从宽字里拆出来：它是宽字的哪一半，另一半清成空格。
    fn split(&mut self, x: usize, y: usize) {
        let row = &mut self.cells[y];
        if row[x].is_empty() && x > 0 {
            row[x - 1] = " ".into();
        }
        if !row[x].is_empty() && x + 1 < row.len() && row[x + 1].is_empty() {
            row[x + 1] = " ".into();
        }
    }

    fn print(&mut self, c: char) {
        if c == '\u{FE0F}' {
            return self.vs16();
        }
        let w = self.width(c);
        if self.at.0 + w > usize::from(W) && self.at.1 + 1 < usize::from(H) {
            // 写到行尾了：折到下一行开头。
            self.at = (0, self.at.1 + 1);
        }
        let (x, y) = self.at;
        if x + w > usize::from(W) {
            return;
        }
        for k in 0..w {
            self.split(x + k, y);
            self.ghosts.remove(&(x + k, y));
        }
        if self.overflow && shaky(&c.to_string()) && x + 1 < usize::from(W) {
            self.ghosts.insert((x + 1, y));
        }
        self.cells[y][x] = c.to_string();
        for k in 1..w {
            self.cells[y][x + k] = String::new();
        }
        self.at.0 += w;
    }

    /// `U+FE0F` 跟在前一个字后面：照一格画的终端只把它记在那一格里；kitty 把那个字撑成两格，后面那一格算它的后半，
    /// 光标再往右一格。
    fn vs16(&mut self) {
        let (x, y) = self.at;
        let Some(prev) = x.checked_sub(1) else {
            return;
        };
        self.cells[y][prev].push('\u{FE0F}');
        if self.emoji == 2 && x < usize::from(W) {
            self.split(x, y);
            self.cells[y][x] = String::new();
            self.at.0 += 1;
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                self.print(c);
                continue;
            }
            if chars.next_if_eq(&'[').is_none() {
                chars.next();
                continue;
            }
            let mut params = String::new();
            let mut end = ' ';
            for p in chars.by_ref() {
                if p.is_ascii_alphabetic() || p == '~' {
                    end = p;
                    break;
                }
                params.push(p);
            }
            if end == 'H' {
                let mut n = params.split(';').map(|v| v.parse::<usize>().unwrap_or(1));
                let (row, col) = (n.next().unwrap_or(1), n.next().unwrap_or(1));
                self.at = (col - 1, row - 1);
            }
        }
    }

    /// 屏上第 `y` 行：这种字当两格时，它的后半记成空格（界面那里排的就是字后面的空格）；别的宽字的后半不写。
    fn row(&self, y: usize) -> String {
        let row = &self.cells[y];
        row.iter()
            .enumerate()
            .map(|(x, cell)| {
                // 字还在的，那半格就是它自己；字不在了还留着的才是残影。
                let orphan = x == 0 || !shaky(&row[x - 1]);
                if self.ghosts.contains(&(x, y)) && orphan {
                    GHOST
                } else if cell.is_empty() && x > 0 && shaky(&row[x - 1]) {
                    " "
                } else if x > 0 && row[x - 1].ends_with('\u{FE0F}') && cell == " " {
                    // 照一格画带 VS16 的 emoji 的终端：它后面那一格是空的（界面那里是它盖住的后半）。
                    ""
                } else {
                    cell.as_str()
                }
            })
            .collect()
    }
}

/// 界面排的第 `y` 行（宽字后面跳过的那一格不写）。
fn intended(buf: &Buffer, y: u16) -> String {
    let mut out = String::new();
    let mut x = 0;
    while x < W {
        let symbol = buf[(x, y)].symbol();
        out.push_str(symbol);
        x += u16::try_from(symbol.width().max(1)).unwrap_or(1);
    }
    out
}

/// 一帧：从第 `top` 行起，几行字。思考那一行上面是一行英文：图标右边那一格往上推时每一帧都变，ratatui 接着图标往后写，
/// 当两格的终端就错开一格（只有中文的话那一格两帧都是宽字的后半，不重写，碰巧对得上）。
fn frame(top: u16, reply: &str) -> Vec<(u16, String)> {
    vec![
        (top, "  hello there, Miyu".into()),
        (top + 1, format!("  {ATOM} 已思考 · 2.1s")),
        (top + 2, reply.into()),
    ]
}

/// 回答那一行的两种：打头是英文、第二格是空格（思考那一行推走以后，图标右边那一格还是空格，ratatui 不重写它）；
/// 打头是中文（宽字盖着图标右边那一格）。
const REPLIES: [&str; 2] = ["  I see 她的回答", "  她的回答写在这里"];

/// 把一帧的几行字画上。
fn lay(f: &mut ratatui::Frame, lines: &[(u16, String)]) {
    for (y, text) in lines {
        f.render_widget(Line::raw(text.clone()), Rect::new(0, *y, W, 1));
    }
}

/// 不包这一层，一帧帧画 `frames`（每一帧几行字），交回界面排的最后一帧。
fn draw_all<B: ratatui::backend::Backend>(
    t: &mut Terminal<B>,
    frames: &[Vec<(u16, String)>],
) -> Buffer {
    let mut last = None;
    for lines in frames {
        let done = t
            .draw(|f| lay(f, lines))
            .unwrap_or_else(|_| panic!("画得出来"));
        last = Some(done.buffer.clone());
    }
    last.unwrap()
}

/// 照界面画的走法（画面那一层，变了的行整行重写、经这一层送）一帧帧画，交回界面排的最后一帧和写出去的字节。
/// `parks` 里是哪几帧以前先放掉画面的缓冲（那一帧照段比）。
fn draw_guarded(frames: &[Vec<(u16, String)>], parks: &[usize]) -> (Buffer, Vec<u8>) {
    let mut screen = Screen::new(Wire::new(W, H));
    let mut last = None;
    let mut bytes = Vec::new();
    for (n, lines) in frames.iter().enumerate() {
        if parks.contains(&n) {
            screen.park();
        }
        let (buf, sent) = screen_frame(&mut screen, |buf| {
            for (y, text) in lines {
                Line::raw(text.clone()).render(Rect::new(0, *y, W, 1), buf);
            }
            None
        });
        bytes.extend(sent);
        last = Some(buf);
    }
    (last.unwrap(), bytes)
}

/// 照 `guarded` 包不包这一层画 `frames`，交回模拟终端（这种字当 `wide` 格，`overflow` 见 [`Model`]）里每一行和界面排
/// 的每一行。
fn shown(guarded: bool, kind: Kind, frames: &[Vec<(u16, String)>]) -> (Vec<String>, Vec<String>) {
    shown_with(guarded, &[], kind, frames)
}

/// 同 [`shown`]，`parks` 见 [`draw_guarded`]。
pub(super) fn shown_with(
    guarded: bool,
    parks: &[usize],
    kind: Kind,
    frames: &[Vec<(u16, String)>],
) -> (Vec<String>, Vec<String>) {
    let options = || TerminalOptions {
        viewport: Viewport::Fixed(Rect::new(0, 0, W, H)),
    };
    let (last, bytes) = if guarded {
        draw_guarded(frames, parks)
    } else {
        let plain = CrosstermBackend::new(Vec::<u8>::new());
        let mut t = Terminal::with_options(plain, options()).unwrap();
        (draw_all(&mut t, frames), t.backend().writer().clone())
    };
    let mut model = Model::new(kind);
    model.feed(&bytes);
    let screen = (0..usize::from(H)).map(|y| model.row(y)).collect();
    let want = (0..H).map(|y| intended(&last, y)).collect();
    (screen, want)
}

/// 四种终端：kitty（这种字照一格画、带 VS16 的 emoji 两格）、这种字当两格的、当一格算却画出两格宽的、带 VS16 的
/// emoji 照一格画的。
pub(super) const TERMINALS: [Kind; 4] = [(1, false, 2), (2, false, 2), (1, true, 2), (1, false, 1)];

/// 一种模拟终端：这种字当几格、画不画出两格宽、带 VS16 的 emoji 当几格（见 [`Model`]）。
pub(super) type Kind = (usize, bool, usize);

/// 思考那一行往上推三回（每一回整体往上一行）。
fn push_up(reply: &str) -> Vec<Vec<(u16, String)>> {
    [3, 2, 1].map(|top| frame(top, reply)).to_vec()
}

#[test]
fn without_the_guard_the_other_two_kinds_of_terminal_go_wrong() {
    // 修以前的样子：当两格的错开一格，画出两格宽的留下半个字形。
    for kind in &TERMINALS[1..3] {
        let (screen, want) = shown(false, *kind, &push_up(REPLIES[0]));
        assert_ne!(
            screen, want,
            "{kind:?}：不包这一层也对得上，这个测试就守不住什么"
        );
    }
}

#[test]
fn with_the_guard_every_kind_of_terminal_matches_what_was_laid_out() {
    for kind in TERMINALS {
        for reply in REPLIES {
            let (screen, want) = shown(true, kind, &push_up(reply));
            assert_eq!(screen, want, "{kind:?}，回答那一行「{reply}」");
        }
    }
}

#[test]
fn the_atom_stays_when_only_the_cell_on_its_right_changes() {
    // 当两格的终端里改了字的后半，整个字就没了：字原样补回来。
    let frames = [
        vec![(1, format!("  {ATOM} 已思考"))],
        vec![(1, format!("  {ATOM}·已思考"))],
    ];
    let (screen, _) = shown(true, (2, false, 2), &frames);
    assert!(screen[1].contains(ATOM), "{screen:?}");
}

#[test]
fn only_the_supplementary_private_use_planes_count_and_never_a_picture_cell() {
    assert!(shaky(ATOM));
    assert!(!shaky("\u{e27f}"), "基本平面的私用区一格，各家都认");
    assert!(!shaky("你"));
    assert!(!shaky("\u{10EEEE}"), "kitty 图片的占位字不归这一层管");
}

#[test]
fn the_atom_survives_when_another_row_changes_in_the_same_frame() {
    // 第二帧最底下一行也变了、又改了图标右边那一格：两行一起重写，图标还得照原样补回来（原来守的是记着的格子照新的
    // 大小重排，LW-17 起不记了，情形照旧守着）。
    let frames = [
        vec![(1, format!("  {ATOM} 已思考"))],
        vec![
            (1, format!("  {ATOM}·已思考")),
            (H - 1, "x".repeat(usize::from(W))),
        ],
    ];
    let (screen, _) = shown(true, (2, false, 2), &frames);
    assert!(screen[1].contains(ATOM), "{screen:?}");
}

#[test]
fn the_cell_right_of_a_new_glyph_is_rewritten_with_what_it_holds() {
    // 第二帧图标换上来，右边那一格（`b`）两帧一样：整行重写时照这一帧的缓冲把 `b` 原样补写，不写成空白。
    let frames = [
        vec![(1, "  ab 已思考".to_string())],
        vec![(1, format!("  {ATOM}b 已思考"))],
    ];
    let (screen, want) = shown(true, (1, false, 2), &frames);
    assert_eq!(screen, want);
}

/// 第 1 行：`x` 那一格写 `glyph`，别的格子照 `fill`（左边一段、右边一段各一种字母），`right` 是它右边那一格。
fn placed(x: usize, glyph: &str, right: char, fill: (char, char)) -> Vec<(u16, String)> {
    let mut cells: Vec<String> = (0..usize::from(W))
        .map(|i| if i < SEGMENT { fill.0 } else { fill.1 }.to_string())
        .collect();
    cells[x] = glyph.into();
    cells[x + 1] = right.into();
    vec![(1, cells.concat())]
}

#[test]
fn glyphs_at_either_side_of_a_segment_boundary_stay_whole() {
    // 这种字在画面第一段的最后一格、第二段的第一格（`SEGMENT` 格一段）。它自己变（出现、换成别的、消失），
    // 它右边那一格变，它左右两段里别的格子变：三种终端里屏上都和界面排的一样。
    for x in [SEGMENT - 1, SEGMENT] {
        let cases: [Vec<Vec<(u16, String)>>; 6] = [
            // 右边那一格变了（字后面照规矩是空格；换成点）。
            vec![
                placed(x, ATOM, ' ', ('a', 'b')),
                placed(x, ATOM, '·', ('a', 'b')),
            ],
            // 它出现。
            vec![
                placed(x, "c", ' ', ('a', 'b')),
                placed(x, ATOM, ' ', ('a', 'b')),
            ],
            // 它换成别的字、再换回来。
            vec![
                placed(x, ATOM, ' ', ('a', 'b')),
                placed(x, "c", ' ', ('a', 'b')),
                placed(x, ATOM, ' ', ('a', 'b')),
            ],
            // 它换成别的字就停在那里（当一格算、画出两格宽的终端里，右边那一格留着半个字形，得跟着重写）。
            vec![
                placed(x, ATOM, ' ', ('a', 'b')),
                placed(x, "c", ' ', ('a', 'b')),
            ],
            // 只有它左边那一段变了。
            vec![
                placed(x, ATOM, ' ', ('a', 'b')),
                placed(x, ATOM, ' ', ('d', 'b')),
            ],
            // 只有它右边那一段变了。
            vec![
                placed(x, ATOM, ' ', ('a', 'b')),
                placed(x, ATOM, ' ', ('a', 'e')),
            ],
        ];
        for (n, frames) in cases.iter().enumerate() {
            // 活跃时逐格比、放掉以后醒来照段比，两条路都走。
            for (kind, parks) in TERMINALS
                .into_iter()
                .flat_map(|k| [(k, vec![]), (k, vec![1, 2])])
            {
                let (screen, want) = shown_with(true, &parks, kind, frames);
                if n == 0 && kind == (2, false, 2) {
                    // 当两格的终端里字的后半就盖在右边那一格上，点显示不出来；字得还在（同上面那条）。
                    assert!(screen[1].contains(ATOM), "第 {x} 格，{screen:?}");
                } else {
                    assert_eq!(
                        screen, want,
                        "第 {x} 格，第 {n} 种，{kind:?}，放掉 {parks:?}"
                    );
                }
            }
        }
    }
}
