//! 跟随头像的配色（蓝图 `tui.md`「主题」第 5 条、「跟随头像的配色」，2026-10-13 项目主人定）：主色照人格的 `seed`，没写的
//! 照头像的候选里打分最高的（候选由核心算、人格信息里给，壁纸的用同一份门面 `miyu_client::seeds`，2026-10-14 项目主人定）；
//! 照 MD3 `SchemeContent` 生成几条色板，每一格都换（`overlay`）。两个头同一套算法（2026-10-14 项目主人定，
//! 网页 `web-demo/src/lib/md3/index.js` 照这个改），同一个人格出来的颜色一样。

use material_colors::blend::harmonize;
use material_colors::color::Rgb;
use material_colors::hct::Hct;
use material_colors::palette::TonalPalette;
use material_colors::scheme::variant::SchemeContent;
use ratatui::style::Color;

use super::{Palette, Tone};

/// 人格里自选的主色 `#rrggbb`；写错的是 `None`。
pub fn hex(seed: &str) -> Option<u32> {
    let digits = seed.strip_prefix('#')?;
    (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
}

/// 照主色生成一套盖在 `base`（tokyonight 深色或浅色那一份）上，两个头同一套算法（2026-10-14 项目主人定统一，网页
/// `theme-persona` 照这个改）：MD3 `SchemeContent`，对比度 0。不带色相的、强调色每一格照 `base` 那一格的亮度（tone），从
/// 主色生成的几条色板里取同一亮度的颜色，明暗层次和 tokyonight 一样，只换色相（项目主人报：思考还是青的、深色的底太深）：
/// 底、正文、面板、思考、粗体、行内代码、公式从中性色板，暗字、边框、引用、斜体、图里的字线、代码注释和运算符从中性变体，
/// 强调色、链接、表头、热度图从主色，标签的底从次色，选中、图片从第三色。有含义的颜色（出错、增删、警告、成功、权限级别、
/// 高亮、标题、代码着色其余几格）色相往主色最多转 15°（`harmonize`），认得出是哪一种；吉祥物也跟着换。
pub fn overlay(base: &Palette, seed: u32, dark: bool) -> Palette {
    let source = Rgb::from_u32(seed);
    let s = SchemeContent::new(Hct::new(source), dark, Some(0.0)).scheme;
    let neutral = &s.neutral_palette;
    let variant = &s.neutral_variant_palette;
    let primary = &s.primary_palette;
    let second = &s.secondary_palette;
    let third = &s.tertiary_palette;
    let like = |from: &TonalPalette, tone: Tone| match tone.0 {
        Color::Rgb(r, g, b) => {
            let lightness = Hct::new(Rgb::new(r, g, b)).get_tone();
            let c = Rgb::from(from.get_hct(lightness));
            Tone::of(Color::Rgb(c.red, c.green, c.blue))
        }
        _ => tone,
    };
    let near = |tone: Tone| match tone.0 {
        Color::Rgb(r, g, b) => {
            let c = harmonize(Rgb::new(r, g, b), source);
            Tone::of(Color::Rgb(c.red, c.green, c.blue))
        }
        _ => tone,
    };
    let b = base;
    Palette {
        dim: like(variant, b.dim),
        accent: like(primary, b.accent),
        workspace: like(primary, b.workspace),
        full: near(b.full),
        read_only: near(b.read_only),
        picked: like(third, b.picked),
        thought: like(neutral, b.thought),
        thought_hover: like(neutral, b.thought_hover),
        shade: like(neutral, b.shade),
        chip_bg: like(second, b.chip_bg),
        chip_hover_bg: like(second, b.chip_hover_bg),
        panel_bg: like(neutral, b.panel_bg),
        row_bg: like(primary, b.row_bg),
        row_dim_bg: like(neutral, b.row_dim_bg),
        heat: [
            like(neutral, b.heat[0]),
            like(primary, b.heat[1]),
            like(primary, b.heat[2]),
            like(primary, b.heat[3]),
            like(primary, b.heat[4]),
        ],
        added: near(b.added),
        removed: near(b.removed),
        diff_added_fg: near(b.diff_added_fg),
        diff_added_bg: near(b.diff_added_bg),
        diff_removed_fg: near(b.diff_removed_fg),
        diff_removed_bg: near(b.diff_removed_bg),
        error: near(b.error),
        warn: near(b.warn),
        good: near(b.good),
        md_heading: near(b.md_heading),
        md_bold: like(neutral, b.md_bold),
        md_italic: like(variant, b.md_italic),
        md_code: like(neutral, b.md_code),
        md_link: like(primary, b.md_link),
        md_url: like(primary, b.md_url),
        md_image: like(third, b.md_image),
        md_quote: like(variant, b.md_quote),
        md_mark: near(b.md_mark),
        md_table_head: like(primary, b.md_table_head),
        md_math: like(neutral, b.md_math),
        diagram_text: like(variant, b.diagram_text),
        diagram_line: like(variant, b.diagram_line),
        diagram_backdrop: like(neutral, b.diagram_backdrop),
        side_title: like(primary, b.side_title),
        hover: like(variant, b.hover),
        faint: like(variant, b.faint),
        on_accent: like(neutral, b.on_accent),
        code_keyword: near(b.code_keyword),
        code_function: near(b.code_function),
        code_string: near(b.code_string),
        code_number: near(b.code_number),
        code_comment: like(variant, b.code_comment),
        code_type: near(b.code_type),
        code_constant: near(b.code_constant),
        code_property: near(b.code_property),
        code_operator: like(variant, b.code_operator),
        code_macro: near(b.code_macro),
        code_parameter: near(b.code_parameter),
        // 吉祥物也跟着换（2026-10-14 项目主人定）：头、耳朵照中性色，眼睛照主色，鳍照第三色。
        mascot_head: like(neutral, b.mascot_head),
        mascot_ear: like(variant, b.mascot_ear),
        mascot_eye: like(primary, b.mascot_eye),
        mascot_fin: like(third, b.mascot_fin),
        background: like(neutral, b.background),
        text: like(neutral, b.text),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use material_colors::color::Rgb;
    use material_colors::hct::Hct;

    use super::{hex, overlay};
    use crate::theme::Tone;
    use crate::theme::builtin;

    #[test]
    fn a_hand_picked_seed_is_read_and_a_bad_one_is_not() {
        assert_eq!(hex("#7aa2f7"), Some(0x7aa2f7));
        assert_eq!(hex("7aa2f7"), None);
        assert_eq!(hex("#7aa2f"), None);
    }

    /// 一格的 HCT：色相、色度、亮度。
    fn hct(tone: Tone) -> (f64, f64, f64) {
        let Color::Rgb(r, g, b) = tone.0 else {
            panic!("{tone:?}")
        };
        let h = Hct::new(Rgb::new(r, g, b));
        (h.get_hue(), h.get_chroma(), h.get_tone())
    }

    #[test]
    fn every_neutral_and_accent_follows_at_tokyonights_lightness_and_meanings_only_lean() {
        // 2026-10-14 项目主人报思考还是青的、深色的底太深，定两个头统一一套：照 tokyonight 每一格的亮度取色，有含义的颜色
        // 色相往主色最多转 15°，吉祥物也跟着换（同一天定）。
        let themes = builtin().unwrap();
        for name in ["tokyonight-night", "tokyonight-day"] {
            let (_, base) = themes.iter().find(|(n, _)| n == name).unwrap();
            let green = overlay(base, 0x2fab34, name.ends_with("night"));
            for (got, was, what) in [
                (green.background, base.background, "底"),
                (green.text, base.text, "正文"),
                (green.thought, base.thought, "思考"),
                (green.accent, base.accent, "强调色"),
                (green.mascot_head, base.mascot_head, "吉祥物的头"),
                (green.mascot_eye, base.mascot_eye, "吉祥物的眼睛"),
            ] {
                assert_ne!(got, was, "{name} {what}跟着主色");
                assert!(
                    (hct(got).2 - hct(was).2).abs() < 1.0,
                    "{name} {what}和 tokyonight 一样亮"
                );
            }
            assert!(hct(green.thought).1 < 12.0, "{name} 思考是中性色");
            let (error, before) = (hct(green.error), hct(base.error));
            let turned = (error.0 - before.0)
                .abs()
                .min(360.0 - (error.0 - before.0).abs());
            assert!(turned <= 15.5, "{name} 出错的红只偏一点：{turned}");
            assert!((error.2 - before.2).abs() < 1.0, "{name} 出错的红亮度不变");
        }
    }
}
