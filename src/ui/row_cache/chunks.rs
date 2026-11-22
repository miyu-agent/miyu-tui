//! 这一帧的全部行（[`Rows`]）：一条一块，共享按条记着的那一份。离视口远的块可以只有长度、没有行（施工 LW-22，蓝图
//! 「正文」第 8 条）；停放着的会话每一块都只有长度（施工 LW-20）。一共几行、每一条从第几行到第几行这些几何的问法不看
//! 有没有行，照样答得出。

use std::ops::Range;
use std::rc::Rc;

use super::owner;
use crate::ui::rows::{Row, Target};

/// 这一帧的全部行：一条一块，共享记着的那一份；放掉了的块只记长度。
#[derive(Debug, Clone, Default)]
pub struct Rows {
    /// 每一块的行；`None` 是放掉了、只留长度（长度照 `ends` 算）。
    chunks: Vec<Option<Rc<[Row]>>>,
    /// 每一块结束在第几行（不含）。
    ends: Vec<usize>,
    /// 每一块是第几条的（条目间的空行、等她第一个字的那几行不是哪一条的）。
    owners: Vec<Option<usize>>,
}

impl Rows {
    /// 一共几行。
    pub fn len(&self) -> usize {
        self.ends.last().copied().unwrap_or(0)
    }

    /// 一行都没有。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 第 `i` 行；落在放掉了的块里的是 `None`。
    pub fn get(&self, i: usize) -> Option<&Row> {
        let k = self.chunk_at(i);
        self.chunks.get(k)?.as_ref()?.get(i - self.start(k))
    }

    /// 留着行的那几块里的每一行，从头到尾；放掉了的块跳过，所以不一定是全部行。只有测试、量尺拿它当全部行（整份
    /// 排出来、还没放掉过的）；画、鼠标、复制只照行号取。
    #[cfg(test)]
    pub fn present(&self) -> impl Iterator<Item = &Row> {
        self.chunks.iter().flatten().flat_map(|chunk| chunk.iter())
    }

    /// 从第 `first` 行起的 `n` 行，带着行号：只画视口里的，不从头数过去。放掉了的块里的行跳过。
    pub fn window(&self, first: usize, n: usize) -> impl Iterator<Item = (usize, &Row)> {
        (first..first.saturating_add(n).min(self.len())).filter_map(|i| Some((i, self.get(i)?)))
    }

    /// 放掉行、只留几何（施工 LW-20）：一共几行、每一条从第几行到第几行照旧答得出，[`Rows::get`]、[`Rows::window`]
    /// 交回空的，`Rows::present`（测试用）一行都没有。停放着的会话用：它的视口切回来时只照上一帧的行号换算位置，不读行里的字。
    pub fn shed(&mut self) {
        for chunk in &mut self.chunks {
            *chunk = None;
        }
    }

    pub(in crate::ui) fn push(&mut self, chunk: Rc<[Row]>) {
        self.push_owned(chunk, None);
    }

    pub(super) fn push_owned(&mut self, chunk: Rc<[Row]>, owner: Option<usize>) {
        let len = chunk.len();
        self.push_chunk(Some(chunk), len, owner);
    }

    /// 接上第 `owner` 条的一块，只有长度、没有行（记着的只剩行数的，施工 LW-22）。
    pub(super) fn push_len(&mut self, len: usize, owner: usize) {
        self.push_chunk(None, len, Some(owner));
    }

    fn push_chunk(&mut self, chunk: Option<Rc<[Row]>>, len: usize, owner: Option<usize>) {
        if len == 0 {
            return;
        }
        let end = self.len() + len;
        self.chunks.push(chunk);
        self.ends.push(end);
        self.owners.push(owner);
    }

    /// 属于条目的有几行：末尾不属于哪一条的（等第一个字时转着的那几行）不算；一行都不属于条目的照全部算。
    pub fn content_len(&self) -> usize {
        self.owners
            .iter()
            .rposition(Option::is_some)
            .map_or_else(|| self.len(), |k| self.ends[k])
    }

    /// 第 `row` 行是第几条的；落在条目间空行上的，照它下面那一条。
    pub fn entry_at(&self, row: usize) -> Option<usize> {
        let k = self.chunk_at(row);
        self.owners.get(k..)?.iter().find_map(|o| *o)
    }

    /// 第 `entry` 条的最后一行后面是第几行（不含）；没有这一条的是 `None`。
    pub fn end_of(&self, entry: usize) -> Option<usize> {
        let k = self.owners.iter().rposition(|o| *o == Some(entry))?;
        Some(self.ends[k])
    }

    /// 第 `entry` 条从第几行起。
    pub fn start_of(&self, entry: usize) -> Option<usize> {
        let k = self.owners.iter().position(|o| *o == Some(entry))?;
        Some(self.start(k))
    }

    /// 点中 `target` 的第一行是第几行（点开、收起时找锚点用）：先只在它那一条的块里找，不从头摊平全部行；行号照每一块
    /// 的起点算，放掉了的块不会让后面的行号偏。那一条里没有的（测试里造的行，目标和块不是同一条），再找留着行的每一块。
    /// 那一条的行放掉了的是 `None`，同找不到。
    pub fn row_of(&self, target: Target) -> Option<usize> {
        let entry = owner(target);
        let find = |k: usize| {
            let at = self.chunks[k]
                .as_ref()?
                .iter()
                .position(|r| r.target == Some(target))?;
            Some(self.start(k) + at)
        };
        self.chunks_of(entry)
            .find_map(find)
            .or_else(|| (0..self.chunks.len()).find_map(find))
    }

    /// 第 `entry` 条的行还留着没有：没有这一条的是 `None`（测试、量尺看放掉了没有）。
    #[cfg(test)]
    pub fn kept(&self, entry: usize) -> Option<bool> {
        let k = self.owners.iter().position(|o| *o == Some(entry))?;
        Some(self.chunks[k].is_some())
    }

    /// 第 `k` 块从第几行起。
    fn start(&self, k: usize) -> usize {
        if k == 0 { 0 } else { self.ends[k - 1] }
    }

    /// 第 `row` 行落在第几块（过了末尾的是块数）。
    fn chunk_at(&self, row: usize) -> usize {
        self.ends.partition_point(|&end| end <= row)
    }

    /// 块的个数。
    pub(super) fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// 第 `k` 块：哪一条的、占哪几行、行还在不在。
    pub(super) fn chunk(&self, k: usize) -> Chunk<'_> {
        (
            self.owners[k],
            self.start(k)..self.ends[k],
            self.chunks[k].as_ref(),
        )
    }

    /// 和 `rows` 这几行有交叠的块，从前往后：交回块的下标。
    pub(super) fn chunks_in(&self, rows: Range<usize>) -> Range<usize> {
        if rows.is_empty() {
            return 0..0;
        }
        self.chunk_at(rows.start)
            ..self
                .chunk_at(rows.end - 1)
                .saturating_add(1)
                .min(self.chunks.len())
    }

    /// 第 `entry` 条的块的下标。
    pub(super) fn chunks_of(&self, entry: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.owners.len()).filter(move |&k| self.owners[k] == Some(entry))
    }

    /// 第 `k` 块换成 `rows`（放掉了的现排出来，施工 LW-22）：后面每一块跟着挪；排出来一行都没有的，这一块去掉（整份
    /// 排的时候没有行的条目不占块）。交回长度变了没有。
    pub(super) fn replace(&mut self, k: usize, rows: Rc<[Row]>) -> bool {
        let was = self.ends[k] - self.start(k);
        let now = rows.len();
        if now == 0 {
            self.chunks.remove(k);
            self.ends.remove(k);
            self.owners.remove(k);
        } else {
            self.chunks[k] = Some(rows);
        }
        if now != was {
            // 去掉了的，第 `k` 块已经是后面那一块；没去掉的，第 `k` 块自己的末尾也跟着挪。
            for end in &mut self.ends[k..] {
                *end = *end - was + now;
            }
        }
        now != was
    }

    /// 第 `k` 块放掉行、只留长度。
    pub(super) fn drop_rows(&mut self, k: usize) {
        self.chunks[k] = None;
    }
}

/// 一块：哪一条的、占哪几行、行还在不在（放掉了的是 `None`）。
pub(super) type Chunk<'a> = (Option<usize>, Range<usize>, Option<&'a Rc<[Row]>>);

impl From<Vec<Row>> for Rows {
    fn from(rows: Vec<Row>) -> Self {
        let mut out = Self::default();
        out.push(rows.into());
        out
    }
}
