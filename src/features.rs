//! 预设里的功能和工具（蓝图 `tui.md`「配置页」第 38 条「功能树」、「第一次打开的引导」第 24、26 条；核心 FG-1，设计 52）：
//! 核心照 `preset.get` 的 `sections` 排好一组一组（组名、先后、语言、重名的括号都排好了），一组里一个个功能，各带归它的
//! 工具。头不翻译、不排序、不认编号。这里读成一棵树、平铺成一行行（组名一行，功能一行，它的工具一件一行；只有一件或没有
//! 工具的只有功能那一行），切一行要写的键（`features.<id>`、`tools.<name>`）也在这里算，配置页、引导共用。两列怎么排、
//! 光标怎么走在 `grid.rs`。旧核心没有 `sections` 的照 `features` 排成一组、不写组名。

mod grid;

use std::ops::{Deref, DerefMut};

use serde_json::Value;

pub use grid::Grid;

/// 一件工具。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// 工具名：写 `tools.<name>` 用，不给人看。
    pub name: String,
    /// 给人看的名字。
    pub label: String,
    /// 它自己开没开（功能关着时照核心给的是关）。
    pub on: bool,
}

/// 一个功能。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    /// 编号：写 `features.<id>` 用，不给人看。
    pub id: String,
    /// 给人看的名字。
    pub name: String,
    /// 开着。
    pub on: bool,
    /// 这台机器上装了；没装的切不了。
    pub installed: bool,
    /// 归它的工具（核心只列多于一件的）。
    pub tools: Vec<Tool>,
    /// 在第几组（[`Tree::sections`]）。
    pub section: usize,
}

/// 一组（`sections` 的一项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// 组名，核心照连接的语言给的；旧核心排成的那一组是空的，不画组名那一行。
    pub name: String,
    /// 哪一种：`group`、`package`、`extensions`、`skills`、`scripts`、`mcp`（没有东西的组照它写「还没有…」）。
    pub kind: String,
}

/// 读来的一棵：几组，和平铺的功能（照组的先后）。当成功能的列表用（`Deref`），切、改、新建照旧。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    /// 一组一组，照核心给的先后。
    pub sections: Vec<Section>,
    /// 功能，照组、组里的先后。
    pub features: Vec<Feature>,
}

impl Deref for Tree {
    type Target = Vec<Feature>;
    fn deref(&self) -> &Vec<Feature> {
        &self.features
    }
}

impl DerefMut for Tree {
    fn deref_mut(&mut self) -> &mut Vec<Feature> {
        &mut self.features
    }
}

impl Feature {
    /// 工具多于一件：下面一件一行。
    pub fn spread(&self) -> bool {
        self.installed && self.tools.len() > 1
    }
}

/// 读 `preset.get`（`preset.set` 的回应同）：照 `sections` 排；装没装照 `features` 里同一个编号的（`sections` 不写）。
/// 旧核心没有 `sections` 的照 `features` 排成一组。
pub fn read(got: &Value) -> Tree {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let missing: Vec<&str> = got["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["installed"] == false)
        .filter_map(|f| f["id"].as_str())
        .collect();
    let tool = |t: &Value, key: &str, label: &str| Tool {
        name: text(&t[key]),
        label: t[label]
            .as_str()
            .map_or_else(|| text(&t[key]), str::to_string),
        on: t["on"] == true,
    };
    let feature = |f: &Value, section: usize, tools: Vec<Tool>| Feature {
        id: text(&f["id"]),
        name: f["name"]
            .as_str()
            .map_or_else(|| text(&f["id"]), str::to_string),
        on: f["on"] == true,
        installed: f["installed"] != false
            && !missing.contains(&f["id"].as_str().unwrap_or_default()),
        tools,
        section,
    };
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    let Some(sections) = got["sections"].as_array() else {
        let features = list(&got["features"])
            .iter()
            .map(|f| {
                feature(
                    f,
                    0,
                    list(&f["tools"])
                        .iter()
                        .map(|t| tool(t, "name", "label"))
                        .collect(),
                )
            })
            .collect();
        return Tree {
            sections: vec![Section {
                name: String::new(),
                kind: String::new(),
            }],
            features,
        };
    };
    let mut tree = Tree::default();
    for (s, section) in sections.iter().enumerate() {
        tree.sections.push(Section {
            name: text(&section["name"]),
            kind: text(&section["kind"]),
        });
        for item in list(&section["items"]) {
            let tools = list(&item["tools"])
                .iter()
                .map(|t| tool(t, "id", "name"))
                .collect();
            tree.features.push(feature(&item, s, tools));
        }
    }
    // 预设写了、这台机器没装的（`features` 里 `installed: false`）核心不排进组：照旧列出来、暗着写「未安装」（「配置页」
    // 第 38 条），排在最后一段，不写组名。
    let stray: Vec<Value> = list(&got["features"])
        .into_iter()
        .filter(|f| f["installed"] == false)
        .filter(|f| {
            !tree
                .features
                .iter()
                .any(|t| Some(t.id.as_str()) == f["id"].as_str())
        })
        .collect();
    if !stray.is_empty() {
        let s = tree.sections.len();
        tree.sections.push(Section {
            name: String::new(),
            kind: String::new(),
        });
        for f in &stray {
            tree.features.push(feature(f, s, Vec::new()));
        }
    }
    tree
}

/// 平铺的一行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// 组名那一行（第几组）：空格整组切（2026-10-30 项目主人）。
    Section(usize),
    /// 一组什么都没有：写「还没有…」（第几组），停不上。
    Empty(usize),
    /// 组和组之间空的一行。
    Blank,
    /// 功能那一行（第几个功能）。
    Feature(usize),
    /// 工具那一行（第几个功能的第几件工具）。
    Tool(usize, usize),
}

/// 勾怎么写。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// 开着、工具都开着：`[*]`。
    On,
    /// 开着、关了几件工具：`[~]`。
    Some,
    /// 关着：`[ ]`。
    Off,
}

impl Mark {
    /// 写出来的样子。
    pub fn text(self) -> &'static str {
        match self {
            Mark::On => "[*]",
            Mark::Some => "[~]",
            Mark::Off => "[ ]",
        }
    }
}

/// 功能那一行的勾。
pub fn mark(feature: &Feature) -> Mark {
    match (feature.on, feature.tools.iter().all(|t| t.on)) {
        (false, _) => Mark::Off,
        (true, true) => Mark::On,
        (true, false) => Mark::Some,
    }
}

/// 一组的几行：组名（旧核心排成的那一组没有）、什么都没有的写「还没有…」、功能和它的工具。
pub fn section_rows(tree: &Tree, s: usize) -> Vec<Row> {
    let mut out = Vec::new();
    if !tree.sections[s].name.is_empty() {
        out.push(Row::Section(s));
    }
    let members: Vec<usize> = (0..tree.features.len())
        .filter(|&i| tree.features[i].section == s)
        .collect();
    if members.is_empty() {
        out.push(Row::Empty(s));
    }
    for i in members {
        out.push(Row::Feature(i));
        if tree.features[i].spread() {
            out.extend((0..tree.features[i].tools.len()).map(|j| Row::Tool(i, j)));
        }
    }
    out
}

/// 一组里装了的功能：开着几个、一共几个（组名后面写）。
pub fn tally(tree: &Tree, s: usize) -> (usize, usize) {
    let installed = tree
        .features
        .iter()
        .filter(|f| f.section == s && f.installed);
    installed.fold((0, 0), |(on, all), f| (on + usize::from(f.on), all + 1))
}

/// 光标停不停得上：组名那一行组里有装了的功能才停；没装的功能、什么都没有的、空行停不上。
pub fn selectable(features: &[Feature], row: Row) -> bool {
    match row {
        Row::Section(s) => features.iter().any(|f| f.section == s && f.installed),
        Row::Empty(_) | Row::Blank => false,
        Row::Feature(i) | Row::Tool(i, _) => features.get(i).is_some_and(|f| f.installed),
    }
}

/// 一件工具现在算不算开着：功能开着、它自己也开着。
pub fn tool_on(feature: &Feature, tool: &Tool) -> bool {
    feature.on && tool.on
}

/// 空格切一行要写的键：功能那一行切整个功能；工具切这一件。功能关着时打开这一件：功能打开，只开这一件，同一功能里别的
/// 工具都关上（2026-10-23 项目主人：「激活读取会激活整个文件读写，不合理」，原来只写功能，工具回到各自原来的开关）。
pub fn toggle(features: &[Feature], row: Row) -> Vec<(String, bool)> {
    match row {
        Row::Empty(_) | Row::Blank => Vec::new(),
        // 组名那一行整组切（2026-10-30 项目主人：「可以整组，也可以 ctrl+A 切换全选」）：同全开全关，只看这一组。
        Row::Section(s) => {
            let members: Vec<Feature> = features
                .iter()
                .filter(|f| f.section == s)
                .cloned()
                .collect();
            toggle_all(&members)
        }
        Row::Feature(i) => features
            .get(i)
            .filter(|f| f.installed)
            .map(|f| vec![(format!("features.{}", f.id), !f.on)])
            .unwrap_or_default(),
        Row::Tool(i, j) => {
            let Some(feature) = features.get(i).filter(|f| f.installed) else {
                return Vec::new();
            };
            if !feature.on {
                let mut out = vec![(format!("features.{}", feature.id), true)];
                for (k, t) in feature.tools.iter().enumerate() {
                    if k == j && !t.on {
                        out.push((format!("tools.{}", t.name), true));
                    } else if k != j && t.on {
                        out.push((format!("tools.{}", t.name), false));
                    }
                }
                return out;
            }
            feature
                .tools
                .get(j)
                .map(|t| vec![(format!("tools.{}", t.name), !t.on)])
                .unwrap_or_default()
        }
    }
}

/// `Ctrl+A`：有没开的（功能、开着的功能里的工具）就全开，都开着才全关功能（同旧版）。
pub fn toggle_all(features: &[Feature]) -> Vec<(String, bool)> {
    let installed: Vec<&Feature> = features.iter().filter(|f| f.installed).collect();
    let open = installed
        .iter()
        .any(|f| !f.on || f.tools.iter().any(|t| !t.on));
    let mut out = Vec::new();
    for feature in installed {
        if open {
            if !feature.on {
                out.push((format!("features.{}", feature.id), true));
            }
            out.extend(
                feature
                    .tools
                    .iter()
                    .filter(|t| !t.on && feature.on)
                    .map(|t| (format!("tools.{}", t.name), true)),
            );
        } else {
            out.push((format!("features.{}", feature.id), false));
        }
    }
    out
}

/// 照写的键改手上的这一份（引导里自定义的预设先攒着，建的时候一起交）。
pub fn apply(features: &mut [Feature], changes: &[(String, bool)]) {
    for (key, on) in changes {
        if let Some(id) = key.strip_prefix("features.") {
            features
                .iter_mut()
                .filter(|f| f.id == id)
                .for_each(|f| f.on = *on);
        } else if let Some(name) = key.strip_prefix("tools.") {
            features
                .iter_mut()
                .flat_map(|f| f.tools.iter_mut())
                .filter(|t| t.name == name)
                .for_each(|t| t.on = *on);
        }
    }
}

/// 建一个新预设时要写的：关着的功能、开着的功能里关掉的工具（新建的什么都不写就是全开）。
pub fn offs(features: &[Feature]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for feature in features.iter().filter(|f| f.installed) {
        if !feature.on {
            out.push((format!("features.{}", feature.id), false));
            continue;
        }
        out.extend(
            feature
                .tools
                .iter()
                .filter(|t| !t.on)
                .map(|t| (format!("tools.{}", t.name), false)),
        );
    }
    out
}

#[cfg(test)]
mod tests;
