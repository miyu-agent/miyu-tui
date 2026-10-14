//! 终端配色的底色照终端的真颜色推（蓝图 `tui.md`「终端配色」，2026-10-14 项目主人定）：字全用终端自己的 16 色（主题文件里
//! 写名字），面板、点开的一步、标签、选中行这些底色照终端的底色往字色掺一点，差异的底往终端的绿、红掺一点，mermaid 图的
//! 字、线、底照终端的字色、底色。终端没答底色的照主题文件里写的；没答 16 色的，差异的底照主题文件。

use ratatui::style::Color;

use super::terminal_colors::{Colors, Ground};
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

/// 照问到的终端颜色推一套：`base` 是主题文件那一份（终端配色深色或浅色）。
pub fn terminal(base: &Palette, ground: Ground, sixteen: Option<&Colors>) -> Palette {
    let Ground { fg, bg, .. } = ground;
    let toward = |to: (u8, u8, u8), t: f64| Tone::of(rgb(mix(bg, to, t)));
    let mut out = base.clone();
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
    out
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
    use crate::theme::terminal_colors::Ground;

    fn base() -> crate::theme::Palette {
        let themes = builtin().unwrap();
        themes.into_iter().find(|(n, _)| n == "terminal").unwrap().1
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
        let out = terminal(&base(), ground, Some(&sixteen));
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
        // 字照主题文件写的终端 16 色名字，不动。
        assert_eq!(out.accent, base().accent);
    }

    #[test]
    fn without_sixteen_colors_the_diff_grounds_stay_as_the_theme_file_says() {
        let ground = Ground {
            fg: (40, 40, 40),
            bg: (240, 240, 240),
            dark: false,
        };
        let out = terminal(&base(), ground, None);
        assert_eq!(out.diff_added_bg, base().diff_added_bg);
        let Color::Rgb(r, ..) = out.panel_bg.0 else {
            panic!()
        };
        assert!(r < 240 && r > 200, "浅色的底往深的字掺一点：{r}");
    }
}
