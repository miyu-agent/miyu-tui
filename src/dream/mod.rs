//! 人打的 `/dream` 正在整理（蓝图 `tui.md`「整理记忆」）：照记忆流推来的几样算第几批、合了多少、滚动区里那几行。只有
//! 人打的才有；整理完由界面收成回执。推来的是她这一刻写的、不落盘，掉了不补：滚动区只管画；回执点开列的是日志里真正
//! 存下的（[`Kept`]）。

mod sky;
#[cfg(test)]
mod tests;

pub use sky::place;

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// 记忆流推来的、和整理有关的一样（`core/memory.rs` 读）。
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// 开始合一批（`ext.memory.merging`）：这一批几条新记的、连它在内还有几条、交给模型的是哪几条。
    Merging {
        /// 这一批的编号：收尾的那一行带同一个。
        upto: u64,
        /// 这一批几条新记的。
        fresh: u64,
        /// 连它在内还有几条。
        left: u64,
        /// 交给模型的全部编号，照编号排。
        given: Vec<String>,
    },
    /// 这一批合完了（`ext.memory.merged`，连着三次没成跳过的也是它）。
    Merged {
        /// 哪一批。
        upto: u64,
    },
    /// 这一批这一次没合成（`ext.memory.merge_failed`）：核心会再来一次。
    Failed {
        /// 哪一批。
        upto: u64,
    },
    /// 她写完一条改的（`memory.dreaming` 的 `revised`）。
    Revised {
        /// 改写后的正文。
        text: String,
    },
    /// 她写完一条删的（`retired`）。
    Retired {
        /// 删的是哪一条：原文照它从 [`Change::Texts`] 取。
        id: String,
        /// 为什么删。
        why: Retire,
    },
    /// 摘要写到第几个字（`summary`）。
    Summary {
        /// 写了几个字。
        chars: u64,
        /// 写完了。
        done: bool,
    },
    /// 记忆的正文（`memory.list` 带作废的）：编号到正文。
    Texts(HashMap<String, String>),
    /// 日志里记忆模块真正存下的一样（`ext.memory.saved` 带 `replaces` 的、`ext.memory.retired`、`ext.memory.summary`）：回执点开列它。
    Landed(Landed),
}

/// 日志里记忆模块存下的一样，原样（删的还只有编号）。
#[derive(Debug, Clone, PartialEq)]
pub enum Landed {
    /// 改写后的正文。
    Revised(String),
    /// 删的哪一条、为什么。
    Retired(String, Retire),
    /// 新的摘要全文。
    Summary(String),
}

/// 回执点开列的一样：删的原文照编号取好了。
#[derive(Debug, Clone, PartialEq)]
pub enum Kept {
    /// 改写后的正文。
    Revised(String),
    /// 删的原文（正文还没取回来的先空着，回来了照编号补上）、种类、编号。
    Retired(String, Retire, String),
    /// 新的摘要全文。
    Summary(String),
}

/// 删的种类（核心的 `outdated`、`repeat`、`worthless`）：只写种类，不写原因（项目主人定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retire {
    /// 过时：被后记的取代。
    Outdated,
    /// 重复。
    Repeat,
    /// 没价值。
    Worthless,
}

impl Retire {
    /// 照核心的写法认；认不得的当没价值。
    pub fn read(word: &str) -> Self {
        match word {
            "outdated" => Self::Outdated,
            "repeat" => Self::Repeat,
            _ => Self::Worthless,
        }
    }
}

/// 滚动区的一行。正文到画的时候才照编号取：交来的编号可能比正文先到。
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// 这一批交给模型的一条，浮过去：它的正文。
    Float(String),
    /// 删了一条：原文（正文还没取回来的先空着，回来了照编号补上）、种类、编号。
    Retired(String, Retire, String),
    /// 改了一条：改写后的正文。
    Revised(String),
    /// 摘要写到几个字、写完没有。
    Summary(u64, bool),
}

/// 正在合的那一批。
#[derive(Debug, Clone)]
struct Batch {
    upto: u64,
    /// 这一批算几条（新记的；整趟都没有新记的照交进去的条数）。
    size: u64,
}

/// 整理的样子。
#[derive(Debug, Clone)]
pub struct Dream {
    /// 哪个会话打的。
    pub session: String,
    /// 打的那一刻：z、星星闪照它算。
    pub started: Instant,
    /// 一共几条：头一批的 `left`；整趟没有新记的照头一批交进去的条数。
    total: u64,
    /// 整趟都没有新记的（只重看旧的）：每批照交进去的条数算。
    by_given: bool,
    /// 合完的几批加起来几条。
    merged: u64,
    /// 第几批（从 1 数）。
    index: u64,
    /// 还剩几批（连正在合的）：照 `left` 和这一批的条数估。
    batches: u64,
    batch: Option<Batch>,
    /// 滚动区，旧的在前。
    rows: VecDeque<Line>,
    /// 还没浮过去的编号。
    floats: VecDeque<String>,
    /// 下一条什么时候浮。
    next_float: Option<Instant>,
    texts: HashMap<String, String>,
    /// 这一批开始合以后正文取回来过：没回来以前先不浮，回来了还没有的才跳过（2026-10-20 项目主人验收截图：正文还在路上，
    /// 交的编号全当没有正文跳过了，滚动区一条没浮）。
    texts_fresh: bool,
    /// 日志里真正存下的，照先后。
    kept: Vec<Kept>,
    /// 整理完了：醒来的那一刻、核心的回执。
    pub woke: Option<(Instant, String)>,
    /// 整理时人说了话，把她叫醒了：那一块不画，整理转到后台跑完，完了照样留回执（第 7 条）。
    pub awake: bool,
    /// 人叫停了（Esc、`/dream stop`）：醒来时星星照合了多少亮，不全亮（第 7 条）。
    pub stopping: bool,
}

/// 滚动区几行。
pub const ROWS: usize = 5;

/// 打的这一行是要整理：不带字的 `/dream`（`/dream stop` 是叫停，不是又一趟）。
pub fn asks_to_dream(line: &str) -> bool {
    line.trim() == "/dream"
}

/// 打的这一行是叫停整理（核心 R-22 的 `/dream stop`）。
pub fn asks_to_stop(line: &str) -> bool {
    let mut words = line.split_whitespace();
    words.next() == Some("/dream") && words.next() == Some("stop") && words.next().is_none()
}

impl Dream {
    /// 刚打 `/dream`。
    pub fn new(session: String, now: Instant) -> Self {
        Self {
            session,
            started: now,
            total: 0,
            by_given: false,
            merged: 0,
            index: 0,
            batches: 0,
            batch: None,
            rows: VecDeque::new(),
            floats: VecDeque::new(),
            next_float: None,
            texts: HashMap::new(),
            texts_fresh: false,
            kept: Vec::new(),
            woke: None,
            awake: false,
            stopping: false,
        }
    }

    /// 推来一样。
    pub fn apply(&mut self, change: Change, now: Instant) {
        match change {
            Change::Merging {
                upto,
                fresh,
                left,
                given,
            } => self.merging(upto, fresh, left, given, now),
            Change::Merged { upto } => {
                if let Some(batch) = self.batch.take_if(|b| b.upto == upto) {
                    self.merged += batch.size;
                }
                self.floats.clear();
            }
            Change::Failed { .. } => {}
            Change::Revised { text } => self.wrote(Line::Revised(text)),
            Change::Retired { id, why } => {
                let text = self.texts.get(&id).cloned().unwrap_or_default();
                self.wrote(Line::Retired(text, why, id));
            }
            // 每多 20 个字推一次：就地改同一行，不另起（第 4 条）。
            Change::Summary { chars, done } => {
                self.heard();
                match self
                    .rows
                    .iter_mut()
                    .rev()
                    .find(|l| matches!(l, Line::Summary(..)))
                {
                    Some(line) => *line = Line::Summary(chars, done),
                    None => self.push(Line::Summary(chars, done)),
                }
            }
            Change::Texts(texts) => {
                self.texts.extend(texts);
                self.texts_fresh = true;
                self.fill_retired();
            }
            // 开始合以前的（会话里先抽的那几条）不算这次整理的。
            Change::Landed(landed) if self.index > 0 => {
                let kept = match landed {
                    Landed::Revised(text) => Kept::Revised(text),
                    Landed::Retired(id, why) => {
                        Kept::Retired(self.texts.get(&id).cloned().unwrap_or_default(), why, id)
                    }
                    Landed::Summary(text) => Kept::Summary(text),
                };
                self.kept.push(kept);
            }
            Change::Landed(_) => {}
        }
    }

    fn merging(&mut self, upto: u64, fresh: u64, left: u64, given: Vec<String>, now: Instant) {
        if self.index == 0 {
            self.by_given = left == 0;
            self.total = if self.by_given {
                given.len() as u64
            } else {
                left
            };
        } else if self.by_given {
            self.total += given.len() as u64;
        }
        let size = if self.by_given {
            given.len() as u64
        } else {
            fresh
        };
        self.index += 1;
        self.batches = self.index - 1 + left.div_ceil(size.max(1)).max(1);
        self.batch = Some(Batch { upto, size });
        // 摘要那一行是上一批的：新的一批重新数。
        self.rows.retain(|l| !matches!(l, Line::Summary(..)));
        self.floats = given.into();
        self.next_float = Some(now);
        self.texts_fresh = false;
    }

    /// 她写完一条改的、删的：往上推一行。
    fn wrote(&mut self, line: Line) {
        self.heard();
        self.push(line);
    }

    /// 她写出东西了：不再浮。
    fn heard(&mut self) {
        self.floats.clear();
        self.next_float = None;
    }

    /// 正文回来了：删的那几条原文先空着的照编号补上（她写得比正文取回来快，2026-10-20 端到端测试抓到）。
    fn fill_retired(&mut self) {
        let texts = &self.texts;
        let rows = self.rows.iter_mut().filter_map(|l| match l {
            Line::Retired(text, _, id) => Some((text, id)),
            _ => None,
        });
        let kept = self.kept.iter_mut().filter_map(|k| match k {
            Kept::Retired(text, _, id) => Some((text, id)),
            _ => None,
        });
        for (text, id) in rows.chain(kept) {
            if text.is_empty()
                && let Some(found) = texts.get(id.as_str())
            {
                text.clone_from(found);
            }
        }
    }

    fn push(&mut self, line: Line) {
        self.rows.push_back(line);
        while self.rows.len() > ROWS {
            self.rows.pop_front();
        }
    }

    /// 到点浮下一条（半秒一条，放完一遍停住，第 4 条）：取不到正文的跳过。
    pub fn tick(&mut self, now: Instant, every: Duration) {
        while self.texts_fresh && self.next_float.is_some_and(|at| at <= now) {
            let Some(id) = self.floats.pop_front() else {
                self.next_float = None;
                return;
            };
            if let Some(text) = self.texts.get(&id) {
                let text = text.clone();
                self.push(Line::Float(text));
                self.next_float = Some(now + every);
            }
        }
    }

    /// 下一次要醒来浮一条的时刻。
    pub fn next_float(&self) -> Option<Instant> {
        self.next_float.filter(|_| !self.floats.is_empty())
    }

    /// 滚动区的几行，旧的在前。
    pub fn rows(&self) -> impl Iterator<Item = &Line> {
        self.rows.iter()
    }

    /// 第几批、估的一共几批；还没开始合的是 `None`。
    pub fn batch(&self) -> Option<(u64, u64)> {
        (self.index > 0).then_some((self.index, self.batches.max(self.index)))
    }

    /// 合了多少、这一批合完到多少、一共多少（第 5 条）：一批之内不装进度（她先想很久，想完一口气写完，2026-10-20 项目主人
    /// 验收时说「进度条是假的」），收了尾这一批才一起算上。
    pub fn progress(&self) -> (u64, u64, u64) {
        let size = self.batch.as_ref().map_or(0, |b| b.size);
        (self.merged, self.merged + size, self.total)
    }

    /// 正在看 `viewing` 这个会话时画不画这一块：叫醒了的不画；首页打的、会话编号还没到的照画（第 1、7 条）。
    pub fn shown_in(&self, viewing: Option<&str>) -> bool {
        !self.awake && (self.session.is_empty() || viewing == Some(self.session.as_str()))
    }

    /// 日志里真正存下的，照先后：回执点开列它。
    pub fn kept(&self) -> &[Kept] {
        &self.kept
    }
}
