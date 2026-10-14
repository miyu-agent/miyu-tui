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
