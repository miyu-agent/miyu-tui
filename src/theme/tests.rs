//! 主题的测试：思考不变暗、流光、终端配色的写法。

#[test]
fn a_dim_slot_is_drawn_dimmed_in_its_terminal_color() {
    // 终端配色照旧 Miyu（2026-10-14 项目主人照推荐定）：思考是暗的亮绿，不是 256 色 73 号的青。
    let themes = super::builtin().unwrap();
    let (_, terminal) = themes.iter().find(|(n, _)| n == "terminal").unwrap();
    assert_eq!(terminal.thought, super::Tone(Color::LightGreen, true));
    let style = super::styled(terminal.thought);
    assert_eq!(style.fg, Some(Color::LightGreen));
    assert!(style.add_modifier.contains(ratatui::style::Modifier::DIM));
    assert!(
        !super::styled(terminal.accent)
            .add_modifier
            .contains(ratatui::style::Modifier::DIM)
    );
}

#[test]
fn terminal_colors_write_every_text_slot_with_the_terminals_own_sixteen() {
    // 「字全用终端自己的 16 色」（2026-10-14）：只有底色（照终端底色推，主题文件里是推不出来时的底）和 mermaid 的图（图
    // 只能用真颜色）写固定的颜色。
    let grounds = [
        "shade",
        "chip_bg",
        "chip_hover_bg",
        "panel_bg",
        "row_bg",
        "row_dim_bg",
        "diff_added_bg",
        "diff_removed_bg",
        "diagram_text",
        "diagram_line",
        "diagram_backdrop",
    ];
    for name in ["terminal", "terminal-light"] {
        let path = format!(
            "{}/resources/themes/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let file: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for (key, value) in file.as_object().unwrap() {
            let values: Vec<&str> = match value {
                serde_json::Value::Array(list) => list.iter().filter_map(|v| v.as_str()).collect(),
                other => other.as_str().into_iter().collect(),
            };
            for v in values {
                let fixed = v.contains('#') || v.contains("idx:");
                assert!(
                    !fixed || grounds.contains(&key.as_str()),
                    "{name} 的 {key} 写死了：{v}"
                );
            }
        }
    }
}

#[test]
fn thoughts_are_upright_not_dimmed() {
    use ratatui::style::Modifier;
    // 读的是全局的主题：别的测试换着主题时等它换完。
    let _hold = super::hold();
    let style = super::thought();
    assert!(
        !style.add_modifier.contains(Modifier::ITALIC),
        "思考的字不斜"
    );
    assert!(
        !style.add_modifier.contains(Modifier::DIM),
        "叠了变暗在深色底上发闷"
    );
}

use super::{accent_rgb, shimmer};
use crate::config::Config;
use ratatui::style::Color;

fn rgb(style: ratatui::style::Style) -> (u8, u8, u8) {
    match style.fg {
        Some(Color::Rgb(r, g, b)) => (r, g, b),
        other => panic!("{other:?}"),
    }
}

#[test]
fn far_from_the_band_is_the_base_color_dimmed() {
    let look = Config::builtin().unwrap().layout.shimmer;
    let (r, g, b) = accent_rgb();
    // 亮光刚出发、还在最左边外面时，第 3 个字离它最远。
    let far = rgb(shimmer(3, 4, 0.0, &look));
    let dim = |c: u8| (f64::from(c) * look.dim).round() as u8;
    assert_eq!(far, (dim(r), dim(g), dim(b)));
}

#[test]
fn the_band_lights_what_it_passes() {
    let look = Config::builtin().unwrap().layout.shimmer;
    let sum = |c: (u8, u8, u8)| u32::from(c.0) + u32::from(c.1) + u32::from(c.2);
    let bright = |i: usize, t: f64| sum(rgb(shimmer(i, 4, t, &look)));
    let span = 4.0 + 2.0 * look.band;
    let at = |i: f64| (i + look.band) / span * look.sweep_seconds;
    assert!(bright(0, at(0.0)) > bright(3, at(0.0)));
    assert!(bright(3, at(3.0)) > bright(0, at(3.0)));
    let base = sum(accent_rgb());
    assert!(bright(1, at(1.0)) > base, "正中的比底色还亮");
}

#[test]
fn setting_the_same_palette_again_keeps_what_was_drawn_with_it() {
    // 2026-10-15 T-235：引导关上时主题照配置项重挑一次，颜色和引导里预览的一模一样，换代数却加了一，头像（图的缓存键带着
    // 换代数）就得重做、重传，首页那一块空了一下。同一套颜色再设一次不算换。
    let _hold = super::hold();
    let themes = super::builtin().unwrap();
    let (_, night) = themes
        .iter()
        .find(|(n, _)| n == "tokyonight-night")
        .unwrap();
    let (_, terminal) = themes.iter().find(|(n, _)| n == "terminal").unwrap();
    super::set(night.clone());
    let before = super::generation();
    super::set(night.clone());
    assert_eq!(super::generation(), before, "同一套颜色不算换");
    super::set(terminal.clone());
    assert_ne!(super::generation(), before, "换了颜色才算");
    // 换回出厂的那一套：别的测试读的是全局的主题。
    super::set(night.clone());
}

#[test]
fn a_barless_pick_uses_the_picked_color_not_the_title_accent() {
    // 2026-10-22 项目主人验收纯 ASCII 主题：选中项换强调色，和配置页标题「* 配置」一个颜色分不出，照推荐换选中色。
    let _hold = super::hold();
    let themes = super::builtin().unwrap();
    let (_, night) = themes
        .iter()
        .find(|(n, _)| n == "tokyonight-night")
        .unwrap();
    super::set(night.clone());
    let picked = super::pick(|p| p.picked);
    let accent = super::pick(|p| p.accent);
    assert_ne!(picked, accent, "这套颜色里两个本来就不一样");
    assert_eq!(
        super::bar_style(true).fg,
        Some(picked),
        "不铺底：字换选中色"
    );
    assert_eq!(super::bar_style(true).bg, None, "不铺底");
    assert_eq!(
        super::bar_style(false).bg,
        Some(accent),
        "铺底的照旧铺强调色"
    );
    // 配置页主菜单选中的那一行（截图里的「| 外观」）是这一种。
    assert_eq!(
        super::focus_style(true).fg,
        Some(picked),
        "配置页焦点那一栏"
    );
    assert_eq!(super::focus_style(true).bg, None);
    // 左边的竖条跟着字换（截图里「| 外观」的竖条也和标题一个颜色）。
    assert_eq!(super::mark_style(true).fg, Some(picked), "竖条");
    assert_eq!(
        super::mark_style(false).fg,
        Some(accent),
        "铺底的照旧强调色"
    );
}
