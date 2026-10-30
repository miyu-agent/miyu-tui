//! 功能树两列怎么排、光标怎么走（蓝图 `tui.md`「配置页」第 38 条「功能树」，核心设计 52 第四节）：一组一组照先后，在组和组
//! 之间切成左右两列，切在两列行数最接近的地方，先后不变；窗不够宽的排成一列。`↑` `↓` 在一列里走，`←` `→` 换到另一列离得
//! 最近的那一行。配置页的预设窗、引导的自定义、引导的「看全」共用。

use super::{Row, Tree, section_rows, selectable};

/// 排好的一列或两列，每一列从上往下一行行（组和组之间空一行）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grid {
    /// 一列或两列。
    pub cols: Vec<Vec<Row>>,
}

impl Grid {
    /// 排：`two` 是窗够宽放得下两列（画的时候量出来、记下的）。
    pub fn new(tree: &Tree, two: bool) -> Self {
        let blocks: Vec<Vec<Row>> = (0..tree.sections.len())
            .map(|s| section_rows(tree, s))
            .collect();
        let at = if two { split(&blocks) } else { blocks.len() };
        let mut cols = vec![column(&blocks[..at])];
        if at < blocks.len() {
            cols.push(column(&blocks[at..]));
        }
        Self { cols }
    }

    /// 停得上的，照先后：左列从上往下，再右列。
    pub fn stops(&self, tree: &Tree) -> Vec<Row> {
        self.cols
            .iter()
            .flatten()
            .copied()
            .filter(|r| selectable(tree, *r))
            .collect()
    }

    /// 同一列往上一行停得上的；到顶了是 `None`（交给外面：回到名字、描述那几行）。
    pub fn up(&self, tree: &Tree, row: Row) -> Option<Row> {
        let (c, i) = self.find(row)?;
        self.cols[c][..i]
            .iter()
            .rev()
            .copied()
            .find(|r| selectable(tree, *r))
    }

    /// 同一列往下一行停得上的；到底了是 `None`。
    pub fn down(&self, tree: &Tree, row: Row) -> Option<Row> {
        let (c, i) = self.find(row)?;
        self.cols[c][i + 1..]
            .iter()
            .copied()
            .find(|r| selectable(tree, *r))
    }

    /// 换到另一列（`right` 往右）离得最近的那一行停得上的；没有另一列的是 `None`。
    pub fn side(&self, tree: &Tree, row: Row, right: bool) -> Option<Row> {
        let (c, i) = self.find(row)?;
        let other = if right { c + 1 } else { c.checked_sub(1)? };
        self.cols
            .get(other)?
            .iter()
            .enumerate()
            .filter(|(_, r)| selectable(tree, **r))
            .min_by_key(|(k, _)| k.abs_diff(i))
            .map(|(_, r)| *r)
    }

    /// 这一行在第几列第几行。
    pub fn find(&self, row: Row) -> Option<(usize, usize)> {
        self.cols
            .iter()
            .enumerate()
            .find_map(|(c, col)| col.iter().position(|r| *r == row).map(|i| (c, i)))
    }
}

/// 几组接成一列：组和组之间空一行。
fn column(blocks: &[Vec<Row>]) -> Vec<Row> {
    let mut out = Vec::new();
    for (k, block) in blocks.iter().enumerate() {
        if k > 0 {
            out.push(Row::Blank);
        }
        out.extend(block.iter().copied());
    }
    out
}

/// 在第几组前面切：左列至少一组，切在两列行数（连同组间的空行）最接近的地方，一样近的切在前面。
fn split(blocks: &[Vec<Row>]) -> usize {
    let lines =
        |part: &[Vec<Row>]| part.iter().map(Vec::len).sum::<usize>() + part.len().saturating_sub(1);
    (1..=blocks.len())
        .min_by_key(|&at| lines(&blocks[..at]).abs_diff(lines(&blocks[at..])))
        .unwrap_or(blocks.len())
}
