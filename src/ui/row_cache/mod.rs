//! 正文按条缓存排好的行（蓝图 `tui.md`「正文」第 8 条）：每一条排好的行按编号记着，这一条自己和排版的条件都没变
//! 就直接拿记着的用，只重排变了的、在进行的那几条。排好的行拼成一份 [`Rows`]，视口、鼠标、复制直接引用，
//! 不再每帧复制。

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::mem::discriminant;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::rows::{self, Ctx, Row, Target};
use crate::theme;
use crate::transcript::{Entry, Step, StepKind, ToolState};

/// 这一帧怎么排（蓝图「正文」第 8 条）：要整份重排时一帧最多花多久，先排哪一条附近的。
#[derive(Debug, Clone, Copy, Default)]
pub struct Plan {
    /// 这一帧重排最多花多久；`None` 是不限（一口气排完）。每帧至少排一条。
    pub budget: Option<Duration>,
    /// 视口附近是第几条：从它往两边排；`None` 是跟着最新，从最底下往上排。
    pub anchor: Option<usize>,
}

impl Plan {
    /// 不限时，一口气排完。
    #[cfg(test)]
    pub fn all() -> Self {
        Self::default()
    }
}

/// 记着的一条：指纹、排好的行（离视口远的放掉了，只剩行数，施工 LW-22）、一共几行、排出来带不带图、排的时候问没问过
/// 链接卡片和 mermaid 图的账（施工 LW-24）。
#[derive(Debug)]
struct Kept {
    key: u64,
    rows: Option<Rc<[Row]>>,
    len: usize,
    figures: bool,
    books: bool,
}

/// 两本账（链接卡片、mermaid 图）各被问过几次：排一条前后各看一眼，变了就是这一条用到了账。
fn asked(ctx: &Ctx) -> (u64, u64) {
    (ctx.cards.borrow().asked(), ctx.diagrams.borrow().asked())
}

/// 两本账各变过几次：用到账的条目把它算进指纹，卡片、图回来了只重排这几条。
fn books(ctx: &Ctx) -> (u64, u64) {
    (
        ctx.cards.borrow().revision(),
        ctx.diagrams.borrow().revision(),
    )
}

impl Kept {
    /// 拼进这一帧的那一块：有行的用行，放掉了的只给行数。
    fn piece(&self) -> Piece {
        match &self.rows {
            Some(rows) => Piece::Rows(rows.clone()),
            None => Piece::Len(self.len),
        }
    }
}

/// 这一帧一条拼进去的：排好的行，或者只有行数（施工 LW-22）。
enum Piece {
    Rows(Rc<[Row]>),
    Len(usize),
}

/// 记着的行：条目的编号到记着的那一条。
#[derive(Debug, Default)]
pub struct RowCache {
    entries: HashMap<u64, Kept>,
    /// 上一帧重排了几条（测试、量尺看）。
    pub rebuilt: usize,
    /// 上一帧没轮到、先用旧行（或者旧的行数）的有几条：还有的话下一帧接着排（`App::deadline`）。
    pub stale: usize,
    /// 上一帧先用旧行的是哪几条（编号）：补行（[`fill`]）现排了的不再算。
    stale_ids: HashSet<u64>,
    /// 上一帧重排的是哪几条（编号）。
    fresh: HashSet<u64>,
    /// 放掉了、还没报给归还堆那一处的行数（[`shed_far`] 攒着，[`RowCache::take_shed`] 取走）：放掉行不改正文一共几行，
    /// 归还堆照行数少没少判断时看不到这一笔。
    shed: usize,
}

impl RowCache {
    /// 上次取走以后放掉了多少行：取走以后从 0 重新攒。
    pub fn take_shed(&mut self) -> usize {
        std::mem::take(&mut self.shed)
    }

    /// 编号 `id` 的那一条上一帧重排过。
    #[cfg(test)]
    pub fn fresh(&self, id: u64) -> bool {
        self.fresh.contains(&id)
    }

    /// 记着的每一条还留着的行（量尺、测试数留着多少）。
    #[cfg(test)]
    pub fn kept_rows(&self) -> impl Iterator<Item = &[Row]> {
        self.entries.values().filter_map(|k| k.rows.as_deref())
    }

    /// 记着几条（有行的、只剩行数的都算）。
    #[cfg(test)]
    pub fn kept_len(&self) -> usize {
        self.entries.len()
    }

    /// 排第 `i` 条（编号 `entry.id`）并记下来，交回排好的行。`figures` 是这一帧开始时图做好了几张。
    fn lay(&mut self, i: usize, entry: &Entry, ctx: &Ctx, figures: u64) -> Rc<[Row]> {
        self.rebuilt += 1;
        self.fresh.insert(entry.id);
        let before = asked(ctx);
        let rows: Rc<[Row]> = rows::entry_rows(i, entry, ctx).into();
        let uses_books = asked(ctx) != before;
        let has_figures = rows
            .iter()
            .any(|r| r.figure().is_some() || r.figure_pending);
        let with = With {
            figures: has_figures.then_some(figures),
            books: uses_books.then(|| books(ctx)),
        };
        if let Some(key) = fingerprint(i, entry, ctx, with) {
            let kept = Kept {
                key,
                rows: Some(rows.clone()),
                len: rows.len(),
                figures: has_figures,
                books: uses_books,
            };
            self.entries.insert(entry.id, kept);
        }
        rows
    }
}

/// 把看得见的正文排成行，一条之间空一行；没变的条目用记着的。和整份重排（测试里的 `fresh_rows`）排出来的一模一样。
/// 要重排的多、`plan` 给了预算的，照预算排视口附近的几条，没轮到的先用旧行，记进 `stale`，下一帧接着排。记着的只剩
/// 行数的（离视口远、放掉了，[`shed_far`]）照样拼进去，只占几行、没有行；视口附近要的由 [`fill`] 当场排出来。
pub fn build(entries: &[Entry], ctx: &Ctx, cache: &RefCell<RowCache>, plan: Plan) -> Rows {
    let figures = ctx.figures.borrow().revision();
    let mut cache = cache.borrow_mut();
    cache.rebuilt = 0;
    cache.stale = 0;
    cache.stale_ids.clear();
    cache.fresh.clear();
    let blank: Rc<[Row]> = Rc::from(vec![ctx.row(ctx.blank_slot(), Vec::new())]);
    let shown: Vec<usize> = (0..entries.len())
        .filter(|&i| rows::shown(&entries[i]))
        .collect();
    // 先看哪几条能直接用记着的，别的记下来要排。
    let mut chunks: Vec<Option<Piece>> = (0..shown.len()).map(|_| None).collect();
    let mut todo = Vec::new();
    for (k, &i) in shown.iter().enumerate() {
        let entry = &entries[i];
        // 图做好了几张只算上次排出来带图的条目，两本账变过几次只算上次排的时候问过账的（没记着的照样算上，反正要排）。
        let kept = cache.entries.get(&entry.id);
        let with = With {
            figures: kept.is_none_or(|kept| kept.figures).then_some(figures),
            books: kept.is_none_or(|kept| kept.books).then(|| books(ctx)),
        };
        let key = fingerprint(i, entry, ctx, with);
        match (key, cache.entries.get(&entry.id)) {
            (Some(key), Some(kept)) if kept.key == key => chunks[k] = Some(kept.piece()),
            _ => todo.push(k),
        }
    }
    // 从视口附近往两边排：跟着最新的从最底下往上。
    let near = plan
        .anchor
        .and_then(|a| shown.iter().position(|&i| i >= a))
        .unwrap_or(shown.len().saturating_sub(1));
    todo.sort_by_key(|&k| k.abs_diff(near));
    let start = Instant::now();
    for k in todo {
        let i = shown[k];
        let entry = &entries[i];
        let over = cache.rebuilt > 0 && plan.budget.is_some_and(|b| start.elapsed() >= b);
        // 预算用完了：记着旧行（或者旧的行数）、不是在进行的，这一帧先用旧的。
        let stale = over
            .then(|| cache.entries.get(&entry.id).map(Kept::piece))
            .flatten()
            .filter(|_| fingerprint(i, entry, ctx, With::default()).is_some());
        if let Some(piece) = stale {
            cache.stale += 1;
            cache.stale_ids.insert(entry.id);
            chunks[k] = Some(piece);
            continue;
        }
        chunks[k] = Some(Piece::Rows(cache.lay(i, entry, ctx, figures)));
    }
    let mut out = Rows::default();
    for (chunk, &i) in chunks.into_iter().zip(&shown) {
        if !out.is_empty() {
            out.push(blank.clone());
        }
        match chunk {
            Some(Piece::Rows(rows)) => out.push_owned(rows, Some(i)),
            Some(Piece::Len(len)) => out.push_len(len, i),
            None => {}
        }
    }
    // 不在了的（撤销后又被删掉、排队退回的）不留着。
    let seen: HashSet<u64> = shown.iter().map(|&i| entries[i].id).collect();
    cache.entries.retain(|id, _| seen.contains(id));
    out
}

/// 只有一部分条目才算进指纹的：图做好了几张（带图的条目）、两本账变过几次（排的时候问过账的条目，施工 LW-24）。
#[derive(Debug, Clone, Copy, Default)]
struct With {
    figures: Option<u64>,
    books: Option<(u64, u64)>,
}

/// 这一条和排版条件的指纹：变了就重排。在进行的那一段（还有步骤在转圈、在走表）、正在压缩的那一行（行首在转圈）
/// 是 `None`，每帧重排。
fn fingerprint(i: usize, entry: &Entry, ctx: &Ctx, with: With) -> Option<u64> {
    let figures = with.figures;
    if let Some(segment) = &entry.segment
        && (!segment.finished || segment.steps.iter().any(Step::busy))
    {
        return None;
    }
    if entry.progress.is_some() {
        return None;
    }
    // 跑着的命令末尾在转圈（「人自己跑命令 `!`」第 4 条）：每帧重排。
    if entry.shell.as_ref().is_some_and(|s| s.ran.is_none()) && !entry.queued {
        return None;
    }
    let mut h = DefaultHasher::new();
    // 排版的条件：行里带着条目的位置（点中的东西），颜色、图标烤在行里，图占几行看图做好没有（只算带图的条目）。
    (
        i,
        ctx.width,
        ctx.indent.len(),
        theme::generation(),
        &ctx.config.icons.name,
        // 时间线的几个开关（配置页能改，「时间线」第 18 条）。
        ctx.config.timeline.shape(),
        // 语言连同是不是自动：自动的中文和手动的中文代码一样，收起那一行却一个英文一个中文。
        (ctx.config.language.code(), ctx.config.auto),
        figures,
        // `/images off` 关着时带图的也排成源码、记成不带图：开关换了每一条都重排，不然开回来图出不来（2026-10-19）。
        ctx.figures.borrow().off(),
        // 图占几行照屏高算（三分之一屏）：只改窗口高度时带图的也要重排（review C4，2026-10-11）。
        figures.map(|_| ctx.screen_rows),
        // 链接卡片、图、mermaid 图回来了：排的时候问过账的条目重排（卡片换掉那一行链接、图占的行数变了）。
        with.books,
    )
        .hash(&mut h);
    ctx.hover.filter(|t| owner(*t) == i).hash(&mut h);
    // 这一条自己：字只看长短和末尾（流式的字只往后接），别的看状态。
    discriminant(&entry.kind).hash(&mut h);
    let text = entry.text.as_bytes();
    (text.len(), &text[text.len().saturating_sub(64)..]).hash(&mut h);
    (entry.hidden, entry.queued, entry.open, entry.level).hash(&mut h);
    entry.shell.hash(&mut h);
    // 写完了才换链接卡片（蓝图「链接卡片」第 1 条）：写完那一刻字没变，也要重排。
    (ctx.writing == Some(entry.id)).hash(&mut h);
    (entry.pasted.len(), &entry.details, &entry.from).hash(&mut h);
    // 块后面那一句（「仅路径」）是核心推来那一句以后才补上的：补上了要重排。
    let noted = entry.pasted.iter().filter(|c| c.note.is_some()).count();
    noted.hash(&mut h);
    // 撤销那一行：回应和改回的文件是两个事件，文件后到（review C5，2026-10-11：原来只看有没有，「改回 N 个文件」
    // 一直不出现）。
    if let Some(undo) = &entry.undo {
        (
            undo.turns,
            &undo.said,
            undo.compactions,
            undo.clears,
            undo.jobs,
        )
            .hash(&mut h);
        for file in &undo.files {
            (&file.path, &file.outcome, file.diff.len()).hash(&mut h);
        }
    }
    entry.undo.is_some().hash(&mut h);
    // 展示的文件：换了一份、后来同一个路径又展示过（缩成一行，蓝图「展示」第 5 条）；照路径读的文件改过、没了的（核心 SW-1）
    // 卡片最后一行跟着换。
    if let Some(file) = &entry.shown {
        let found = discriminant(&crate::shown_files::find(file));
        (&file.path, &file.modified, &file.title, found).hash(&mut h);
        ctx.replaced.contains(&entry.id).hash(&mut h);
        entry.unopened.hash(&mut h);
    }
    if let Some(job) = &entry.job {
        (discriminant(&job.mark), job.detail.len()).hash(&mut h);
    }
    // 语音条：放得了没有、在不在放（「她说的语音」）。
    entry
        .voice
        .as_ref()
        .map(|v| ctx.speaker.state(v))
        .hash(&mut h);
    if let Some(segment) = &entry.segment {
        (segment.finished, segment.open, segment.steps.len()).hash(&mut h);
        for step in &segment.steps {
            (step.took, step.open).hash(&mut h);
            match &step.kind {
                StepKind::Thought { text } => text.len().hash(&mut h),
                StepKind::Tool {
                    name,
                    args,
                    state,
                    output,
                    said,
                    ..
                } => {
                    (name, args.len(), output.len(), said.is_some()).hash(&mut h);
                    discriminant(state).hash(&mut h);
                    if let ToolState::Done(status) = state {
                        discriminant(status).hash(&mut h);
                    }
                }
                StepKind::Report { note, text, .. } => {
                    (text.len(), note.detail.len()).hash(&mut h);
                    discriminant(&note.mark).hash(&mut h);
                }
            }
        }
    }
    Some(h.finish())
}

/// 点中的东西属于第几条。
pub(crate) fn owner(target: Target) -> usize {
    match target {
        Target::Segment(i) | Target::Step(i, _) | Target::Entry(i) | Target::Details(i, _) => i,
    }
}

mod chunks;
mod keep;

pub use chunks::Rows;
pub use keep::{fill, shed_far};

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
