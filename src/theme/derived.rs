//! 终端配色照终端的真颜色生成（蓝图 `tui.md`「终端配色」）。
//!
//! - 底色（2026-10-14 项目主人定）：面板、点开的一步、标签、选中行这些底色照终端的底色往字色掺一点，差异的底往终端的
//!   绿、红掺一点，mermaid 图的字、线、底照终端的字色、底色。终端没答底色的照主题文件里写的；没答 16 色的，差异的底照
//!   主题文件。
//! - 字（2026-10-19 项目主人定，取代 H15 逐格换成终端那一号的真颜色）：真彩色、16 色和字色底色都问到了，拿 16 色里最
//!   鲜艳的那个当主色生成 MD3（`seeds`、`seeded`，和跟随头像、跟随壁纸同一套），底照旧从终端的底推、不铺。不是真彩色、
//!   问不到的照旧写 16 色的名字，字直接用终端的 16 色。

use std::sync::{Mutex, PoisonError};

use ratatui::style::Color;

use super::terminal_colors::{Colors, Ground, Seen};
use super::{Palette, Tone};

#[cfg(test)]
mod tests;

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

/// 一个颜色的红、绿、蓝。
type Rgb3 = (u8, u8, u8);

/// 16 色里带颜色的几号：1 到 6、9 到 14（黑、白、灰不算）。
const HUES: [usize; 12] = [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14];

/// 照问到的终端颜色推底色：`base` 是主题文件那一份（终端配色深色或浅色）。没问到字色、底色的照主题文件。
pub fn terminal(base: &Palette, seen: Seen) -> Palette {
    let mut out = base.clone();
    if let Some(ground) = seen.ground {
        let diff = seen.sixteen.map(|s| (s[2], s[1]));
        grounds(&mut out, ground.bg, ground.fg, diff);
    }
    out
}

/// 终端的主色候选（「主色」第 4a 条）：真彩色、16 色和字色底色都问到了，带颜色的那 12 号照跟随壁纸同一套打分挑（门面
/// `from_rgba`），好的在前；别的是空的，照 [`terminal`] 回退。
pub fn seeds(seen: Seen) -> Vec<u32> {
    let Some(sixteen) = seen
        .sixteen
        .filter(|_| seen.truecolor && seen.ground.is_some())
    else {
        return Vec::new();
    };
    // 每一帧都要看一次候选（`app/theme.rs` 的 `follow_theme`）：照这 16 色记住上次挑的，变了才重挑。
    static LAST: Mutex<Option<(Colors, Vec<u32>)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((colors, seeds)) = last.as_ref()
        && *colors == sixteen
    {
        return seeds.clone();
    }
    let rgba: Vec<u8> = HUES
        .iter()
        .flat_map(|&n| {
            let (r, g, b) = sixteen[n];
            [r, g, b, 255]
        })
        .collect();
    // 只认终端自己的颜色：一个都不合格时打分给的 Google 蓝不是终端的，没有候选，照 16 色那一套。
    let own: Vec<u32> = HUES.iter().map(|&n| hex(sixteen[n])).collect();
    let seeds: Vec<u32> = miyu_client::seeds::from_rgba(&rgba)
        .into_iter()
        .map(|s| s.argb() & 0x00ff_ffff)
        .filter(|s| own.contains(s))
        .collect();
    *last = Some((sixteen, seeds.clone()));
    seeds
}

/// 照主色生成好的一套（`md3`，盖在 tokyonight 上的）换上终端的底：面板这些从终端的底往生成的正文色掺，差异的底往生成的
/// 增、删字色掺，mermaid 图的底是终端的底。没问到底色的原样。
pub fn seeded(md3: Palette, seen: Seen) -> Palette {
    let Some(Ground { bg, .. }) = seen.ground else {
        return md3;
    };
    let mut out = md3;
    let text = rgb_of(out.text).unwrap_or(bg);
    let diff = rgb_of(out.diff_added_fg).zip(rgb_of(out.diff_removed_fg));
    grounds(&mut out, bg, text, diff);
    out
}

fn hex((r, g, b): Rgb3) -> u32 {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn rgb_of(tone: Tone) -> Option<Rgb3> {
    match tone.0 {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
}

/// 底色照 `bg` 往 `fg` 掺，差异的底往 `diff` 的绿、红掺（有才掺），mermaid 图照 `fg`、`bg`。
fn grounds(out: &mut Palette, bg: Rgb3, fg: Rgb3, diff: Option<(Rgb3, Rgb3)>) {
    let toward = |to: Rgb3, t: f64| Tone::of(rgb(mix(bg, to, t)));
    out.row_dim_bg = toward(fg, ROW_DIM);
    out.panel_bg = toward(fg, PANEL);
    out.shade = toward(fg, PANEL);
    out.chip_bg = toward(fg, CHIP);
    out.chip_hover_bg = toward(fg, CHIP_HOVER);
    out.row_bg = toward(fg, ROW);
    out.diagram_text = Tone::of(rgb(fg));
    out.diagram_line = toward(fg, LINE);
    out.diagram_backdrop = Tone::of(rgb(bg));
    if let Some((green, red)) = diff {
        out.diff_added_bg = toward(green, DIFF);
        out.diff_removed_bg = toward(red, DIFF);
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
