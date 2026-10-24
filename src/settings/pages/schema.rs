//! `config.schema` 读成几页（蓝图「配置页」第 5、32 条）：页、组、一项项。通用、权限、高级这几页照它画；键里有人起
//! 的名字的（`providers.<id>.…`）归「供应商和模型」，别的头专有的（`web.*`）不列。

use serde_json::Value;

/// 一项。
/// 下拉的一个选项（`config.schema` 的 `options`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt {
    /// 值。
    pub value: Value,
    /// 给人看的名字。
    pub name: String,
    /// 核心查得出的一句，接在名字后面暗色写（核心 R-5 再补：`models.embedding` 的 `local` 写内置模型的名字）。
    pub note: Option<String>,
    /// 用得了：核心查得出用不了的（没装内置语义模型的包）列着、灰的、选不了（核心 R-5 三补）。
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// 键。
    pub key: String,
    /// 控件：`select`、`toggle`、`number`、`text`、`list`、`secret`、`custom:…`。
    pub control: String,
    /// 类型（`config.schema` 的 `type`）：`text`、`int`、`secret`、`list` 这些（密钥照它认，「配置页」第 48 条）。
    pub kind: String,
    /// 列表的元素的类型（`list`，「配置页」第 47 条）：`text`、`int`、`secret` 这些；别的控件是空的。
    pub element: String,
    /// 下拉加「其他」手写的建议值（`combo`，核心 P-9 补：字表照连接的语言给，照先后）；别的控件是空的。
    pub suggestions: Vec<String>,
    /// 下拉的选项。
    pub options: Vec<Opt>,
    /// 能写在哪几层。
    pub layers: Vec<String>,
    /// 默认值；没有的是 `null`。
    pub default: Value,
    /// 名字。
    pub name: String,
    /// 在哪一页。
    pub page: String,
    /// 哪一组。
    pub group: String,
    /// 属于哪个包（核心给的 `package`）；没写的是空的，照组的编号算（「配置页」第 43a 条）。
    pub package: String,
    /// 常用项：排在组里前面。
    pub common: bool,
    /// 带选项的列表反着勾（核心给的 `"ticks": "unlisted"`）：勾着的是不在值里的（「配置页」第 47a 条，2026-10-21 项目主人：
    /// 「默认私密目录」进去是几个已经勾着的目录，空格取消勾选）。
    pub unlisted: bool,
}

impl Item {
    /// 能写进个人设置。
    pub fn personal(&self) -> bool {
        self.layers.iter().any(|l| l == "personal")
    }

    /// 能写进系统配置（大家共用）：管理员模式开着时写这一层（「管理员模式」第 3 条）。
    pub fn allows_system(&self) -> bool {
        self.layers.iter().any(|l| l == "system")
    }

    /// 只能写系统配置（运行日志级别这类，「配置页」第 33 条）。
    pub fn system_only(&self) -> bool {
        !self.personal() && self.layers.iter().any(|l| l == "system")
    }

    /// 终端改得了：个人设置、系统配置有一层能写。
    pub fn writable(&self) -> bool {
        self.personal() || self.system_only()
    }
}

/// 一组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// 编号。
    pub id: String,
    /// 名字。
    pub name: String,
    /// 在哪一页。
    pub page: String,
}

/// 读来的清单。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Schema {
    /// 页：编号、名字，照核心交回的先后。
    pub pages: Vec<(String, String)>,
    /// 组，照核心交回的先后。
    pub groups: Vec<Group>,
    /// 项，照清单登记的先后。
    pub items: Vec<Item>,
}

/// 一页里的一行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// 组名：第几组。
    Group(usize),
    /// 一项：第几项。
    Item(usize),
}

/// 「供应商和模型」那一页的编号：不另列。
pub const MODELS: &str = "models";
/// 「通用」那一页的编号。
pub const GENERAL: &str = "general";
/// 「外观」那一页的编号：终端自己加的一页，核心不知道，主菜单排在通用后面（2026-10-14 项目主人照推荐定）。
pub const APPEARANCE: &str = "appearance";
/// 「用户」页（核心 P-9 再补）：主菜单紧跟「通用」（「配置页」第 46 条）。
pub const USER: &str = "user";
/// 「隐私与安全」页（核心 5-15 下）：主菜单排在「用户」后面、「外观」前面（2026-10-21 项目主人照推荐定，同核心、网页的先后）。
pub const PRIVACY: &str = "privacy";
/// 核心的「新会话」一组（核心排着做，2026-10-23）。
const NEW_SESSION: &str = "new_session";
/// 「新会话不再询问人格预设」。
const SKIP_PICK: &str = "tui.skip_pick";
/// 终端自己的配置项的键打头的那一截（终端软件包的编号）。
const OWN: &str = "tui.";

/// 终端自己的配置项里另分一组的：键的开头、组的编号、放哪一页（「配置页」第 33 条，2026-10-11 项目主人：时间线单独一个
/// 标题，吉祥物也分出去；2026-10-14：主题、吉祥物、图标单开一页「外观」，时间线留在通用）。组照这个先后排。
const OWN_GROUPS: [(&str, &str, &str); 7] = [
    ("tui.look", "tui.look", APPEARANCE),
    ("tui.theme", "tui.theme", APPEARANCE),
    ("tui.mascot", "tui.mascot", APPEARANCE),
    ("tui.icons", "tui.icons", APPEARANCE),
    ("tui.timeline_", "tui.timeline", GENERAL),
    ("tui.keys", "tui.keys", GENERAL),
    // 新会话不问人格、预设（2026-10-22）：也是怎么操作的事，跟按键模式一组。
    ("tui.skip_pick", "tui.keys", GENERAL),
];

/// 另分的那几组的名字（`text/zh.json` 的 `settings.more.own_groups`）。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnGroups {
    /// 主题（2026-10-21：框线、记号这些，`tui.look`）。
    pub look: String,
    /// 配色方案（2026-10-13：配色、明暗、铺底色；2026-10-21 从「主题」改名）。
    pub theme: String,
    /// 时间线。
    pub timeline: String,
    /// 吉祥物。
    pub mascot: String,
    /// 图标。
    pub icons: String,
    /// 交互（按键模式这些，2026-10-21 项目主人：「按键模式」应在「交互」这个标题下）。
    pub keys: String,
}

impl OwnGroups {
    /// 照组的编号取名字。
    fn name(&self, id: &str) -> &str {
        match id {
            "tui.look" => &self.look,
            "tui.theme" => &self.theme,
            "tui.timeline" => &self.timeline,
            "tui.icons" => &self.icons,
            "tui.keys" => &self.keys,
            _ => &self.mascot,
        }
    }
}

impl Schema {
    /// 读 `config.schema` 的回应。
    pub fn read(got: &Value) -> Schema {
        let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
        let pages = got["pages"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| (text(&p["id"]), text(&p["name"])))
            .collect();
        let groups = got["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|g| Group {
                id: text(&g["id"]),
                name: text(&g["name"]),
                page: text(&g["page"]),
            })
            .collect();
        let items = got["items"]
            .as_array()
            .into_iter()
            .flatten()
            // 标了 `hidden` 的设置页不画（照样能写、能查，核心 9-1 下）。
            .filter(|i| i["hidden"] != true)
            .map(|i| Item {
                key: text(&i["key"]),
                control: text(&i["control"]),
                kind: text(&i["type"]),
                element: text(&i["element"]),
                suggestions: i["suggestions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect(),
                options: i["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| Opt {
                        value: o["value"].clone(),
                        name: text(&o["name"]),
                        note: o["note"].as_str().map(str::to_string),
                        available: o["available"] != false,
                    })
                    .collect(),
                layers: i["layers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(text)
                    .collect(),
                default: i["default"].clone(),
                name: text(&i["name"]),
                page: text(&i["page"]),
                group: text(&i["group"]),
                package: text(&i["package"]),
                common: i["common"] == true,
                unlisted: i["ticks"] == "unlisted",
            })
            // 键里有人起的名字的归供应商和模型；别的头专有的不列（蓝图「配置页」第 5 条）。
            .filter(|i: &Item| !i.key.contains('<') && !i.key.starts_with("web."))
            .collect();
        let mut schema = Schema {
            pages,
            groups,
            items,
        };
        schema.own_items_to_general();
        schema
    }

    /// 终端自己的配置项（`tui.` 开头，核心排在「软件包」页这个包那一组）挪到「通用」页，组照旧（「配置页」第 33 条，
    /// 2026-10-10 项目主人）。
    fn own_items_to_general(&mut self) {
        let mut moved = Vec::new();
        for item in self.items.iter_mut().filter(|i| i.key.starts_with(OWN)) {
            item.page = GENERAL.to_string();
            moved.push(item.group.clone());
        }
        for group in self.groups.iter_mut().filter(|g| moved.contains(&g.id)) {
            group.page = GENERAL.to_string();
        }
    }

    /// 终端自己的配置项分组、分页：主题、吉祥物、图标各一组放进「外观」（`appearance` 是这一页的名字，排在通用后面），
    /// 时间线一组留在通用、排在终端那一组后面；组名照界面的字。核心没有这几项的（旧核心）那几组是空的，不画，外观一项都没有
    /// 的主菜单不列。
    pub fn split_own(&mut self, names: &OwnGroups, appearance: &str) {
        let own = |g: &Group, items: &[Item]| {
            items
                .iter()
                .any(|i| i.key.starts_with(OWN) && i.group == g.id)
        };
        let Some(at) = self.groups.iter().position(|g| own(g, &self.items)) else {
            return;
        };
        let here = self.groups[at].page.clone();
        let mut added = 0;
        for (prefix, id, page) in &OWN_GROUPS {
            let page = if *page == APPEARANCE {
                APPEARANCE
            } else {
                &here
            };
            for item in self.items.iter_mut().filter(|i| i.key.starts_with(prefix)) {
                item.group = (*id).to_string();
                item.page = page.to_string();
            }
            // 几样并进同一组的（交互：按键模式、不问人格预设）只开一组。
            if self.groups.iter().any(|g| g.id == *id) {
                continue;
            }
            let group = Group {
                id: (*id).to_string(),
                name: names.name(id).to_string(),
                page: page.to_string(),
            };
            self.groups.insert(at + 1 + added, group);
            added += 1;
        }
        self.join_new_session();
        let after = self.pages.iter().position(|(id, _)| id == GENERAL);
        let page = (APPEARANCE.to_string(), appearance.to_string());
        self.pages.insert(after.map_or(0, |at| at + 1), page);
    }

    /// 「新会话不再询问人格预设」并进核心的「新会话」一组（`new_session`：默认人格、默认预设、开局只读，2026-10-23 项目主人：
    /// 「做成一个新会话标题下面的项，而不是分散各处」）；核心还没有这一组的照旧在「交互」。
    fn join_new_session(&mut self) {
        let Some(page) = self
            .groups
            .iter()
            .find(|g| g.id == NEW_SESSION)
            .map(|g| g.page.clone())
        else {
            return;
        };
        for item in self.items.iter_mut().filter(|i| i.key == SKIP_PICK) {
            item.group = NEW_SESSION.to_string();
            item.page = page.clone();
        }
    }

    /// 这一页的名字。
    pub fn page_name(&self, page: &str) -> Option<&str> {
        self.pages
            .iter()
            .find(|(id, _)| id == page)
            .map(|(_, name)| name.as_str())
    }

    /// 这一页有项的组名，照组的先后（主菜单底下那一行写它们）。
    pub fn group_names(&self, page: &str) -> Vec<&str> {
        self.groups
            .iter()
            .filter(|g| g.page == page)
            .filter(|g| self.items.iter().any(|i| i.page == page && i.group == g.id))
            .map(|g| g.name.as_str())
            .collect()
    }

    /// 一页排成行：组名一行，下面它的项，常用的在前；一项都没有的组不写。
    pub fn rows(&self, page: &str) -> Vec<Row> {
        let mut out = Vec::new();
        for (g, group) in self
            .groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.page == page)
        {
            let mut items: Vec<usize> = (0..self.items.len())
                .filter(|&i| self.items[i].page == page && self.items[i].group == group.id)
                .collect();
            if items.is_empty() {
                continue;
            }
            items.sort_by_key(|&i| !self.items[i].common);
            out.push(Row::Group(g));
            out.extend(items.into_iter().map(Row::Item));
        }
        out
    }

    /// 有项的页，照先后，不算「供应商和模型」那一页。
    pub fn listed_pages(&self) -> Vec<&str> {
        self.pages
            .iter()
            .map(|(id, _)| id.as_str())
            .filter(|id| *id != MODELS && !self.rows(id).is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests;
