//! 她的回答排好的 Markdown 行（蓝图 `tui.md`「正文」第 8 条）：同一篇因为悬停之类重排时不用再解析一遍。只留最近用过的
//! 几篇（`layout.json` 的 `markdown_cache`）：按条记着的行已经兜住了没变的那些，这里全留着就是重复的一份
//! （2026-10-01 性能体检：150 轮的会话光这份就 9.9 MB）。

use std::collections::HashMap;

use crate::markdown::MdLine;

/// 原文超过这么多字节的回答不记（压测报告 O17，施工 YC-11，蓝图「量尺」）：排好的行已经是一份，再记一份就是两份；真模型
/// 一条最多 1～1.5 MB，照记。代价是这么大的一篇悬停、卡片到了这类重排时整篇重新解析（4 MB 约零点几秒）。
pub const KEEP_UP_TO: usize = 4 * 1024 * 1024;

/// 最近用过的几篇回答排好的行。
#[derive(Debug)]
pub struct MdCache {
    /// 最多留几篇。
    keep: usize,
    /// 用一次加一，记着的每篇写上最后一次用的时候。
    clock: u64,
    /// 第几条回答 → 排好的样子。
    kept: HashMap<usize, Kept>,
}

/// 一篇排好的样子：照什么字（哈希）、多宽排的，最后一次用是什么时候。
#[derive(Debug)]
struct Kept {
    hash: u64,
    width: u16,
    lines: Vec<MdLine>,
    used: u64,
}

impl MdCache {
    /// 最多留 `keep` 篇（至少一篇：正在收的那一篇每帧都要用）。
    pub fn new(keep: usize) -> Self {
        Self {
            keep: keep.max(1),
            clock: 0,
            kept: HashMap::new(),
        }
    }

    /// 第 `index` 条回答照 `hash`、`width` 排好的行：记着的字、宽度对得上就用记着的，不然用 `render` 现排，记下来；
    /// 超过篇数的丢掉最久没用的那篇。原文（`bytes`）超过 [`KEEP_UP_TO`] 的现排、不记，记过的丢掉。
    pub fn lines(
        &mut self,
        index: usize,
        (hash, bytes): (u64, usize),
        width: u16,
        render: impl FnOnce() -> Vec<MdLine>,
    ) -> Vec<MdLine> {
        if bytes > KEEP_UP_TO {
            self.kept.remove(&index);
            return render();
        }
        self.clock += 1;
        let used = self.clock;
        if let Some(kept) = self
            .kept
            .get_mut(&index)
            .filter(|k| k.hash == hash && k.width == width)
        {
            kept.used = used;
            return kept.lines.clone();
        }
        let lines = render();
        self.kept.insert(
            index,
            Kept {
                hash,
                width,
                lines: lines.clone(),
                used,
            },
        );
        while self.kept.len() > self.keep {
            let oldest = self
                .kept
                .iter()
                .min_by_key(|(_, k)| k.used)
                .map(|(i, _)| *i);
            match oldest {
                Some(i) => self.kept.remove(&i),
                None => break,
            };
        }
        lines
    }

    /// 记着几篇。
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.kept.len()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::MdCache;

    #[test]
    fn only_the_most_recently_used_replies_are_kept() {
        let renders = Cell::new(0);
        let render = || {
            renders.set(renders.get() + 1);
            Vec::new()
        };
        let mut cache = MdCache::new(2);
        cache.lines(0, (1, 10), 80, render);
        cache.lines(1, (1, 10), 80, render);
        cache.lines(0, (1, 10), 80, render);
        assert_eq!(renders.get(), 2, "第 0 篇用记着的");
        cache.lines(2, (1, 10), 80, render);
        assert_eq!(cache.len(), 2, "只留两篇");
        cache.lines(0, (1, 10), 80, render);
        assert_eq!(renders.get(), 3, "第 0 篇最近用过，留着");
        cache.lines(1, (1, 10), 80, render);
        assert_eq!(renders.get(), 4, "第 1 篇最久没用，丢了，重排");
        cache.lines(1, (2, 10), 80, render);
        cache.lines(1, (2, 10), 60, render);
        assert_eq!(renders.get(), 6, "字、宽度变了都重排");
    }

    /// 几 MB 以上的一篇不记：每次现排，原来记着的（流式时还小）丢掉。
    #[test]
    fn a_huge_reply_is_not_kept() {
        let renders = Cell::new(0);
        let render = || {
            renders.set(renders.get() + 1);
            Vec::new()
        };
        let mut cache = MdCache::new(4);
        cache.lines(0, (1, 10), 80, render);
        assert_eq!(cache.len(), 1);
        let huge = super::KEEP_UP_TO + 1;
        cache.lines(0, (2, huge), 80, render);
        cache.lines(0, (2, huge), 80, render);
        assert_eq!(renders.get(), 3, "每次现排");
        assert_eq!(cache.len(), 0, "记着的那份丢了");
    }
}
