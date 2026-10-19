//! 整理时的夜空（蓝图 `tui.md`「整理记忆」第 5 条）：星星散在两行里，按打乱的顺序一颗颗亮。位置和先后照种子定，同一宽度
//! 每一帧一样，不会跳。

use crate::rng::Rng;

/// 一颗星：在第几行（0、1）、第几列、第几个亮（从 0 数）、长什么样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spot {
    /// 第几行。
    pub row: usize,
    /// 第几列。
    pub col: usize,
    /// 第几个亮。
    pub rank: usize,
    /// 亮了以后画哪一个（`layout.json` 的 `dream.glyphs` 第几个）。
    pub glyph: usize,
}

/// 在 `width` 列宽的两行里放 `count` 颗：同一行的两颗之间至少空一列，放不下的少放。`glyphs` 是有几种样子。
pub fn place(width: usize, count: usize, glyphs: usize, seed: u64) -> Vec<Spot> {
    let mut rng = Rng::new(seed);
    let mut taken = [vec![false; width], vec![false; width]];
    let mut spots = Vec::new();
    let mut tries = 0;
    while spots.len() < count && tries < count * 50 {
        tries += 1;
        let row = (rng.next() % 2) as usize;
        let col = (rng.next() % width.max(1) as u64) as usize;
        let near = |c: usize| taken[row].get(c).copied().unwrap_or(false);
        if width == 0 || near(col) || near(col + 1) || col.checked_sub(1).is_some_and(near) {
            continue;
        }
        taken[row][col] = true;
        let glyph = spots.len() % glyphs.max(1);
        spots.push(Spot {
            row,
            col,
            rank: 0,
            glyph,
        });
    }
    // 亮的先后打乱：东一颗西一颗（项目主人挑的「满天散星」）。
    let mut order: Vec<usize> = (0..spots.len()).collect();
    for i in (1..order.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    for (spot, rank) in spots.iter_mut().zip(order) {
        spot.rank = rank;
    }
    spots
}
