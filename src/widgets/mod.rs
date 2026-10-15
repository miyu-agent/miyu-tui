//! 界面块（蓝图 `tui.md`「界面块」，核心 `widgets.md`，施工 X-15）：扩展包放在首页、侧边栏、底栏、整页上的东西。包只说
//! 放什么，终端自己画。这里是读进来的样子和记着的那一份：订阅的回应整份换，`widget.changed` 一块一块换。

mod read;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use serde_json::{Value, json};

pub use read::{block, blocks};

/// 放在哪。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// 空会话的首页，工作目录下面。
    Home,
    /// 宽屏的侧边栏，自带的几段后面。
    Sidebar,
    /// 框下面那一行的右边，一小段字。
    Status,
    /// 整屏一页。
    Page,
}

impl Place {
    /// 协议里的写法。
    pub fn word(self) -> &'static str {
        match self {
            Place::Home => "home",
            Place::Sidebar => "sidebar",
            Place::Status => "status",
            Place::Page => "page",
        }
    }
}

/// 一块。
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// 哪个包的。
    pub package: String,
    /// 包里的编号。
    pub id: String,
    pub place: Place,
    /// 这一刻生效的标题：程序换过的照它，没换的照清单的。
    pub title: Option<String>,
    /// 包的名字（照连接的语言）：让人认得出是谁放的。
    pub source: String,
    /// 包的图标：Lucide 的图标名。
    pub icon: Option<String>,
    /// 程序在不在跑。
    pub running: bool,
    /// 内容：程序还没推、清掉了、不在屏上的是 `None`。
    pub content: Option<Vec<Segment>>,
}

impl Block {
    /// 是不是 `package` 的 `id` 那一块。
    pub fn is(&self, package: &str, id: &str) -> bool {
        self.package == package && self.id == id
    }

    /// 标题那一行写的：标题后面接软件名；标题和软件名一样、没写标题的只写软件名（「界面块」第 3 条）。
    pub fn heading(&self) -> (Option<&str>, &str) {
        let title = self
            .title
            .as_deref()
            .filter(|t| !t.is_empty() && *t != self.source);
        (title, &self.source)
    }
}

/// 一段。
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Heading(String),
    Text {
        text: String,
        tone: Tone,
    },
    Pairs(Vec<Pair>),
    /// 进度：`value`、`mark` 都在 0 到 1 之间。
    Progress {
        value: f64,
        text: Option<String>,
        mark: Option<f64>,
    },
    List(Vec<Item>),
    Buttons(Vec<Button>),
}

/// `pairs` 的一对。
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub label: String,
    pub value: String,
    pub tone: Tone,
}

/// `list` 的一条。
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub title: String,
    pub detail: Option<String>,
    pub right: Option<String>,
    pub state: Option<State>,
    pub action: Option<Action>,
}

/// 一个按钮。
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub label: String,
    pub action: Action,
}

/// 点了做什么。
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// 调这个包的程序登记的方法（`package.call`）。
    Call { method: String, params: Value },
    /// 打开同一个包的一整页。
    Open(String),
    /// 把这段字写进剪贴板。
    Copy(String),
}

/// 语气。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Normal,
    Dim,
    Accent,
    Good,
    Warn,
    Bad,
}

/// 列表一条的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Todo,
    Doing,
    Done,
    Failed,
    Off,
}

/// 开着的那一整页。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRef {
    pub package: String,
    pub widget: String,
}

impl PageRef {
    /// 读 `{"package", "widget"}`（`command.catalog` 的 `opens`、`command.run` 回的 `opens`）。
    pub fn read(value: &Value) -> Option<PageRef> {
        Some(PageRef {
            package: value["package"].as_str()?.to_string(),
            widget: value["widget"].as_str()?.to_string(),
        })
    }
}

/// 开着的整页（蓝图「界面块」第 2、7 条）：哪一页、往下滚了几行、键盘选着第几个能点的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageView {
    pub target: PageRef,
    pub scroll: usize,
    pub selected: usize,
}

/// 订阅时说的：正在看的会话、屏上这一刻有的位置、开着的那一整页（「界面块」第 8 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub session: Option<String>,
    pub showing: Vec<Place>,
    pub page: Option<PageRef>,
}

impl View {
    /// `subscribe` 的参数。
    pub fn params(&self) -> Value {
        let mut params = json!({
            "stream": "widgets",
            "showing": self.showing.iter().map(|p| p.word()).collect::<Vec<_>>(),
        });
        if let Some(session) = &self.session {
            params["session"] = json!(session);
        }
        if let Some(page) = &self.page {
            params["page"] = json!({"package": page.package, "widget": page.widget});
        }
        params
    }
}

/// 记着的那一份。
#[derive(Debug, Default)]
pub struct Widgets {
    /// 装着、开着的扩展声明的全部块，照核心给的先后。
    pub blocks: Vec<Block>,
    /// 上一次订阅时说的：一样的不再订阅。
    pub sent: Option<View>,
    /// 点开了全部的块（包、编号）：再点收起（「界面块」第 5 条）。
    pub expanded: BTreeSet<(String, String)>,
}

impl Widgets {
    /// 订阅的回应：整份换。
    pub fn listed(&mut self, blocks: Vec<Block>) {
        self.blocks = blocks;
    }

    /// 一块变了：换掉；`None` 是关掉、卸掉了；以前没有的接在同一个包的后面，没有同一个包的接在最后。
    pub fn changed(&mut self, package: &str, id: &str, block: Option<Block>) {
        let at = self.blocks.iter().position(|b| b.is(package, id));
        match (at, block) {
            (Some(at), Some(block)) => self.blocks[at] = block,
            (Some(at), None) => {
                self.blocks.remove(at);
            }
            (None, Some(block)) => {
                let after = self.blocks.iter().rposition(|b| b.package == package);
                let at = after.map_or(self.blocks.len(), |a| a + 1);
                self.blocks.insert(at, block);
            }
            (None, None) => {}
        }
    }

    /// `place` 那里有内容的块，照先后。
    pub fn shown(&self, place: Place) -> impl Iterator<Item = &Block> {
        self.blocks
            .iter()
            .filter(move |b| b.place == place && b.content.is_some())
    }

    /// 找一块。
    pub fn find(&self, package: &str, id: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.is(package, id))
    }

    /// 这一块点开了全部。
    pub fn is_expanded(&self, block: &Block) -> bool {
        self.expanded
            .contains(&(block.package.clone(), block.id.clone()))
    }

    /// 点开全部、再点收起。
    pub fn toggle(&mut self, package: &str, id: &str) {
        let key = (package.to_string(), id.to_string());
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
        }
    }
}
