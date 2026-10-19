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

mod laptop;

/// 哪一张：过程中的通知（等你确认、等你回答）抱着电脑，完成的（回答完了、出错了）站着（2026-10-19 项目主人）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    /// 站着。
    Standing,
    /// 抱着电脑。
    Working,
}

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

/// 画成像素图：照 `look` 的形状（换了吉祥物包的照包），网页那一套的格子大小，正面、睁着眼；`icon` 是抱着电脑的盖上电脑。
pub fn draw(look: &Look, icon: Icon) -> RgbaImage {
    let mut grid = Look {
        cols: GRID,
        rows: GRID,
        radius: RADIUS,
        center_row: CENTER_ROW,
        cell_aspect: 1.0,
        ..look.clone()
    };
    // 格子少，脸照另调的那一份（网页的像素吉祥物同一组数）。
    if let Some(face) = &look.pixel_face {
        grid.face = face.clone();
    }
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
                // 抱着电脑的不露嘴（2026-10-20 项目主人）：嘴那几格照脸画。
                Some(cell) if icon == Icon::Working && cell.mark == LINE_MARK => {
                    color(cell.part, ' ', LEVELS - 1)
                }
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
mod tests {
    use super::{Icon, MORNING, SCALE, draw, file};
    use crate::config::Config;

    #[test]
    fn the_mascot_is_pixel_art_in_the_morning_colors_with_an_outline() {
        // 2026-10-19 项目主人：照网页晨光主题那一只、符号化一些。
        let look = Config::builtin().unwrap().mascot;
        let picture = draw(&look, Icon::Standing);
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
    fn the_pixel_face_is_used_when_the_mascot_has_one() {
        // 2026-10-19 项目主人：照字符画那一份的脸画，肚子上的圈大成一个「H」，丑；像素图照 `pixel_face`。
        let look = Config::builtin().unwrap().mascot;
        assert!(look.pixel_face.is_some(), "内置的吉祥物写了像素图用的脸");
        let without = crate::mascot::Look {
            pixel_face: None,
            ..look.clone()
        };
        assert_ne!(
            draw(&look, Icon::Standing),
            draw(&without, Icon::Standing),
            "照另调的那一份画"
        );
    }

    #[test]
    fn the_eyes_are_one_column_each_and_stay_clear_of_the_mouth() {
        // 2026-10-19 项目主人：眼睛和嘴挤在一起；挑了眼睛往上、只占一列，嘴往下。
        let picture = draw(&Config::builtin().unwrap().mascot, Icon::Standing);
        let cell = |x: u32, y: u32| picture.get_pixel(x * SCALE, y * SCALE).0;
        let side = picture.width() / SCALE;
        let eye = [MORNING.eye[0], MORNING.eye[1], MORNING.eye[2], 255];
        let line = super::mix(MORNING.head_shade, MORNING.outline, super::LINE_SHADE);
        let line = [line[0], line[1], line[2], 255];
        let mut columns = std::collections::BTreeSet::new();
        for y in 0..side {
            for x in 0..side {
                if cell(x, y) != eye {
                    continue;
                }
                columns.insert(x);
                for (dx, dy) in [(0i32, 1i32), (1, 1), (-1, 1), (0, 2)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as u32) < side && (ny as u32) < side {
                        assert_ne!(cell(nx as u32, ny as u32), line, "眼睛下面隔一行才是嘴");
                    }
                }
            }
        }
        assert_eq!(columns.len(), 2, "两只眼睛各一列：{columns:?}");
    }

    #[test]
    fn a_notification_in_progress_holds_a_laptop_and_a_finished_one_does_not() {
        // 2026-10-19 项目主人：过程中的通知用玩电脑的，完成的用站着的。
        let look = Config::builtin().unwrap().mascot;
        let lamp = [MORNING.lamp[0], MORNING.lamp[1], MORNING.lamp[2], 255];
        let working = draw(&look, Icon::Working);
        let standing = draw(&look, Icon::Standing);
        assert!(working.pixels().any(|p| p.0 == lamp), "电脑盖子上的灯");
        assert!(!standing.pixels().any(|p| p.0 == lamp), "站着的没有电脑");
        assert_eq!(working.dimensions(), standing.dimensions());
        let dir = std::env::temp_dir().join(format!("miyu-mascot-laptop-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        assert_ne!(
            file(&look, Icon::Working, &dir),
            file(&look, Icon::Standing, &dir),
            "两张分开存"
        );
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn one_look_is_drawn_once() {
        let look = Config::builtin().unwrap().mascot;
        let dir = std::env::temp_dir().join(format!("miyu-mascot-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let first = file(&look, Icon::Standing, &dir).expect("画得出来");
        let stamp = std::fs::metadata(&first).unwrap().modified().unwrap();
        assert_eq!(file(&look, Icon::Standing, &dir), Some(first.clone()));
        assert_eq!(
            std::fs::metadata(&first).unwrap().modified().unwrap(),
            stamp,
            "不重画"
        );
        drop(std::fs::remove_dir_all(&dir));
    }

    /// 嘴的颜色在第几格（照没放大的格子数）。
    fn mouth_cells(icon: Icon) -> Vec<(u32, u32)> {
        let picture = draw(&Config::builtin().unwrap().mascot, icon);
        let side = picture.width() / SCALE;
        let line = super::mix(MORNING.head_shade, MORNING.outline, super::LINE_SHADE);
        let line = [line[0], line[1], line[2], 255];
        (0..side)
            .flat_map(|y| (0..side).map(move |x| (x, y)))
            .filter(|&(x, y)| picture.get_pixel(x * SCALE, y * SCALE).0 == line)
            .collect()
    }

    #[test]
    fn the_mouth_is_one_cell_thick_in_the_middle_and_the_laptop_one_shows_none() {
        // 2026-10-20 项目主人：中间嘴巴的地方不需要加厚；玩电脑的不要漏嘴。
        let mouth = mouth_cells(Icon::Standing);
        assert!(!mouth.is_empty());
        let side = u32::from(super::GRID) + 2;
        for x in [side / 2 - 1, side / 2] {
            // 肚子上的圈同一个颜色，从第 17 行起：只数上面那道锯齿线。
            let thick = mouth.iter().filter(|(cx, cy)| *cx == x && *cy < 17).count();
            assert_eq!(thick, 1, "正中第 {x} 列只一格：{mouth:?}");
        }
        assert!(mouth_cells(Icon::Working).is_empty(), "抱着电脑的不露嘴");
    }

    /// 画出来看（`ICON_OUT=<目录> cargo test dump_icons -- --ignored`）：调吉祥物的格子时用。
    #[test]
    #[ignore]
    fn dump_icons() {
        let look = Config::builtin().unwrap().mascot;
        let out = std::path::PathBuf::from(std::env::var("ICON_OUT").unwrap());
        draw(&look, Icon::Standing)
            .save(out.join("standing.png"))
            .unwrap();
        draw(&look, Icon::Working)
            .save(out.join("working.png"))
            .unwrap();
    }
}
