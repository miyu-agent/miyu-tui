//! 终端配色照终端的真颜色生成（蓝图 `tui.md`「终端配色」）。
//!
//! - 底色（2026-10-14 项目主人定）：面板、点开的一步、标签、选中行这些底色照终端的底色往字色掺一点，差异的底往终端的
//!   绿、红掺一点，mermaid 图的字、线、底照终端的字色、底色。终端没答底色的照主题文件里写的；没答 16 色的，差异的底照
//!   主题文件。
//! - 字（2026-10-15 项目主人定，设计 13 的 H15）：真彩色、16 色和字色底色都问到了，主题文件里写 16 色名字的格子换成终端
//!   那几号的真颜色；写 `dim:` 的往底色掺，不靠终端的「变暗」；暗一档的字、悬停亮一档照字色往底色掺。不是真彩色、问不到
//!   的照旧写 16 色的名字，字直接用终端的 16 色。

use ratatui::style::Color;

use super::terminal_colors::{Colors, Ground, Seen, index};
use super::{Palette, Tone};

/// 底色往字色掺多少：没选中的一行、面板和点开的一步、标签、悬停的标签、选中的一行。
const ROW_DIM: f64 = 0.04;
const PANEL: f64 = 0.08;
const CHIP: f64 = 0.12;
const CHIP_HOVER: f64 = 0.16;
const ROW: f64 = 0.18;
/// 差异的底往绿、红掺多少。
const DIFF: f64 = 0.15;
/// mermaid 的线：底色往字色掺多少。
const LINE: f64 = 0.55;
/// 字往底色掺多少（照 tokyonight 深色那一份量的：暗字在正文色往底色三分之一处，更暗的说明六成，悬停亮一档一成半）：
/// 暗一档的字、更暗的说明、悬停的字、写了 `dim:` 的。
const DIM_TEXT: f64 = 0.33;
const FAINT_TEXT: f64 = 0.6;
const HOVER_TEXT: f64 = 0.14;
const DIMMED: f64 = 0.4;

/// 照问到的终端颜色生成一套：`base` 是主题文件那一份（终端配色深色或浅色）。没问到字色、底色的照主题文件。
pub fn terminal(base: &Palette, seen: Seen) -> Palette {
    let Some(ground) = seen.ground else {
        return base.clone();
    };
    let mut out = match seen.sixteen.filter(|_| seen.truecolor) {
        Some(sixteen) => true_text(base, ground, &sixteen),
        None => base.clone(),
    };
    grounds(&mut out, ground, seen.sixteen.as_ref());
    out
}

/// 字换成终端那几号的真颜色：写名字的照问到的那一号，写 `dim:` 的往底色掺；暗字、说明、悬停照字色往底色掺，铺强调色
/// 的底上的字照对比度挑终端的字色或底色。终端自己的字色（`default`）照旧。
fn true_text(base: &Palette, ground: Ground, sixteen: &Colors) -> Palette {
    let Ground { fg, bg, .. } = ground;
    let fade = |from: (u8, u8, u8), t: f64| Tone::of(rgb(mix(from, bg, t)));
    let mut out = base.map(|tone| match index(tone.0) {
        Some(n) if tone.1 => fade(sixteen[n], DIMMED),
        Some(n) => Tone::of(rgb(sixteen[n])),
        None => tone,
    });
    out.dim = fade(fg, DIM_TEXT);
    out.md_quote = out.dim;
    out.faint = fade(fg, FAINT_TEXT);
    out.mascot_ear = out.faint;
    out.hover = fade(fg, HOVER_TEXT);
    out.on_accent = match out.accent.0 {
        Color::Rgb(r, g, b) => Tone::of(rgb(legible((r, g, b), fg, bg))),
        _ => Tone::of(rgb(bg)),
    };
    out
}

/// 铺在 `under` 上的字：终端的字色、底色里和它对比度高的那个（终端的蓝可能是浅粉这类浅色，浅底的终端照底色写就看不清）。
fn legible(under: (u8, u8, u8), fg: (u8, u8, u8), bg: (u8, u8, u8)) -> (u8, u8, u8) {
    if contrast(under, fg) >= contrast(under, bg) {
        fg
    } else {
        bg
    }
}

/// 两个颜色的对比度（WCAG 2 的相对亮度之比，1 到 21）。
fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let luminance = |(r, g, b): (u8, u8, u8)| {
        let linear = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    };
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// 底色照终端的底往字色掺，差异的底往终端的绿、红掺（问到了 16 色才掺），mermaid 图照字色、底色。
fn grounds(out: &mut Palette, ground: Ground, sixteen: Option<&Colors>) {
    let Ground { fg, bg, .. } = ground;
    let toward = |to: (u8, u8, u8), t: f64| Tone::of(rgb(mix(bg, to, t)));
    out.row_dim_bg = toward(fg, ROW_DIM);
    out.panel_bg = toward(fg, PANEL);
    out.shade = toward(fg, PANEL);
    out.chip_bg = toward(fg, CHIP);
    out.chip_hover_bg = toward(fg, CHIP_HOVER);
    out.row_bg = toward(fg, ROW);
    out.diagram_text = Tone::of(rgb(fg));
    out.diagram_line = toward(fg, LINE);
    out.diagram_backdrop = Tone::of(rgb(bg));
    if let Some(sixteen) = sixteen {
        out.diff_added_bg = toward(sixteen[2], DIFF);
        out.diff_removed_bg = toward(sixteen[1], DIFF);
    }
}

/// 从 `from` 往 `to` 走 `t`（0 到 1）。
fn mix(from: (u8, u8, u8), to: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let step = |a: u8, b: u8| {
        let v = f64::from(a) + (f64::from(b) - f64::from(a)) * t;
        v.round().clamp(0.0, 255.0) as u8
    };
    (step(from.0, to.0), step(from.1, to.1), step(from.2, to.2))
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::terminal;
    use crate::theme::builtin;
    use crate::theme::terminal_colors::{Colors, Ground, Seen, index};
    use crate::theme::{Palette, Tone};

    fn base() -> Palette {
        named("terminal")
    }

    fn named(name: &str) -> Palette {
        let themes = builtin().unwrap();
        themes.into_iter().find(|(n, _)| n == name).unwrap().1
    }

    /// 问到的：`truecolor` 是不是真彩色。
    fn seen(ground: Ground, sixteen: Option<Colors>, truecolor: bool) -> Seen {
        Seen {
            sixteen,
            ground: Some(ground),
            truecolor,
        }
    }

    #[test]
    fn panels_lean_from_the_terminals_ground_toward_its_text() {
        // 2026-10-14 项目主人照推荐定：面板、差异这些底色照终端的底色推，不再是写死的 256 色序号。
        let ground = Ground {
            fg: (200, 200, 200),
            bg: (20, 20, 20),
            dark: true,
        };
        let mut sixteen = [(0, 0, 0); 16];
        sixteen[1] = (220, 40, 40);
        sixteen[2] = (40, 220, 40);
        let out = terminal(&base(), seen(ground, Some(sixteen), false));
        let Color::Rgb(r, g, b) = out.panel_bg.0 else {
            panic!("{:?}", out.panel_bg)
        };
        assert!(
            (25..=40).contains(&r) && r == g && g == b,
            "比底亮一点的灰：{r}"
        );
        let Color::Rgb(_, g, _) = out.diff_added_bg.0 else {
            panic!()
        };
        let Color::Rgb(r, _, _) = out.diff_removed_bg.0 else {
            panic!()
        };
        assert!(g > 40 && r > 40, "差异的底往绿、红掺了一点");
        assert_eq!(out.diagram_backdrop.0, Color::Rgb(20, 20, 20));
        // 不是真彩色：字照主题文件写的终端 16 色名字，不动。
        assert_eq!(out.accent, base().accent);
    }

    #[test]
    fn without_sixteen_colors_the_diff_grounds_stay_as_the_theme_file_says() {
        let ground = Ground {
            fg: (40, 40, 40),
            bg: (240, 240, 240),
            dark: false,
        };
        let out = terminal(&base(), seen(ground, None, true));
        assert_eq!(out.diff_added_bg, base().diff_added_bg);
        let Color::Rgb(r, ..) = out.panel_bg.0 else {
            panic!()
        };
        assert!(r < 240 && r > 200, "浅色的底往深的字掺一点：{r}");
    }

    #[test]
    fn with_truecolor_the_text_takes_the_terminals_own_colors() {
        // 2026-10-15 项目主人定（设计 13 的 H15）：真彩色、问到了 16 色和字色底色，字换成终端那几号的真颜色，暗字、悬停照
        // 字色往底色掺。
        let ground = Ground {
            fg: (200, 200, 200),
            bg: (20, 20, 20),
            dark: true,
        };
        let mut sixteen = [(0, 0, 0); 16];
        for (n, slot) in (0u8..).zip(sixteen.iter_mut()) {
            *slot = (n * 10, 100, 250 - n * 10);
        }
        let out = terminal(&base(), seen(ground, Some(sixteen), true));
        assert_eq!(
            out.accent,
            Tone::of(Color::Rgb(40, 100, 210)),
            "blue 是 4 号"
        );
        let Tone(Color::Rgb(r, g, b), false) = out.thought else {
            panic!("dim:lightgreen 往底色掺、不另加变暗：{:?}", out.thought)
        };
        assert!(r < 100 && g < 100 && b < 150, "比 10 号暗：{r} {g} {b}");
        let gray = |tone: Tone| match tone.0 {
            Color::Rgb(r, g, b) if r == g && g == b => r,
            other => panic!("{other:?}"),
        };
        let (hover, dim, faint) = (gray(out.hover), gray(out.dim), gray(out.faint));
        assert!(
            200 > hover && hover > dim && dim > faint && faint > 20,
            "{hover} {dim} {faint}"
        );
        assert_eq!(
            out.on_accent,
            Tone::of(Color::Rgb(20, 20, 20)),
            "亮蓝底上的字用深的底色"
        );
        // 浅底的终端、蓝是浅粉：强调色底上的字用深的字色，不用浅的底色（10-15 真 kitty 上看不清）。
        let light = Ground {
            fg: (30, 30, 30),
            bg: (235, 235, 235),
            dark: false,
        };
        let mut pink = sixteen;
        pink[4] = (0xff, 0xb4, 0xab);
        let out = terminal(&named("terminal-light"), seen(light, Some(pink), true));
        assert_eq!(out.on_accent, Tone::of(Color::Rgb(30, 30, 30)));
        assert_eq!(out.text, base().text, "终端自己的字色照旧");
        // 深浅两份都不再剩写 16 色名字的格子。
        for name in ["terminal", "terminal-light"] {
            let left = std::cell::Cell::new(0);
            let out = terminal(&named(name), seen(ground, Some(sixteen), true));
            let same = out.map(|tone| {
                left.set(left.get() + usize::from(index(tone.0).is_some()));
                tone
            });
            assert_eq!(same, out);
            assert_eq!(left.get(), 0, "{name}");
        }
        // 不是真彩色、没问到 16 色：照旧写名字。
        assert_eq!(
            terminal(&base(), seen(ground, Some(sixteen), false)).accent,
            base().accent
        );
        assert_eq!(
            terminal(&base(), seen(ground, None, true)).accent,
            base().accent
        );
    }
}
