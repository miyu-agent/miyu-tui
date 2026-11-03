//! 「下载软件」怎么排、怎么筛（蓝图「配置页」第 43c 条，核心设计 54）：最近照 `published`、下载量照 `downloads`、趋势照
//! `downloads_week`，都从大到小，没有这一格的排在最后、再照编号；一个包都没有某一格的那一种不给。筛是终端自己筛：几个词
//! 每个都要出现在名字、编号、说明里，不分大小写。

use std::cmp::Ordering;

use super::read::Remote;

/// 三种排序，照 `s` 轮换的先后。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// 最近：`published`。
    Recent,
    /// 下载量：`downloads`。
    Downloads,
    /// 趋势：`downloads_week`。
    Trend,
}

impl Sort {
    const ALL: [Sort; 3] = [Sort::Recent, Sort::Downloads, Sort::Trend];

    /// 第几种（字表 `sorts` 的下标）。
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// 这个包有没有这一格。
    fn has(self, p: &Remote) -> bool {
        match self {
            Sort::Recent => p.published.is_some(),
            Sort::Downloads => p.downloads.is_some(),
            Sort::Trend => p.week.is_some(),
        }
    }

    /// 两个都有这一格的比大小（大的在前）。
    fn compare(self, a: &Remote, b: &Remote) -> Ordering {
        match self {
            // 时刻照同一种写法（`2026-10-31T09:00:00Z`），照字比就是照先后比。
            Sort::Recent => b.published.cmp(&a.published),
            Sort::Downloads => b.downloads.cmp(&a.downloads),
            Sort::Trend => b.week.cmp(&a.week),
        }
    }
}

/// 这些包能用的几种排序，照轮换的先后：一个包都没有那一格的不给。
pub fn available(list: &[Remote]) -> Vec<Sort> {
    Sort::ALL
        .into_iter()
        .filter(|s| list.iter().any(|p| s.has(p)))
        .collect()
}

/// `s` 换到下一种：能用的里面轮换；`now` 不在能用的里面的从第一种起。
pub fn next(now: Option<Sort>, list: &[Remote]) -> Option<Sort> {
    let can = available(list);
    let at = now.and_then(|s| can.iter().position(|c| *c == s));
    match at {
        Some(i) => can.get((i + 1) % can.len()).copied(),
        None => can.first().copied(),
    }
}

/// 照 `sort` 排（`None` 照软件源给的先后），照 `query` 筛：交回下标。
pub fn shown(list: &[Remote], sort: Option<Sort>, query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut out: Vec<usize> = (0..list.len())
        .filter(|&i| matches(&list[i], &words))
        .collect();
    if let Some(sort) = sort {
        out.sort_by(|&i, &j| {
            let (a, b) = (&list[i], &list[j]);
            match (sort.has(a), sort.has(b)) {
                (true, true) => sort.compare(a, b).then_with(|| a.id.cmp(&b.id)),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => a.id.cmp(&b.id),
            }
        });
    }
    out
}

/// 几个词每个都出现在名字、编号、说明里。
fn matches(p: &Remote, words: &[String]) -> bool {
    let hay = format!("{}\n{}\n{}", p.name, p.id, p.summary).to_lowercase();
    words.iter().all(|w| hay.contains(w.as_str()))
}

#[cfg(test)]
mod tests;
