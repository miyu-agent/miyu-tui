//! 过程中的通知（等你确认、等你回答）图标上手里那台电脑（2026-10-19 项目主人：过程中的用玩电脑的，完成的用站着的）：照网页
//! `packages/mascot` 清单 `props.laptop` 的第一帧，16×10，看得到盖子背面、上面的记号是灯的颜色；照网页摆在身子前面，左右
//! 居中；比网页抬高 [`RAISE`] 格，盖住嘴的大半（2026-10-20 项目主人）。

use image::{Rgba, RgbaImage};

use super::MORNING;

/// 网页清单里的那一帧：`.` 是空的，别的字照下面的图例取颜色。
const ROWS: [&str; 10] = [
    ".oooooooooooooo.",
    ".oLLLLLLLLLLLLo.",
    ".oLLLLLLLLLLLLo.",
    ".oLLLLLGGLLLLLo.",
    ".oLLLLGGGGLLLLo.",
    ".oLLLLLGGLLLLLo.",
    ".oLLLLLLLLLLLLo.",
    "oooooooooooooooo",
    "oBBBBBBBBBBBBBBo",
    ".oooooooooooooo.",
];

/// 图例（网页 `props.legend` 里电脑用到的几个）。
fn rgb(ch: char) -> Option<[u8; 3]> {
    match ch {
        'o' => Some(MORNING.outline),
        'L' => Some(MORNING.ear_shade),
        'B' => Some(MORNING.ear),
        'G' => Some(MORNING.lamp),
        _ => None,
    }
}

/// 电脑底边离脚几格（2026-10-20 项目主人：「把电脑往上移动一些，然后嘴巴遮住大部分」；原来照网页贴着脚，只盖住嘴的下半截）。
pub const RAISE: u32 = 2;

/// 盖在还没放大的图上：左右居中，底边离最下面画了东西的那一行 `raise` 格。
pub fn hold(small: &mut RgbaImage, raise: u32) {
    let foot = (0..small.height())
        .rev()
        .find(|&y| (0..small.width()).any(|x| small.get_pixel(x, y).0[3] > 0))
        .unwrap_or(small.height() - 1);
    let width = ROWS[0].len() as u32;
    let left = small.width().saturating_sub(width) / 2;
    let top = (foot + 1).saturating_sub(ROWS.len() as u32 + raise);
    for (dy, row) in ROWS.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if let Some(c) = rgb(ch) {
                small.put_pixel(
                    left + dx as u32,
                    top + dy as u32,
                    Rgba([c[0], c[1], c[2], 255]),
                );
            }
        }
    }
}
