//! 老会话按页读（蓝图 `tui.md`「会话列表 `/sessions`」第 5 条「按页读」，核心 9-6）：更早的还有时正文最上面一行暗色的
//! 「正在读更早的…」；更早的一页读成一份临时的正文，拼在前面；订阅回应里的累计用量、权限整个换掉页里算出来的。

use std::collections::HashMap;

use super::{Entry, Kind, Slot, Transcript};
use crate::config::Texts;
use crate::core::{Level, Push, Spent};

/// 按页读读到哪了。
#[derive(Debug, Default)]
pub struct Older {
    /// 更早的还有：最上面那一行在。
    more: bool,
    /// 要了更早的一页，还没回来：不再要。
    loading: bool,
    /// 每一轮最后是撤掉（`true`）还是恢复：更早的页拼上来时照它藏起、显示回来（新的页说了算）。
    undone: HashMap<u64, bool>,
    /// 读一个会话的最新一页（或者更早的一页）、还不知道这一页开头时的权限级别（切换权限那一条多半在更早的页里）：这期间
    /// 你说的话、收尾行先不记级别，知道了再补（[`Transcript::fill`]）；画的时候竖线暗色，不照现在的（2026-10-26 项目主人报：
    /// 开放权限发的话重开 resume 以后竖线变成了工作区的颜色；2026-11-16 又报：换了级别，已经发出去的话跟着变色）。
    level_unknown: bool,
    /// 读进来的最上面那一条开头时的权限级别：读到了会话开头（`session.created`），或者订阅回应来时这一页里还没换过级别的
    /// 是它；最上面那一段还不知道的是 `None`，等更早的一页。
    top: Option<Level>,
}

impl Transcript {
    /// 最新一页读进来了，或者更早的一页拼好了：更早的还有的，最上面那一行在，不然去掉。
    pub fn paged(&mut self, more: bool, texts: &Texts) {
        self.older.more = more;
        self.older.loading = false;
        let marked = self.entries.first().is_some_and(|e| e.kind == Kind::Older);
        if more && !marked {
            let id = self.fresh_id();
            self.entries
                .insert(0, Entry::new(id, Kind::Older, texts.older.clone()));
            self.shift(1);
        } else if !more && marked {
            self.entries.remove(0);
            self.removed(0);
        }
    }

    /// 该要更早的一页了：最上面那一行在、还没要。
    pub fn wants_older(&self) -> bool {
        self.older.more && !self.older.loading
    }

    /// 要了更早的一页。
    pub fn asked_older(&mut self) {
        self.older.loading = true;
    }

    /// 核心断开了：要了的那一页不会回来了，那一行露着时再要（review B5，2026-10-11）。
    pub fn lost(&mut self) {
        self.older.loading = false;
    }

    /// 撤掉、恢复了这几轮：记下最后是哪样。
    pub(super) fn decided(&mut self, turns: &[u64], hidden: bool) {
        for &turn in turns {
            self.older.undone.insert(turn, hidden);
        }
    }

    /// 开始读会话 `session` 的最新一页：这一页开头的权限级别还不知道（[`Older::level_unknown`]）。
    pub fn opening(&mut self, session: String) {
        self.session = Some(session);
        self.older.level_unknown = true;
        self.older.top = None;
    }

    /// 权限级别变了：之后照它记。
    fn set_level(&mut self, level: Level) {
        self.level = level;
        self.older.level_unknown = false;
    }

    /// 换了级别（日志里切换权限的那一条，[`Push::Policy`]，只读开着的是只读）：带着换以前的（核心 PL-1 的 `from`）的，这一条
    /// 以前的就是它，还没记的当场补上，不等更早的一页；以前的日志没有，还没记的照旧等。
    pub(super) fn switched(&mut self, policy: &Push) {
        let Push::Policy {
            level,
            read_only,
            from,
        } = *policy
        else {
            return;
        };
        if let Some(from) = from {
            self.settled(from);
        }
        self.set_level(if read_only { Level::ReadOnly } else { level });
    }

    /// 从读进来的最上面那一条到这一刻一直是 `level`（读到了 `session.created`，或者订阅回应来时这一页里还没换过级别）：
    /// 还没记的照它补上。已经知道的（这一页里换过级别）只换现在的级别。
    pub(super) fn settled(&mut self, level: Level) {
        if self.older.level_unknown {
            self.fill(level);
            self.older.top = Some(level);
        }
        self.set_level(level);
    }

    /// 还没记级别的你说的话、收尾行，连同开始时还不知道级别、还在跑的那一轮，补成 `level`：它们都在最上面那一段，
    /// 那一段从头到尾同一个级别。
    fn fill(&mut self, level: Level) {
        for entry in &mut self.entries {
            if matches!(entry.kind, Kind::User | Kind::Done | Kind::Cut) && entry.level.is_none() {
                entry.level = Some(level);
            }
        }
        self.turn_level.get_or_insert(level);
    }

    /// 这一刻记进条目的级别：还不知道的先不记，知道了再补（[`Transcript::settled`]、[`Transcript::prepend`]）。
    pub(super) fn known(&self, level: Level) -> Option<Level> {
        (!self.older.level_unknown).then_some(level)
    }

    /// 读更早的一页用的一份空正文：会话、连接照这一份；这一页开头的权限级别同样还不知道。
    pub fn history(&self) -> Transcript {
        let mut older = Transcript {
            session: self.session.clone(),
            link: self.link.clone(),
            level: self.level,
            ..Transcript::default()
        };
        older.older.level_unknown = true;
        older
    }

    /// 更早的一页（`older`）拼上来以前，两页接头处的权限级别互相补：它里面换过级别的，它末尾的级别就是这一份开头的；
    /// 没换过的，照这一份开头的补它。都不知道的照旧等再早的一页。
    fn join_levels(&mut self, older: &mut Transcript) {
        if !older.older.level_unknown {
            let level = older.level;
            self.fill(level);
            if self.older.level_unknown {
                self.set_level(level);
            }
            self.older.top = older.older.top;
        } else if let Some(top) = self.older.top {
            older.fill(top);
        }
    }

    /// 把读成的更早的一页（[`Transcript::history`] 那一份）拼在前面：编号重发、不和已有的撞；新的页撤掉、恢复过的轮照
    /// 新的页藏起、显示回来；更早的还有没有照 `more`。交回拼进来几条（不算最上面那一行）。用量、缓存那几样不加：
    /// 订阅回应里给的是整个会话的。
    pub fn prepend(&mut self, mut older: Transcript, more: bool, texts: &Texts) -> usize {
        if self.entries.first().is_some_and(|e| e.kind == Kind::Older) {
            self.entries.remove(0);
            self.removed(0);
        }
        older.finish_segment();
        self.join_levels(&mut older);
        let base = self.next_id;
        for entry in &mut older.entries {
            entry.id += base;
        }
        self.next_id += older.next_id;
        let n = older.entries.len();
        self.shift(n);
        for (call, at) in std::mem::take(&mut older.calls) {
            self.calls.entry(call).or_insert(at);
        }
        for (&turn, &hidden) in &self.older.undone {
            older.hide(&[turn], hidden);
        }
        for (turn, hidden) in older.older.undone {
            self.older.undone.entry(turn).or_insert(hidden);
        }
        older.entries.append(&mut self.entries);
        self.entries = older.entries;
        self.paged(more, texts);
        n
    }

    /// 前面插进来 `by` 条：记着的第几条都往后挪。
    fn shift(&mut self, by: usize) {
        for slot in self.blocks.values_mut() {
            match slot {
                Slot::Reply(i) | Slot::Step(i, _) => *i += by,
            }
        }
        for (i, _) in self.unnamed.iter_mut().chain(self.calls.values_mut()) {
            *i += by;
        }
        if let Some(i) = self.compacting.as_mut() {
            *i += by;
        }
    }

    /// 订阅回应里的累计用量、权限（核心 9-6 上）、有没有能压缩的（核心 W-12）：整个换掉页里的事件算出来的，之后照推来的往上加。
    /// 没有的那一样不动。
    pub fn snapshot(
        &mut self,
        spent: Option<&Spent>,
        level: Option<Level>,
        compactable: Option<bool>,
    ) {
        if compactable.is_some() {
            self.cache.compactable = compactable;
        }
        if let Some(spent) = spent {
            self.total = spent.total;
            self.bill = spent.bill.clone();
            self.cache.compactions = spent.compactions;
            self.cache.breaks = spent.breaks;
        }
        if let Some(level) = level {
            self.settled(level);
        }
    }
}

#[cfg(test)]
mod tests;
