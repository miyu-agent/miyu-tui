//! 运行状态行的流光（蓝图 `tui.md`「运行状态行和排队的消息」第 1 条）：一道亮光从左往右扫过，没扫到的压暗。底色是真彩色的照
//! 字节算明暗；是终端调色板的（终端配色），启动时问到了终端 16 色的真颜色就照它算（`terminal_colors`，2026-10-14 项目主人：
//! 只加粗变暗「怪怪的，是字变颜色，不是真的流光了」），问不到的扫到的加粗、没扫到的变暗，照样跟着终端的配色走。

use ratatui::style::{Color, Modifier, Style};

use super::{accent_rgb, terminal_colors};
use crate::config::Shimmer;

/// 第 `index` 个字（一共 `len` 个）在 `t` 秒时离亮光多近：0 是远，1 是正中。
fn nearness(index: usize, len: usize, t: f64, look: &Shimmer) -> f64 {
    // 亮光走一趟要多走两个带宽，头尾都扫得干净。
    let span = len as f64 + 2.0 * look.band;
    let head = (t / look.sweep_seconds.max(0.1)).fract() * span - look.band;
    1.0 - ((index as f64 - head).abs() / look.band.max(0.1)).min(1.0)
}

/// 底色是 `base` 的流光：没扫到的乘 `dim` 压暗，扫到的往白里偏，正中偏 `lift`。
pub fn shimmer_on(base: Color, index: usize, len: usize, t: f64, look: &Shimmer) -> Style {
    sweep(base, terminal_colors::rgb(base), index, len, t, look)
}

/// `bytes` 是 `base` 的真字节，不知道的（问不到的终端调色板色）是 `None`。
fn sweep(
    base: Color,
    bytes: Option<(u8, u8, u8)>,
    index: usize,
    len: usize,
    t: f64,
    look: &Shimmer,
) -> Style {
    let near = nearness(index, len, t, look);
    let Some((r, g, b)) = bytes else {
        let style = Style::new().fg(base).add_modifier(Modifier::BOLD);
        return if near > 0.5 {
            style
        } else {
            style.add_modifier(Modifier::DIM)
        };
    };
    let scale = look.dim + (1.0 - look.dim) * near;
    let lift = look.lift * near;
    let mix = |c: u8| {
        let v = f64::from(c) * scale;
        (v + (255.0 - v) * lift).round().clamp(0.0, 255.0) as u8
    };
    Style::new()
        .fg(Color::Rgb(mix(r), mix(g), mix(b)))
        .add_modifier(Modifier::BOLD)
}

/// 流光的点和用时：底色压暗（和没扫到的字一样），不跟着流光变。
pub fn shimmer_rest_on(base: Color, look: &Shimmer) -> Style {
    let Some((r, g, b)) = terminal_colors::rgb(base) else {
        return Style::new().fg(base).add_modifier(Modifier::DIM);
    };
    let dim = |c: u8| (f64::from(c) * look.dim).round().clamp(0.0, 255.0) as u8;
    Style::new().fg(Color::Rgb(dim(r), dim(g), dim(b)))
}

/// 强调色的流光（引导的「按 Enter 开始」、压缩那一行）：不是真彩色的强调色退回 tokyonight 的前景色。
pub fn shimmer(index: usize, len: usize, t: f64, look: &Shimmer) -> Style {
    let (r, g, b) = accent_rgb();
    shimmer_on(Color::Rgb(r, g, b), index, len, t, look)
}

/// 强调色流光的点和用时。
pub fn shimmer_rest(look: &Shimmer) -> Style {
    let (r, g, b) = accent_rgb();
    shimmer_rest_on(Color::Rgb(r, g, b), look)
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier};

    use super::{shimmer_on, shimmer_rest_on, sweep};
    use crate::config::Config;

    #[test]
    fn a_palette_color_sweeps_with_bold_and_dim_instead_of_bytes() {
        // 2026-10-14 项目主人报：终端配色时运行状态行的字是写死的蓝，和底栏的模式字对不上。照模式字的颜色走，终端调色板的
        // 没法按字节算明暗，扫到的加粗、没扫到的变暗，颜色还是终端的那一个。
        let look = Config::builtin().unwrap().layout.shimmer;
        let lit = shimmer_on(Color::Blue, 0, 4, look.sweep_seconds * 0.4, &look);
        let far = shimmer_on(Color::Blue, 3, 4, 0.0, &look);
        assert_eq!((lit.fg, far.fg), (Some(Color::Blue), Some(Color::Blue)));
        assert!(far.add_modifier.contains(Modifier::DIM), "没扫到的变暗");
        let rest = shimmer_rest_on(Color::Blue, &look);
        assert_eq!(rest.fg, Some(Color::Blue));
        assert!(rest.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn an_answered_palette_color_sweeps_as_a_real_gradient() {
        // 2026-10-14 项目主人：终端配色时流光只加粗变暗「怪怪的，是字变颜色，不是真的流光了」。问到了终端蓝的真颜色，就照它
        // 算渐变：挨着的字字节不一样，都是那个蓝压暗、提亮出来的。
        let look = Config::builtin().unwrap().layout.shimmer;
        let blue = Some((0x40, 0x80, 0xff));
        let t = look.sweep_seconds * 0.4;
        let colors: Vec<_> = (0..6)
            .map(|i| sweep(Color::Blue, blue, i, 6, t, &look).fg)
            .collect();
        assert!(
            colors.windows(2).any(|w| w[0] != w[1]),
            "挨着的字颜色不一样：{colors:?}"
        );
        for color in colors {
            let Some(Color::Rgb(r, g, b)) = color else {
                panic!("照真字节算：{color:?}");
            };
            assert!(b >= g && g >= r, "还是那个蓝：{r} {g} {b}");
        }
    }
}
