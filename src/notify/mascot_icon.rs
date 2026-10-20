//! 没有头像时通知的图标：终端的吉祥物画成的像素图（蓝图 `tui.md`「系统通知」第 3 条，2026-10-19 项目主人：先选「吉祥物画成图」，
//! 看了嫌不够像素画、配色不好看，定照网页晨光主题那一只、符号化一些）。照网页 `packages/mascot/sprite.js` 的画法：24×24 格一格
//! 一个像素，头、耳朵、鳍各两色（亮的和影子的），照那一格多亮在两个之间取四档；眼睛照原色；锯齿线和肚子上的圈是头的影子往轮廓色
//! 靠一半；外面一圈一像素的轮廓；放大时不插值。颜色固定用晨光那一套，不跟着终端的配色走。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::imageops::{FilterType, resize};
use image::{Rgba, RgbaImage};

use crate::mascot::{Look, Part};
use pixel::Px;

mod laptop;
mod pixel;

/// 哪一张：过程中的通知（等你确认、等你回答）抱着电脑，完成的（回答完了、出错了）站着（2026-10-19 项目主人）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// 站着。
    Standing,
    /// 抱着电脑。
    Working,
}

/// 画多少格见方、头的半径、头的中心在第几行、亮暗分几档（同网页的 `cols`、`rows`、`radius`、`center_row`、`levels`）。
const GRID: pixel::Grid = pixel::Grid {
    cols: 24,
    rows: 24,
    radius: 8.0,
    center_row: 13.6,
    levels: LEVELS,
};
/// 亮暗分几档（同网页模型的 `levels`）。
const LEVELS: usize = 4;
/// 一格放大成几个像素：连轮廓 26 格，放大到 130 像素。
const SCALE: u32 = 5;
/// 锯齿线、肚子上的圈往轮廓色靠多少（同网页的 `line_shade`）。
const LINE_SHADE: f64 = 0.5;

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
    lamp: [u8; 3],
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
    lamp: [0x5a, 0xa9, 0xff],
};

/// 两个颜色之间取 `k`（0 是 `a`）。
fn mix(a: [u8; 3], b: [u8; 3], k: f64) -> [u8; 3] {
    let step = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * k).round() as u8;
    [step(a[0], b[0]), step(a[1], b[1]), step(a[2], b[2])]
}

/// 一格的颜色（同网页 `sprite.js` 的 `rgbOf`）：眼睛照原色；锯齿线、肚子上的圈是头的影子往轮廓色靠一半；别的照第几档在影子
/// 和亮色之间取。
fn color(px: Px) -> [u8; 3] {
    let p = &MORNING;
    let (part, level) = match px {
        Px::Eye => return p.eye,
        Px::Line | Px::Belly => return mix(p.head_shade, p.outline, LINE_SHADE),
        Px::Body(part, level) => (part, level),
    };
    let (light, shade) = match part {
        Part::Ear => (p.ear, p.ear_shade),
        Part::Fin => (p.fin, p.fin_shade),
        _ => (p.head, p.head_shade),
    };
    mix(shade, light, level as f64 / (LEVELS - 1) as f64)
}

/// 画成像素图：照 `look` 的形状（换了吉祥物包的照包）、像素图用的那一份脸（没写的照字符画的脸），网页那一套的格子和算法
/// （`pixel.rs`），正面、睁着眼；`icon` 是抱着电脑的盖上电脑（照网页，电脑不抬，盖住嘴的下半截）。
pub fn draw(look: &Look, icon: Icon) -> RgbaImage {
    let face = look.pixel_face.as_ref().unwrap_or(&look.face);
    let cells = pixel::render(look, face, &GRID);
    let at = |x: i32, y: i32| {
        usize::try_from(y)
            .ok()
            .zip(usize::try_from(x).ok())
            .and_then(|(r, c)| cells.get(r).and_then(|row| row.get(c)).copied().flatten())
    };
    // 四边各多一格给轮廓（同网页 `sprite.js` 的 `paint`）。
    let side = GRID.cols as u32 + 2;
    let mut small = RgbaImage::new(side, side);
    for y in -1..=GRID.rows as i32 {
        for x in -1..=GRID.cols as i32 {
            let rgb = match at(x, y) {
                Some(px) => color(px),
                None if [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .iter()
                    .any(|&(nx, ny)| at(nx, ny).is_some()) =>
                {
                    MORNING.outline
                }
                None => continue,
            };
            small.put_pixel(
                (x + 1) as u32,
                (y + 1) as u32,
                Rgba([rgb[0], rgb[1], rgb[2], 255]),
            );
        }
    }
    if icon == Icon::Working {
        laptop::hold(&mut small);
    }
    resize(&small, side * SCALE, side * SCALE, FilterType::Nearest)
}

/// 画好的那一张放进 `dir`：名字照样子算，已经在的直接用；写不进去的是 `None`。
pub fn file(look: &Look, icon: Icon, dir: &Path) -> Option<PathBuf> {
    let mut h = DefaultHasher::new();
    format!("{look:?}").hash(&mut h);
    icon.hash(&mut h);
    let path = dir.join(format!("mascot-{:016x}.png", h.finish()));
    if path.is_file() {
        return Some(path);
    }
    fs::create_dir_all(dir).ok()?;
    draw(look, icon).save(&path).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests;
