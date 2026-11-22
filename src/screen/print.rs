//! 一段格子的指纹（`screen.rs`）：每一格的字、前景、背景、下划线的颜色、修饰、比对时的选项都算进去。

use std::hash::{BuildHasher, Hasher};

use ratatui::buffer::{Cell, CellDiffOption};
use ratatui::style::Color;

/// 算行指纹的哈希：foldhash 的快速版，种子固定（同样的一行每帧算出来一样）。只比这一行和上一帧同一行变没变，不用防人造
/// 碰撞；标准库默认的 SipHash 一帧要喂上万次，慢得多（LiteMiyu 量过，转圈、流式那种帧里占界面线程的三分之一）。
const PRINT: foldhash::fast::FixedState = foldhash::fast::FixedState::with_seed(0);

/// 一行的指纹：每一格的字、前景、背景、下划线的颜色、修饰、比对时的选项（图占着不写的、强定宽度的）。
///
/// 一格拼成三个数喂进去（前景和背景；下划线的颜色、比对的选项；修饰、字的头四个字节和字长），字长过四个字节的再喂后面
/// 那截：不同的格喂进去的一定不同。没照 `Hash` 一样样喂：那样一格要喂七八次，转圈那种帧里慢一截（LiteMiyu 量过）。
#[allow(deprecated)] // `skip` 是 ratatui 0.30.1 起换成 `diff_option` 的旧字段，比对时照旧认它，指纹里也得带上。
pub fn fingerprint(cells: &[Cell]) -> u64 {
    let mut hasher = PRINT.build_hasher();
    for cell in cells {
        let symbol = cell.symbol().as_bytes();
        hasher.write_u64(code(cell.fg) | code(cell.bg) << 32);
        let mut head = [0u8; 4];
        let shown = symbol.len().min(head.len());
        head[..shown].copy_from_slice(&symbol[..shown]);
        let length = u64::try_from(symbol.len()).map_or(0xffff, |n| n.min(0xffff));
        hasher.write_u64(
            code(cell.underline_color)
                | option(cell.diff_option) << 25
                | u64::from(cell.skip) << 44,
        );
        hasher.write_u64(
            u64::from(cell.modifier.bits())
                | u64::from(u32::from_le_bytes(head)) << 16
                | length << 48,
        );
        if let Some(rest) = symbol.get(head.len()..).filter(|rest| !rest.is_empty()) {
            hasher.write(rest);
        }
    }
    hasher.finish()
}

/// 比对时的选项记成一个数（19 位以内）：没有 0，不写 1，总写 2，强定宽度 3 加宽度往左挪两位。
fn option(diff: CellDiffOption) -> u64 {
    match diff {
        CellDiffOption::None => 0,
        CellDiffOption::Skip => 1,
        CellDiffOption::AlwaysUpdate => 2,
        CellDiffOption::ForcedWidth(width) => 3 | u64::from(width.get()) << 2,
    }
}

/// 一种颜色记成一个数，不同的颜色不同（25 位以内）：默认 0，十六色 1–16，256 色 `0x100` 加序号，真彩色 `0x100_0000` 加 RGB。
fn code(color: Color) -> u64 {
    match color {
        Color::Reset => 0,
        Color::Black => 1,
        Color::Red => 2,
        Color::Green => 3,
        Color::Yellow => 4,
        Color::Blue => 5,
        Color::Magenta => 6,
        Color::Cyan => 7,
        Color::Gray => 8,
        Color::DarkGray => 9,
        Color::LightRed => 10,
        Color::LightGreen => 11,
        Color::LightYellow => 12,
        Color::LightBlue => 13,
        Color::LightMagenta => 14,
        Color::LightCyan => 15,
        Color::White => 16,
        Color::Indexed(index) => 0x100 | u64::from(index),
        Color::Rgb(r, g, b) => 0x100_0000 | u64::from(r) << 16 | u64::from(g) << 8 | u64::from(b),
    }
}
