//! 辅助平面私用区的字（Nerd Font 的 md 那一族，第 15、16 平面）画到终端以前的保护（蓝图 `tui.md`「图标」第 2 条）。
//!
//! 这种字占几格，各家终端认得不一样：kitty 照一格画，有的（项目主人手机上经 ssh 用的那个）当两格。界面照一格排，
//! 终端当两格时：字后面接着写的那几格整体往右错一格，那一行以后只重写变了的格子，错开的字、半个字形就留在屏上
//! （2026-10-16 项目主人报：思考那一行往上推，时间线左边留下半个原子）。
//!
//! 画面（`screen.rs`）变了的段重写，一帧里要重写的几截都经 `spans` 照这样送：
//! 1. 这种字右边那一格和别的格子一起先写（字后面都跟着空格）：当一格画的终端那里是空格，当两格的被字的后半盖住；
//! 2. 这种字最后一个一个单独写，前面都显式挪光标：写完以后终端的光标在哪不要紧，别的格子不再接着它写；
//! 3. 这种字的格子变了（换成别的、或者换成它）、它右边那一格变了：相邻那一格跟着重写（当两格的终端里改了一半，
//!    整个字就没了；当一格画、字形宽出一截的终端里，右边那格留着半个字形）。画面那一层把截往外扩来做到这一条
//!    （`screen/spans.rs` 的第 3、4 条）。
//!
//! 原来 ratatui 只送变了的格子，这一层为了第 3 条要知道相邻那一格现在写着什么、哪几格写着这种字，得另外记着；现在要重写
//! 的截在这一帧的缓冲里，相邻那一格由画面那一层扩进截里，这一层什么都不记（2026-11-22 LW-17）。

use ratatui::backend::Backend;
use ratatui::buffer::{Cell, CellDiffOption, CellWidth};

/// kitty 图片占着的格子写的字（`U+10EEEE`，第 16 平面私用区）：一张图几百格，不归这一层管。
const PICTURE: char = '\u{10EEEE}';

/// 这一格写的是宽度认不准的字：第 15、16 平面私用区的（kitty 图片的占位字除外）。
pub fn shaky(symbol: &str) -> bool {
    symbol
        .chars()
        .next()
        .is_some_and(|c| matches!(u32::from(c), 0xF_0000..=0x10_FFFD) && c != PICTURE)
}

/// 把要重写的几截（第几行、从第几列起、这一截的格子）交给 `backend`：先写普通的格子，再写要补写后面那一格的带 VS16 的
/// emoji（见 `shown`），这种字最后一个一个写（上面第 1、2 条）。一截里照 ratatui 比对两帧时的规矩跳格子（`shown`，`trail`
/// 见那里），`keep` 说不跳的格子里哪些写（重格子没变的不写）；没有要重写的，什么都不写。每一截的第一格得是能单独写的格子
/// （不是宽字盖住的后半，画面那一层扩过）。
///
/// # Errors
/// 写终端失败。
pub fn spans<B: Backend>(
    backend: &mut B,
    spans: &[(u16, u16, &[Cell])],
    keep: impl Fn(u16, u16, &Cell) -> bool,
    trail: impl Fn(u16, u16, &Cell) -> bool,
) -> Result<(), B::Error> {
    if spans.is_empty() {
        return Ok(());
    }
    let cells = || {
        spans
            .iter()
            .flat_map(|(y, x, cells)| shown(*y, *x, cells, &trail))
            .filter(|(x, y, cell, _)| keep(*x, *y, cell))
    };
    let plain = |(_, _, cell, late): &(u16, u16, &Cell, bool)| !late && !shaky(cell.symbol());
    backend.draw(cells().filter(plain).map(|(x, y, cell, _)| (x, y, cell)))?;
    // 要补写的 emoji：先写它盖住的那几格，再写它自己，每一格都另起一次、挪过光标。
    let late: Vec<(u16, u16, &Cell)> = cells()
        .filter(|(_, _, _, late)| *late)
        .map(|(x, y, c, _)| (x, y, c))
        .collect();
    for group in late.chunk_by(|a, b| emoji(a.2) && !emoji(b.2) && a.1 == b.1) {
        for &cell in group.iter().skip(1).chain(group.first()) {
            backend.draw(std::iter::once(cell))?;
        }
    }
    for (x, y, cell, _) in cells().filter(|(_, _, c, _)| shaky(c.symbol())) {
        backend.draw(std::iter::once((x, y, cell)))?;
    }
    Ok(())
}

/// 第 `y` 行从第 `x0` 列起的这一截写哪几格，规矩和 ratatui 0.30 的 `Buffer::diff` 一样：图占着的格子（`Skip`，或者旧的
/// `skip`）不写；写了一个宽字（或者强定了宽度的）以后，它盖住的后面那几格不写。空格照写，不留旧字。
///
/// 带 VS16（`U+FE0F`）的 emoji 例外：有的终端把它当一格画、不清后面那一格，ratatui 写了它以后，紧接着把它盖住的格子里
/// 字变了的那几格也写一遍（`BufferDiff` 的 `TrailingState`，`force` 为假）。这几格写不写由 `trail` 说（活跃时是 ratatui
/// 比出来变了的，醒来那一帧不知道上一帧，一律写）。
///
/// 照 ratatui 的先后（先 emoji、紧接着不挪光标写后面那一格）在把它画成两格的终端（kitty）上会错位：写完 emoji 光标已经在
/// 两格以后，那一格落到后一格上，后面接着写的整截往右错一格（2026-11-23 在 kitty 里看到「和」「还」被擦掉）；emoji 到了行尾
/// 的，那一格还会折到下一行开头。所以要补写的 emoji 和它盖住的那几格交回时带上「另写」（第四项为真），由 `spans` 放到普通的
/// 格子后面、先写盖住的那几格、再写 emoji，每一格都挪过光标：两种终端上都落在原处，emoji 最后写，盖住的那半格归它。这是唯一
/// 故意和 ratatui 写的不一样的地方：写的格子一样，先后和挪光标不一样。
pub fn shown<'a, T: Fn(u16, u16, &Cell) -> bool>(
    y: u16,
    x0: u16,
    cells: &'a [Cell],
    trail: &'a T,
) -> impl Iterator<Item = (u16, u16, &'a Cell, bool)> + 'a {
    let mut covered = 0u16;
    let mut trailing = false;
    let mut at = 0usize;
    (x0..).zip(cells).filter_map(move |(x, cell)| {
        at += 1;
        if covered > 0 {
            covered -= 1;
            let write = trailing && !skipped(cell) && trail(x, y, cell);
            return write.then_some((x, y, cell, true));
        }
        if skipped(cell) {
            return None;
        }
        covered = cell.cell_width().saturating_sub(1);
        // 这个 emoji 盖住的格子里有要补写的：它也另写。
        let behind = cells.get(at..at + usize::from(covered)).unwrap_or_default();
        trailing = emoji(cell)
            && (1u16..)
                .zip(behind)
                .any(|(k, c)| !skipped(c) && trail(x.saturating_add(k), y, c));
        Some((x, y, cell, trailing))
    })
}

/// 这一格写着带 VS16 的 emoji（占不止一格）。
pub fn emoji(cell: &Cell) -> bool {
    cell.cell_width() > 1 && cell.symbol().contains('\u{FE0F}')
}

/// 这一格是图占着的、比对时不写（同 ratatui 的 `Buffer::diff`：新的 `Skip`，或者没设新选项时旧的 `skip`）。
#[allow(deprecated)] // `skip` 是 ratatui 0.30.1 起换成 `diff_option` 的旧字段，ratatui 比对时照旧认它。
pub fn skipped(cell: &Cell) -> bool {
    matches!(cell.diff_option, CellDiffOption::Skip)
        || (cell.skip && matches!(cell.diff_option, CellDiffOption::None))
}

#[cfg(test)]
mod emoji_tests;
#[cfg(test)]
mod tests;
