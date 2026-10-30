//! 「软件包」分三页：下载软件、已安装、软件后台（蓝图「配置页」第 43c 条，2026-10-30 项目主人照 shorin-pac 的样子提，
//! 2026-10-31 看了样板定）。这里是三页的状态：读来的列表、光标、筛的字、排序、勾选、正在问的和正在做的；按键在
//! `keys.rs`，先问再做的流程在 `flow.rs`，回应在 `answer.rs`，画在 `ui/settings/store/`。

mod answer;
pub mod flow;
mod keys;
pub mod read;
pub mod sort;
pub mod texts;

use std::collections::{BTreeSet, HashMap, HashSet};

pub use answer::Ask;
pub use flow::Flow;
pub use read::{Info, Local, Remote};
pub use sort::Sort;
pub use texts::StoreTexts;

/// 三页，从左到右。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// 下载软件：软件源里的全部。
    #[default]
    Source,
    /// 已安装：装着的全部。
    Installed,
    /// 软件后台：带后台页的。
    Backstage,
}

impl Tab {
    /// 三页的先后。
    pub const ALL: [Tab; 3] = [Tab::Source, Tab::Installed, Tab::Backstage];

    /// 第几页。
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

/// 软件源读到哪了。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Listing {
    /// 还没要。
    #[default]
    Unasked,
    /// 要了、没回来。
    Loading,
    /// 拒了：拒绝的原话。
    Failed(String),
    /// 读到了。
    Got(Vec<Remote>),
}

/// 三页的状态。
#[derive(Debug, Default)]
pub struct Store {
    /// 在哪一页。
    pub tab: Tab,
    /// 三页各自的光标（列表里第几行）。
    pub at: [usize; 3],
    /// 三页各自从第几行露起（画的时候照光标挪）。
    pub top: [usize; 3],
    /// 「下载软件」筛的字。
    pub query: String,
    /// 正在打筛的字。
    pub typing: bool,
    /// 「下载软件」怎么排；`None` 照软件源给的先后。
    pub sort: Option<Sort>,
    /// 「下载软件」勾了的编号。
    pub picked: BTreeSet<String>,
    /// 软件源里的（`package.search`）。
    pub source: Listing,
    /// 装着的（`package.list`）；还没读到的是 `None`。
    pub local: Option<Vec<Local>>,
    /// 能升级的：编号到新版本（`package.upgrades`）。
    pub upgrades: HashMap<String, String>,
    /// 要来的详情：（是不是软件源里的，编号）。
    pub infos: HashMap<(bool, String), Info>,
    /// 要过详情的，不重复要。
    pub asked: HashSet<(bool, String)>,
    /// 正在问、正在做的。
    pub flow: Option<Flow>,
    /// 做完重读以后光标停回去的那个包：（哪一页，编号）。
    pub keep: Option<(Tab, String)>,
    /// 要 App 在后台打开后台页的包：（编号，名字），App 拿走一次（`backstage.rs`）。
    pub web: Option<(String, String)>,
}

/// 列表里的一行指的包：哪一页、第几个。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item<'a> {
    /// 软件源里的。
    Remote(&'a Remote),
    /// 装着的。
    Local(&'a Local),
}

impl Item<'_> {
    /// 编号。
    pub fn id(&self) -> &str {
        match self {
            Item::Remote(p) => &p.id,
            Item::Local(p) => &p.id,
        }
    }

    /// 名字。
    pub fn name(&self) -> &str {
        match self {
            Item::Remote(p) => &p.name,
            Item::Local(p) => &p.name,
        }
    }
}

impl Store {
    /// 这一页列出来的，照列表的先后。
    pub fn rows(&self) -> Vec<Item<'_>> {
        self.rows_of(self.tab)
    }

    /// 某一页列出来的。
    pub fn rows_of(&self, tab: Tab) -> Vec<Item<'_>> {
        match tab {
            Tab::Source => match &self.source {
                Listing::Got(list) => sort::shown(list, self.sort, &self.query)
                    .into_iter()
                    .map(|i| Item::Remote(&list[i]))
                    .collect(),
                _ => Vec::new(),
            },
            Tab::Installed => self.locals().map(Item::Local).collect(),
            Tab::Backstage => self.locals().filter(|p| p.page).map(Item::Local).collect(),
        }
    }

    fn locals(&self) -> impl Iterator<Item = &Local> {
        self.local.iter().flatten()
    }

    /// 这一页的光标，挪进列表里。
    pub fn cursor(&self) -> usize {
        self.at[self.tab.index()].min(self.rows().len().saturating_sub(1))
    }

    /// 光标停的那一个。
    pub fn current(&self) -> Option<Item<'_>> {
        self.rows().get(self.cursor()).copied()
    }

    /// 光标挪到第 `n` 行（挪不出列表）。
    pub fn go(&mut self, n: usize) {
        let last = self.rows().len().saturating_sub(1);
        self.at[self.tab.index()] = n.min(last);
    }

    /// 换页：`ahead` 往右；到头停、不绕回去（同「供应商和模型」）。
    pub fn turn(&mut self, ahead: bool) {
        let i = self.tab.index();
        let to = if ahead { i + 1 } else { i.wrapping_sub(1) };
        if let Some(tab) = Tab::ALL.get(to) {
            self.tab = *tab;
            self.typing = false;
        }
    }

    /// 装着的这一个（编号）。
    pub fn local(&self, id: &str) -> Option<&Local> {
        self.locals().find(|p| p.id == id)
    }

    /// 软件源里的这一个（编号）。
    pub fn remote(&self, id: &str) -> Option<&Remote> {
        match &self.source {
            Listing::Got(list) => list.iter().find(|p| p.id == id),
            _ => None,
        }
    }

    /// 光标停的那一个要不要去要详情：交回（是不是软件源里的，编号），要过的不再要。
    pub fn wants_info(&mut self) -> Option<(bool, String)> {
        if self.tab == Tab::Backstage {
            return None;
        }
        let key = (self.tab == Tab::Source, self.current()?.id().to_string());
        self.asked.insert(key.clone()).then_some(key)
    }

    /// 读到了新的列表：光标停回做完之前的那个包（它没了的停在原来那一格）。
    pub fn settle(&mut self) {
        let Some((tab, id)) = self.keep.clone() else {
            return;
        };
        if let Some(n) = self.rows_of(tab).iter().position(|r| r.id() == id) {
            self.at[tab.index()] = n;
            self.keep = None;
        }
    }

    /// 读到了新的软件源：排序换成能用的第一种（还在用的那一种能用就留着）。
    pub fn source_read(&mut self, list: Vec<Remote>) {
        let can = sort::available(&list);
        if !self.sort.is_some_and(|s| can.contains(&s)) {
            self.sort = can.first().copied();
        }
        self.source = Listing::Got(list);
        self.settle();
    }
}

#[cfg(test)]
mod tests;
