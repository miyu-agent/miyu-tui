//! 没有头像时通知的图标：终端的吉祥物画成的像素图（蓝图 `tui.md`「系统通知」第 3 条，2026-10-19 项目主人：先选「吉祥物画成图」，
//! 看了嫌不够像素画、配色不好看，定照网页晨光主题那一只、符号化一些）。照网页 `packages/mascot/sprite.js` 的画法：24×24 格一格
//! 一个像素，头、耳朵、鳍各两色（亮的和影子的），照那一格多亮在两个之间取四档；眼睛照原色；锯齿线和肚子上的圈是头的影子往轮廓色
//! 靠一半；外面一圈一像素的轮廓；放大时不插值。颜色固定用晨光那一套，不跟着终端的配色走。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::imageops::{FilterType, resize};
use image::{Rgba, RgbaImage};

use crate::mascot::{Look, Part, Pose};

/// 画多少格见方（同网页的 `cols`、`rows`）、头的半径、头的中心在第几行。
const GRID: u16 = 24;
const RADIUS: f64 = 8.0;
const CENTER_ROW: f64 = 13.6;
/// 亮暗分几档（同网页模型的 `levels`）。
const LEVELS: usize = 4;
/// 一格放大成几个像素：连轮廓 26 格，放大到 130 像素。
const SCALE: u32 = 5;
/// 锯齿线、肚子上的圈往轮廓色靠多少（同网页的 `line_shade`）。
const LINE_SHADE: f64 = 0.5;
/// 锯齿线画成的字：换成亮度那一串里没有的，好和最暗的那一档分开。
const LINE_MARK: char = '\u{1}';

/// 晨光那一套（网页 `theme-default` 的 `light_t_mascot_*`）。
struct Palette {
    head: [u8; 3],
    head_shade: [u8; 3],
    ear: [u8; 3],
    ear_shade: [u8; 3],
    fin: [u8; 3],
    fin_shade: [u8; 3],
    eye: [u8; 3],
    outline: [u8; 3],
}

const MORNING: Palette = Palette {
    head: [0xfb, 0xf6, 0xef],
    head_shade: [0xc9, 0xc3, 0xd9],
    ear: [0xaa, 0xb2, 0xd6],
    ear_shade: [0x73, 0x7b, 0xa3],
    fin: [0xf4, 0x9a, 0xa8],
    fin_shade: [0xc8, 0x64, 0x78],
    eye: [0x3a, 0x63, 0xc9],
    outline: [0x4a, 0x42, 0x60],
};

/// 两个颜色之间取 `k`（0 是 `a`）。
fn mix(a: [u8; 3], b: [u8; 3], k: f64) -> [u8; 3] {
    let step = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * k).round() as u8;
    [step(a[0], b[0]), step(a[1], b[1]), step(a[2], b[2])]
}

/// 一格的颜色：眼睛照原色；锯齿线、肚子上的圈是头的影子往轮廓色靠一半；别的照第几档在影子和亮色之间取。
fn color(part: Part, mark: char, level: usize) -> [u8; 3] {
    let p = &MORNING;
    if part == Part::Eye {
        return p.eye;
    }
    if mark == LINE_MARK {
        return mix(p.head_shade, p.outline, LINE_SHADE);
    }
    let (light, shade) = match part {
        Part::Ear => (p.ear, p.ear_shade),
        Part::Fin => (p.fin, p.fin_shade),
        _ => (p.head, p.head_shade),
    };
    mix(shade, light, level as f64 / (LEVELS - 1) as f64)
}

/// 画成像素图：照 `look` 的形状（换了吉祥物包的照包），网页那一套的格子大小，正面、睁着眼。
pub fn draw(look: &Look) -> RgbaImage {
    let mut grid = Look {
        cols: GRID,
        rows: GRID,
        radius: RADIUS,
        center_row: CENTER_ROW,
        cell_aspect: 1.0,
        ..look.clone()
    };
    grid.face.line_mark = LINE_MARK;
    let ramp: Vec<char> = grid.ramp.chars().collect();
    let top = ramp.len().saturating_sub(1).max(1) as f64;
    let cells = crate::mascot::render(&grid, &Pose::facing(0.0, 0.0));
    let solid = |x: i32, y: i32| {
        usize::try_from(y)
            .ok()
            .and_then(|y| cells.get(y))
            .zip(usize::try_from(x).ok())
            .and_then(|(row, x)| row.get(x))
            .is_some_and(Option::is_some)
    };
    // 四边各多一格给轮廓。
    let side = u32::from(GRID) + 2;
    let mut small = RgbaImage::new(side, side);
    for y in -1..=i32::from(GRID) {
        for x in -1..=i32::from(GRID) {
            let at = (
                u32::try_from(x + 1).unwrap_or(0),
                u32::try_from(y + 1).unwrap_or(0),
            );
            let cell = usize::try_from(y)
                .ok()
                .zip(usize::try_from(x).ok())
                .and_then(|(r, c)| cells.get(r).and_then(|row| row.get(c)).copied().flatten());
            let rgb = match cell {
                Some(cell) => {
                    let found = ramp.iter().position(|c| *c == cell.mark).unwrap_or(0) as f64;
                    let level = ((found / top) * (LEVELS - 1) as f64).round() as usize;
                    color(cell.part, cell.mark, level)
                }
                None if solid(x - 1, y)
                    || solid(x + 1, y)
                    || solid(x, y - 1)
                    || solid(x, y + 1) =>
                {
                    MORNING.outline
                }
                None => continue,
            };
            small.put_pixel(at.0, at.1, Rgba([rgb[0], rgb[1], rgb[2], 255]));
        }
    }
    resize(&small, side * SCALE, side * SCALE, FilterType::Nearest)
}

/// 画好的那一张放进 `dir`：名字照样子算，已经在的直接用；写不进去的是 `None`。
pub fn file(look: &Look, dir: &Path) -> Option<PathBuf> {
    let mut h = DefaultHasher::new();
    format!("{look:?}").hash(&mut h);
    let path = dir.join(format!("mascot-{:016x}.png", h.finish()));
    if path.is_file() {
        return Some(path);
    }
    fs::create_dir_all(dir).ok()?;
    draw(look).save(&path).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::{MORNING, SCALE, draw, file};
    use crate::config::Config;

    #[test]
    fn the_mascot_is_pixel_art_in_the_morning_colors_with_an_outline() {
        // 2026-10-19 项目主人：照网页晨光主题那一只、符号化一些。
        let look = Config::builtin().unwrap().mascot;
        let picture = draw(&look);
        assert_eq!(picture.width(), 26 * SCALE, "24 格连轮廓，放大不插值");
        let has = |rgb: [u8; 3]| {
            picture
                .pixels()
                .any(|p| p.0 == [rgb[0], rgb[1], rgb[2], 255])
        };
        assert!(has(MORNING.outline), "一圈轮廓");
        assert!(has(MORNING.eye), "眼睛照原色");
        assert!(has(MORNING.head), "头最亮的那一档就是晨光的奶白");
        assert!(has(MORNING.fin) || has(MORNING.fin_shade), "鳍是粉的");
        assert_eq!(picture.get_pixel(0, 0).0[3], 0, "角上透明");
        // 放大不插值：每个格子里的像素颜色都一样。
        let colors: std::collections::BTreeSet<[u8; 4]> = picture.pixels().map(|p| p.0).collect();
        assert!(colors.len() <= 20, "只有几种颜色，像素画：{}", colors.len());
        for y in (0..picture.height()).step_by(SCALE as usize) {
            for x in (0..picture.width()).step_by(SCALE as usize) {
                let first = picture.get_pixel(x, y);
                assert_eq!(picture.get_pixel(x + SCALE - 1, y + SCALE - 1), first);
            }
        }
    }

    #[test]
    fn one_look_is_drawn_once() {
        let look = Config::builtin().unwrap().mascot;
        let dir = std::env::temp_dir().join(format!("miyu-mascot-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let first = file(&look, &dir).expect("画得出来");
        let stamp = std::fs::metadata(&first).unwrap().modified().unwrap();
        assert_eq!(file(&look, &dir), Some(first.clone()));
        assert_eq!(
            std::fs::metadata(&first).unwrap().modified().unwrap(),
            stamp,
            "不重画"
        );
        drop(std::fs::remove_dir_all(&dir));
    }
}
