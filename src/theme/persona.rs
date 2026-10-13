//! 跟随头像的配色（蓝图 `tui.md`「主题」第 5 条、「跟随头像的配色」，2026-10-13 项目主人定）：主色照人格的 `seed`，没写的从
//! 头像的像素取；照 MD3 `TonalSpot`、对比度 0 生成一套，只盖中性色和强调色。算法照网页的 `theme-persona`（`model.js`），
//! 两个头同一个人格出来的颜色一样。

use std::path::Path;

use material_colors::color::Rgb;
use material_colors::hct::Hct;
use material_colors::quantize::{Quantizer, QuantizerCelebi};
use material_colors::scheme::variant::SchemeTonalSpot;
use material_colors::score::Score;
use ratatui::style::Color;

use super::{Palette, Tone};

/// 取主色时最多挑多少种颜色（MD3 取壁纸色的做法）。
const QUANTIZE: usize = 128;
/// 太透明的像素不算（头像的圆角外面、抠了图的背景）。
const MIN_ALPHA: u8 = 200;
/// 取色前先缩到这么大（边长）：头像几百像素，缩小了取出来的主色一样，快得多。
const SIDE: u32 = 128;

/// 头像的主色：RGBA 的像素一格四个数，太透明的不算；一格都不剩的是 `None`。
pub fn seed_of(rgba: &[u8]) -> Option<u32> {
    let pixels: Vec<Rgb> = rgba
        .chunks_exact(4)
        .filter(|p| p[3] >= MIN_ALPHA)
        .map(|p| Rgb::new(p[0], p[1], p[2]))
        .collect();
    if pixels.is_empty() {
        return None;
    }
    let result = QuantizerCelebi::quantize(&pixels, QUANTIZE);
    let ranked = Score::score(&result.color_to_count, None, None, None);
    ranked.first().map(|c| argb(*c))
}

/// 读一张头像取主色；读不成的是 `None`。
pub fn seed_of_file(path: &Path) -> Option<u32> {
    let image = image::open(path).ok()?.thumbnail(SIDE, SIDE).to_rgba8();
    seed_of(image.as_raw())
}

/// 人格里自选的主色 `#rrggbb`；写错的是 `None`。
pub fn hex(seed: &str) -> Option<u32> {
    let digits = seed.strip_prefix('#')?;
    (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
}

/// 照主色生成一套盖在 `base` 上：中性色、强调色换成 MD3 的角色（深浅照 `dark`），出错的红、差异的红绿、链接、Markdown、
/// 代码高亮、吉祥物、思考的青照 `base`（语义色跟着人格变了反而认不出，同网页）。
pub fn overlay(base: &Palette, seed: u32, dark: bool) -> Palette {
    let s = SchemeTonalSpot::new(Hct::new(Rgb::from_u32(seed)), dark, Some(0.0)).scheme;
    let tone = |c: Rgb| Tone(Color::Rgb(c.red, c.green, c.blue));
    let mix = |a: Rgb, b: Rgb, t: f64| {
        let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
        Tone(Color::Rgb(
            m(a.red, b.red),
            m(a.green, b.green),
            m(a.blue, b.blue),
        ))
    };
    let (low, high) = (s.primary_container(), s.primary());
    Palette {
        dim: tone(s.on_surface_variant()),
        accent: tone(s.primary()),
        workspace: tone(s.primary()),
        picked: tone(s.tertiary()),
        shade: tone(s.surface_container_high()),
        chip_bg: tone(s.secondary_container()),
        chip_hover_bg: tone(s.tertiary_container()),
        panel_bg: tone(s.surface_container()),
        row_bg: tone(s.primary_container()),
        row_dim_bg: tone(s.surface_container_low()),
        // 热度图：最淡一格是底上浮起来的那一层，往上从主色的容器色走到主色。
        heat: [
            tone(s.surface_container_highest()),
            mix(low, high, 0.0),
            mix(low, high, 1.0 / 3.0),
            mix(low, high, 2.0 / 3.0),
            mix(low, high, 1.0),
        ],
        md_table_head: tone(s.primary()),
        diagram_backdrop: tone(s.surface()),
        side_title: tone(s.primary()),
        hover: tone(s.on_surface()),
        faint: tone(s.outline()),
        on_accent: tone(s.on_primary()),
        background: tone(s.surface()),
        text: tone(s.on_surface()),
        ..base.clone()
    }
}

/// `Rgb` 写成 `0xRRGGBB`。
fn argb(c: Rgb) -> u32 {
    (u32::from(c.red) << 16) | (u32::from(c.green) << 8) | u32::from(c.blue)
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::{hex, overlay, seed_of};
    use crate::theme::builtin;

    #[test]
    fn the_seed_is_the_main_color_and_see_through_pixels_do_not_count() {
        // 一张大部分是红、一小块绿的图：主色是红；透明的蓝不算。
        let mut rgba = Vec::new();
        for i in 0..400 {
            let px: [u8; 4] = if i < 300 {
                [0xd0, 0x30, 0x30, 255]
            } else if i < 340 {
                [0x30, 0xb0, 0x40, 255]
            } else {
                [0x20, 0x40, 0xf0, 10]
            };
            rgba.extend(px);
        }
        let seed = seed_of(&rgba).unwrap();
        let (r, g, b) = ((seed >> 16) & 0xff, (seed >> 8) & 0xff, seed & 0xff);
        assert!(r > g && r > b, "主色是红的：{seed:06x}");
        assert_eq!(seed_of(&[0, 0, 0, 0]), None, "全透明的没有主色");
    }

    #[test]
    fn a_hand_picked_seed_is_read_and_a_bad_one_is_not() {
        assert_eq!(hex("#7aa2f7"), Some(0x7aa2f7));
        assert_eq!(hex("7aa2f7"), None);
        assert_eq!(hex("#7aa2f"), None);
    }

    #[test]
    fn the_overlay_changes_the_neutrals_and_accents_but_keeps_the_meaningful_colors() {
        let themes = builtin().unwrap();
        let (_, night) = themes
            .iter()
            .find(|(n, _)| n == "tokyonight-night")
            .unwrap();
        let green = overlay(night, 0x2fab34, true);
        assert_ne!(green.accent, night.accent, "强调色跟着主色");
        assert_ne!(green.background, night.background, "底色跟着主色");
        assert_eq!(green.error, night.error, "出错的红不动");
        assert_eq!(green.diff_added_bg, night.diff_added_bg, "差异的颜色不动");
        assert_eq!(green.code_keyword, night.code_keyword, "代码高亮不动");
        let Color::Rgb(r, g, b) = green.background.0 else {
            panic!()
        };
        assert!(
            u32::from(r) + u32::from(g) + u32::from(b) < 3 * 0x40,
            "深色的底是暗的"
        );
        let (_, day) = themes.iter().find(|(n, _)| n == "tokyonight-day").unwrap();
        let Color::Rgb(r, g, b) = overlay(day, 0x2fab34, false).background.0 else {
            panic!()
        };
        assert!(
            u32::from(r) + u32::from(g) + u32::from(b) > 3 * 0xc0,
            "浅色的底是亮的"
        );
    }
}
