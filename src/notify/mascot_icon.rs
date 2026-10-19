//! 没有头像时通知的图标：终端的吉祥物画成的图（蓝图 `tui.md`「系统通知」第 3 条，2026-10-19 项目主人选「吉祥物画成图」）。
//! 照首页画吉祥物的那一套逐点打光线（`mascot::render`），一个像素当一格、格子是方的：打中的照那一块的颜色、照亮度从暗到亮
//! 涂，眼睛照眼睛的颜色涂满，没打中的透明。同一个样子、同一套颜色只画一次，存进缓存目录。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};

use crate::mascot::{Look, Part, Pose};

/// 画多宽（像素）。
const SIDE: u32 = 128;
/// 最暗的地方还有原色的几成：照字的疏密涂，最稀的点也看得出是哪一块。
const FLOOR: f64 = 0.45;

/// 四块的颜色：头、耳朵、眼睛、鳍（照 [`Part`] 的先后）。
pub type Colors = [(u8, u8, u8); 4];

/// 这一块用哪个颜色。
fn color_of(colors: &Colors, part: Part) -> (u8, u8, u8) {
    match part {
        Part::Head => colors[0],
        Part::Ear => colors[1],
        Part::Eye => colors[2],
        Part::Fin => colors[3],
    }
}

/// 画成图：`look` 照比例放大到 [`SIDE`] 像素宽，格子当成方的（`cell_aspect` 是 1），正面、睁着眼。
pub fn draw(look: &Look, colors: &Colors) -> RgbaImage {
    let k = f64::from(SIDE) / f64::from(look.cols.max(1));
    let tall = f64::from(look.rows) * look.cell_aspect * k;
    let square = Look {
        cols: SIDE as u16,
        rows: tall.round().clamp(1.0, f64::from(u16::MAX)) as u16,
        radius: look.radius * k,
        center_row: look.center_row * look.cell_aspect * k,
        cell_aspect: 1.0,
        ..look.clone()
    };
    let ramp: Vec<char> = square.ramp.chars().collect();
    let top = ramp.len().saturating_sub(1).max(1) as f64;
    let cells = crate::mascot::render(&square, &Pose::facing(0.0, 0.0));
    let mut out = RgbaImage::new(u32::from(square.cols), u32::from(square.rows));
    for (y, row) in cells.into_iter().enumerate() {
        for (x, cell) in row.into_iter().enumerate() {
            let Some(cell) = cell else { continue };
            let (r, g, b) = color_of(colors, cell.part);
            let lum = if cell.part == Part::Eye {
                1.0
            } else {
                let level = ramp.iter().position(|c| *c == cell.mark).unwrap_or(0) as f64;
                FLOOR + (1.0 - FLOOR) * (level / top)
            };
            let shade = |v: u8| (f64::from(v) * lum).round().clamp(0.0, 255.0) as u8;
            if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
                out.put_pixel(x, y, Rgba([shade(r), shade(g), shade(b), 255]));
            }
        }
    }
    out
}

/// 画好的那一张放进 `dir`：名字照样子和颜色算，已经在的直接用；写不进去的是 `None`。
pub fn file(look: &Look, colors: &Colors, dir: &Path) -> Option<PathBuf> {
    let mut h = DefaultHasher::new();
    format!("{look:?}").hash(&mut h);
    colors.hash(&mut h);
    let path = dir.join(format!("mascot-{:016x}.png", h.finish()));
    if path.is_file() {
        return Some(path);
    }
    fs::create_dir_all(dir).ok()?;
    draw(look, colors).save(&path).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::{Colors, draw, file};
    use crate::config::Config;

    const COLORS: Colors = [(200, 200, 200), (100, 100, 100), (255, 0, 0), (0, 0, 255)];

    #[test]
    fn the_mascot_is_drawn_square_with_its_parts_colored_and_a_clear_background() {
        // 2026-10-19 项目主人：没有头像的通知用终端的吉祥物画成的图。
        let look = Config::builtin().unwrap().mascot;
        let picture = draw(&look, &COLORS);
        assert_eq!(picture.width(), 128);
        let tall = f64::from(look.rows) * look.cell_aspect * 128.0 / f64::from(look.cols);
        assert_eq!(
            f64::from(picture.height()),
            tall.round(),
            "格子当方的，比例照终端里看到的"
        );
        let pixels: Vec<_> = picture.pixels().collect();
        assert_eq!(pixels[0].0[3], 0, "角上没打中：透明");
        assert!(
            pixels.iter().any(|p| p.0 == [255, 0, 0, 255]),
            "眼睛照眼睛的颜色涂满"
        );
        assert!(
            pixels
                .iter()
                .any(|p| p.0[3] == 255 && p.0[2] > 0 && p.0[0] == 0),
            "鳍是鳍的颜色"
        );
        let head = pixels
            .iter()
            .filter(|p| p.0[3] == 255 && p.0[0] == p.0[1] && p.0[1] == p.0[2] && p.0[0] > 150)
            .count();
        assert!(head > 1000, "头是头的颜色、占一大块：{head}");
    }

    #[test]
    fn one_look_and_color_set_is_drawn_once() {
        let look = Config::builtin().unwrap().mascot;
        let dir = std::env::temp_dir().join(format!("miyu-mascot-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let first = file(&look, &COLORS, &dir).expect("画得出来");
        let stamp = std::fs::metadata(&first).unwrap().modified().unwrap();
        assert_eq!(file(&look, &COLORS, &dir), Some(first.clone()));
        assert_eq!(
            std::fs::metadata(&first).unwrap().modified().unwrap(),
            stamp,
            "不重画"
        );
        let other = file(&look, &[(1, 2, 3); 4], &dir).expect("换了颜色");
        assert_ne!(other, first, "换了颜色另画一张");
        drop(std::fs::remove_dir_all(&dir));
    }
}
