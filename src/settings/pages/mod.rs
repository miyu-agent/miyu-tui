//! 主菜单里「供应商和模型」以外的几页（蓝图「配置页」第 5、32 到 36 条，2026-10-07 项目主人定）：通用、权限、高级
//! 照核心 `config.schema` 画，人格只看。这里是状态、主菜单的先后、值怎么写；按键在 `keys.rs`，画在
//! `ui/settings/pages.rs`。

mod avatar;
pub mod choose;
pub mod combo;
pub mod dialogs;
mod keys;
pub mod list;
pub mod mascots;
pub mod mcp;
pub mod model_or;
pub mod packages;
pub mod persona_edit;
mod persona_seed;
pub mod personas;
pub mod preset_edit;
pub mod presets;
pub mod schema;
pub mod secret;
pub mod target;
pub mod textarea;
pub mod ticks;

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::data::Data;
use choose::{Choice, Choose};
use personas::Persona;
use schema::{Item, Schema};

/// 默认人格那一项的键：下拉照 `persona.list` 列。
pub const DEFAULT_PERSONA: &str = "persona.default";
/// 默认预设那一项的键：下拉照 `preset.list` 列（核心 P-2 上）。
pub const DEFAULT_PRESET: &str = "preset.default";

/// 换哪一只吉祥物（终端自己的配置项，「吉祥物包」第 3 条）：值是吉祥物包的编号，没写的是内置的。
pub const MASCOT: &str = "tui.mascot";
/// 主题（蓝图「主题」第 5 条）：值是一个主题包的编号，没设的是默认主题。
pub const LOOK: &str = "tui.look";
/// 放在最后的那一页。
const LAST: &str = "advanced";
/// 放在第一行的那一页。
const FIRST: &str = schema::GENERAL;

/// 主菜单的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// 照 `config.schema` 画的一页：页的编号。
    Page(String),
    /// 供应商和模型。
    Models,
    /// 人格。
    Personas,
    /// 预设（核心有 `preset.list` 才列）。
    Presets,
    /// MCP 服务（有服务才列，第 43b 条）。
    Mcp,
}

/// 进了哪一页（「供应商和模型」那三页不算，归 `Nav`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    /// 照清单画的一页：页的编号、选中第几项（只算项，不算组名）。
    Page(String, usize),
    /// 人格：选中第几个。
    Personas(usize),
    /// 预设：选中第几个。
    Presets(usize),
    /// 「软件包」：有配置项的包的列表，选中第几个（「配置页」第 43a 条）；进去是 `Page("packages/<包>")`。
    Packages(usize),
    /// 「MCP」：服务的列表，选中第几个（第 43b 条）。
    Mcp(usize),
}

/// 新几页的状态。
#[derive(Debug, Default)]
pub struct More {
    /// 读来的清单；还没读到、核心不认的是 `None`。
    pub schema: Option<Schema>,
    /// 读来的人格；还没读到、核心不认的是 `None`。
    pub personas: Option<Vec<Persona>>,
    /// 读来的预设（和人格一个形状）；还没读到、核心不认的是 `None`。
    pub presets: Option<Vec<Persona>>,
    /// 装了的吉祥物包（和人格一个形状）；还没读到、核心不认的是 `None`。
    pub mascots: Option<Vec<Persona>>,
    /// 装了的主题包（默认主题包不列：没设就是它）；还没读到的是 `None`。
    pub looks: Option<Vec<Persona>>,
    /// 读来的清单、预设列表的原样：终端记下，下次开配置页第一帧照它画主菜单（`app/look.rs`，2026-10-21 项目主人：
    /// 「/config 打开配置菜单闪了一下」）。只有清单和名字，没有配置的值。
    pub raw: (Option<Value>, Option<Value>),
    /// 主菜单选中第几项。
    pub menu_at: usize,
    /// 每个包的一句说明（`package.list` 的 `summary`）：「软件包」的列表照它写。
    pub summaries: HashMap<String, String>,
    /// 每个包的作者、从哪来、上游（`package.list` 的 `author`、`from`、`upstream`，核心 F-12）：包页面包屑下面那一行。
    pub origins: HashMap<String, packages::Origin>,
    /// 个人设置里的 MCP 服务（`mcp.list`）；核心不认的（旧核心）是空的。
    pub mcp_home: Vec<mcp::Server>,
    /// 包里带的 MCP 服务（`package.list` 每个包的 `mcp`）。
    pub mcp_bundled: Vec<mcp::Server>,
    /// 进了哪一页。
    pub section: Option<Section>,
    /// 找到了能取色的桌面壁纸：配色列表里列「跟随壁纸」（App 每一拍照 `theme/wallpaper.rs` 设，2026-10-14 项目主人定：
    /// 找不到的不列）。
    pub wallpaper: bool,
    /// 能当主色的几个候选，好的在前（跟随头像、跟随壁纸时；App 每一拍照 `app/theme.rs` 设）。
    pub seeds: Vec<u32>,
    /// 正在用的主色：「主色」那一项照它画色块。
    pub seed: Option<u32>,
    /// 配色是跟随头像、跟随壁纸：「主色」那一项才列（2026-10-14 项目主人定）。
    pub following: bool,
    /// 配色是终端配色：「铺底色」那一项不列，永远露出终端自己的底（2026-10-15 项目主人定）。
    pub terminal_colors: bool,
    /// 跟随头像时是哪个人格：选了主色写它（`persona.set`）；跟随壁纸的是 `None`（写 `tui.theme_seed`）。
    pub seed_persona: Option<String>,
}

impl More {
    /// 主菜单照这个先后（2026-10-07 项目主人，2026-10-10 再定）：通用、外观（2026-10-14 加，2026-10-21 项目主人挪到通用下面）、
    /// 供应商和模型（2026-10-19 项目主人：挪到用户上面）、用户（核心 P-9 再补，2026-10-16 项目主人定）、隐私与安全（核心 5-15
    /// 下，2026-10-21 项目主人照推荐定）、人格、预设、软件包（和以后的别的页）、高级。「权限」
    /// 并进了「通用」，「接入」并进了「软件包」（`schema.rs`）。
    pub fn entries(&self) -> Vec<Entry> {
        let pages: Vec<&str> = self
            .schema
            .as_ref()
            .map(Schema::listed_pages)
            .unwrap_or_default();
        let mut out = Vec::new();
        if pages.contains(&FIRST) {
            out.push(Entry::Page(FIRST.to_string()));
        }
        // 外观紧跟通用（2026-10-21 项目主人：「把外观的顺序放在通用下面」）。
        if pages.contains(&schema::APPEARANCE) {
            out.push(Entry::Page(schema::APPEARANCE.to_string()));
        }
        out.push(Entry::Models);
        for page in [schema::USER, schema::PRIVACY] {
            if pages.contains(&page) {
                out.push(Entry::Page(page.to_string()));
            }
        }
        out.push(Entry::Personas);
        if self.presets.as_ref().is_some_and(|l| !l.is_empty()) {
            out.push(Entry::Presets);
        }
        // 核心以后多了的页排在人格（以后是预设）和高级中间。
        out.extend(
            pages
                .iter()
                .filter(|p| {
                    ![
                        FIRST,
                        schema::USER,
                        schema::PRIVACY,
                        schema::APPEARANCE,
                        LAST,
                    ]
                    .contains(*p)
                })
                .map(|p| Entry::Page((*p).to_string())),
        );
        // MCP 放在软件包上面（同网页，2026-10-20 项目主人），没有软件包页的放在高级前面。
        if !self.servers().is_empty() {
            let packages = Entry::Page(packages::PACKAGES.to_string());
            let at = out.iter().position(|e| *e == packages).unwrap_or(out.len());
            out.insert(at, Entry::Mcp);
        }
        if pages.contains(&LAST) {
            out.push(Entry::Page(LAST.to_string()));
        }
        out
    }

    /// 一页排成行：配色不是跟随头像、跟随壁纸的，「主色」那一项不列；配色是终端配色的，「铺底色」那一项不列。
    pub fn rows(&self, page: &str) -> Vec<schema::Row> {
        if let Some(id) = packages::package_of(page) {
            return self.package_rows(id);
        }
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        schema
            .rows(page)
            .into_iter()
            .filter(|row| match row {
                schema::Row::Item(i) => {
                    let key = schema.items[*i].key.as_str();
                    (self.following || key != crate::theme::choice::SEED_KEY)
                        && !(self.terminal_colors && key == crate::theme::choice::GROUND_KEY)
                }
                schema::Row::Group(_) => true,
            })
            .collect()
    }

    /// 主色的选择窗：候选一个一行、前面一块颜色，第一个后面暗色写「默认」；停在正在用的那个上。跟随头像的写人格，跟随
    /// 壁纸的写个人设置。
    pub fn seed_choose(&self, title: &str, default_mark: &str) -> Choose {
        let choices = self
            .seeds
            .iter()
            .enumerate()
            .map(|(i, seed)| Choice {
                value: Value::from(format!("#{seed:06x}")),
                label: format!("#{seed:06x}"),
                note: (i == 0).then(|| default_mark.to_string()),
                off: None,
            })
            .collect();
        let at = self
            .seed
            .map_or(Value::Null, |s| Value::from(format!("#{s:06x}")));
        let mut choose = Choose::open(crate::theme::choice::SEED_KEY, title, choices, &at);
        choose.swatches = true;
        if let Some(persona) = &self.seed_persona {
            choose.target = target::Target::PersonaSeed(persona.clone());
        }
        choose
    }

    /// 选择窗列哪几个：找不到壁纸的，配色里不列「跟随壁纸」。
    pub fn offered(&self, key: &str, mut list: Vec<Choice>) -> Vec<Choice> {
        if key == "tui.theme" && !self.wallpaper {
            list.retain(|c| c.value != "wallpaper");
        }
        list
    }

    /// 名字照列表写、下拉照列表列的那几项（默认人格、默认预设）的列表；别的项、还没读到的是 `None`。
    pub fn named(&self, key: &str) -> Option<&[Persona]> {
        match key {
            DEFAULT_PERSONA => self.personas.as_deref(),
            DEFAULT_PRESET => self.presets.as_deref(),
            MASCOT => self.mascots.as_deref(),
            LOOK => self.looks.as_deref(),
            _ => None,
        }
    }

    /// 主菜单停在哪一项：没读到清单以前「供应商和模型」排第一，读到以后通用排第一，选中的照「是哪一项」留住。
    pub fn selected(&self) -> Entry {
        let entries = self.entries();
        entries
            .get(self.menu_at.min(entries.len().saturating_sub(1)))
            .cloned()
            .unwrap_or(Entry::Models)
    }
}

/// 新几页的字（`text/zh.json` 的 `settings.more`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 主菜单「人格」那一项和它底下那一行（写清它决定什么：AI 的人格和记忆）。
    pub personas: [String; 2],
    /// 主菜单「预设」那一项和它底下那一行（决定开关哪些功能）。
    pub presets: [String; 2],
    /// 预设的窗：功能（`{on}/{all}`）、没安装。
    pub preset_detail: [String; 3],
    /// 来自哪一层：`default`、`system`、`personal`、`project`。
    pub origins: HashMap<String, String>,
    /// 开关的两种：开、关。
    pub toggle: [String; 2],
    /// 密钥的两种：已设置、未设置。
    pub secret: [String; 2],
    /// 终端还没有编辑器的项按了 Enter。
    pub no_editor: String,
    /// 数读不出来。
    pub bad_number: String,
    /// 恢复默认问的那一句：标题 `{name}`、一行话 `{from}`、两个按钮。
    pub unset: [String; 4],
    /// 个人设置里没写这一项，按 `d`。
    pub nothing_to_unset: String,
    /// 现在的默认人格行尾写的。
    pub default_mark: String,
    /// 一个人格都没有。
    pub no_personas: String,
    /// 正在读。
    pub loading: String,
    /// 「软件包」一个有配置项的包都没有。
    pub no_packages: String,
    /// 「软件包」列表右边写的几项：`{n}`。
    pub package_items: String,
    /// 只写个数的列表写的：`{n}`（[`list::counted`]）。
    pub counted: String,
    /// 默认人格没设、选择窗第一行：不带人格（2026-10-08 项目主人定叫「无人格」）。
    pub no_persona: String,
    /// 吉祥物没设、选择窗第一行：内置的那只（「吉祥物包」第 3 条）。
    pub builtin_mascot: String,
    /// 主题没设、选择窗第一行：默认主题（「主题」第 5 条）。
    pub default_look: String,
    /// 选着的吉祥物包卸掉了：`{id}` 是它的编号。
    pub mascot_missing: String,
    /// 「主色」跟随头像时恢复默认问的那一句：清掉人格的主题色。
    pub seed_unset: String,
    /// 终端自己的配置项另分的几组的名字（「配置页」第 33 条）。
    pub own_groups: schema::OwnGroups,
    /// 「外观」那一页的名字（主题、吉祥物、图标，2026-10-14）。
    pub appearance: String,
    /// 终端自己的选项写成什么（清单里只有值，照界面语言写名字）：键、值、名字。
    pub choice_names: HashMap<String, HashMap<String, String>>,
    /// 选项用不了（核心给了 `available: false`，现在只有没装内置语义模型时的「内置模型」）。
    pub unavailable: String,
    /// 人格的窗：名字、具体人格提示词、示范对话、人设提醒短语、`{n} 轮`、已写、简单描述、子代理可选（2026-10-17 加在最后，不挪下标）。没有的空着（2026-10-09 项目主人：「没有的东西
    /// 就空着，不需要写`没有`」）。
    pub detail: [String; 8],
    /// 几页的按键提示（键、做什么）：通用这类的页、（空）、选择窗、编辑窗、（空）、人格页和预设页、人格的窗、预设的窗、
    /// 示范对话列表、两格窗、「软件包」的包列表。
    pub hints: [Vec<[String; 2]>; 11],
    /// 改预设（「配置页」第 39 条）。
    pub preset_edit: preset_edit::EditTexts,
    /// 改人格（「配置页」第 36、37 条）。
    pub persona_edit: persona_edit::EditTexts,
    /// 示范对话列表（「配置页」第 41 条）。
    pub dialogs: dialogs::DialogTexts,
    /// `model_or` 那种项的选择窗最后一行：自己填一个模型（「配置页」第 21 条）。
    pub custom_model: String,
    /// 下拉加「其他」手写（`combo`，「配置页」第 44 条）的两个字：第一行「不设」、最后一行「其他…」。
    pub combo: [String; 2],
    /// 几行字的编辑窗（`textarea`，第 45 条）下面的按键提示，同引导的大编辑浮窗。
    pub sheet_hints: Vec<[String; 2]>,
    /// 列表窗（第 47 条）。
    pub list: list::ListTexts,
    /// 勾选窗（第 47a 条）。
    pub ticks: ticks::TicksTexts,
    /// 包从哪来（第 43a 条，包页面包屑下面那一行）：出厂自带、官方软件源、本机安装（照 `package.list` 的 `from`）。
    pub package_from: std::collections::BTreeMap<String, String>,
    /// 「MCP」那一页（第 43b 条）。
    pub mcp: mcp::McpTexts,
}

/// 一项现在的值：读来的最终值和来自哪一层；哪一层都没写的照清单的默认，来自「默认」。
pub fn current<'a>(item: &'a Item, data: &'a Data) -> (&'a Value, &'a str) {
    match data.values.get(&item.key) {
        Some((value, origin)) => (value, origin.as_str()),
        None => (&item.default, "default"),
    }
}

/// 名字照列表写、下拉照列表列的项（默认人格、默认预设）：不管清单写什么控件，一律开选择窗。
pub fn is_named(key: &str) -> bool {
    key == DEFAULT_PERSONA || key == DEFAULT_PRESET || key == MASCOT || key == LOOK
}

/// 值写成给人看的样子：下拉写选项的名字（默认人格、默认预设写名字，`named` 是它们的列表），开关写开、关，密钥写设没设，
/// 列表用顿号连（私密目录两项只写个数）。
pub fn shown(item: &Item, value: &Value, named: Option<&[Persona]>, texts: &Texts) -> String {
    if let (Some(list), Some(id)) = (named, value.as_str())
        && let Some(p) = list.iter().find(|p| p.id == id)
    {
        return p.label().to_string();
    }
    // 选着的吉祥物包、主题包卸掉了：编号后面写没装（「吉祥物包」第 3 条、「主题」第 5 条）。
    if (item.key == MASCOT || item.key == LOOK)
        && named.is_some()
        && let Some(id) = value.as_str()
    {
        return texts.mascot_missing.replace("{id}", id);
    }
    if let Some(name) = own_choice(item, value, texts) {
        return name;
    }
    if let Some(name) = item
        .options
        .iter()
        .find(|o| o.value == *value)
        .map(|o| &o.name)
    {
        return name.clone();
    }
    // 密钥：只写设了没设，不露引用（第 48 条）。
    if secret::is_secret(item) {
        return texts.secret[usize::from(value.is_null())].clone();
    }
    if let Some(n) = list::counted(item, value) {
        return match n {
            0 => "—".to_string(),
            n => texts.counted.replace("{n}", &n.to_string()),
        };
    }
    match (item.control.as_str(), value) {
        // 默认人格没设：就是无人格。
        (_, Value::Null) if item.key == DEFAULT_PERSONA => texts.no_persona.clone(),
        // 吉祥物没设：就是内置的。
        (_, Value::Null) if item.key == MASCOT => texts.builtin_mascot.clone(),
        // 主题没设：就是默认主题。
        (_, Value::Null) if item.key == LOOK => texts.default_look.clone(),
        // 下拉加「其他」手写没设：写「不设」（「配置页」第 44 条）。
        ("combo", Value::Null) => texts.combo[0].clone(),
        (_, Value::Null) => "—".to_string(),
        // 几行字：只写第一行，多行的末尾 `…`（第 45 条）。
        ("textarea", Value::String(s)) if s.trim_end().contains('\n') => {
            format!(
                "{}{}",
                s.lines().next().unwrap_or_default(),
                crate::sheet::ellipsis()
            )
        }
        ("toggle", Value::Bool(on)) => texts.toggle[usize::from(!*on)].clone(),
        (_, Value::String(s)) => s.clone(),
        // 空列表同没设（「配置页」第 47 条：删光了写的是空列表）。
        (_, Value::Array(list)) if list.is_empty() => "—".to_string(),
        (_, Value::Array(list)) => list
            .iter()
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
            .collect::<Vec<_>>()
            .join("、"),
        (_, other) => other.to_string(),
    }
}

/// 下拉的选择窗里列什么：默认人格、默认预设照列表（`named`，写错的选不了），别的照清单的选项。
pub fn choices(item: &Item, named: Option<&[Persona]>, texts: &Texts) -> Vec<Choice> {
    if let Some(list) = named {
        // 默认人格第一行「无人格」：选了就不设（2026-10-08 项目主人：人格允许为空）。
        let first = match item.key.as_str() {
            DEFAULT_PERSONA => Some(&texts.no_persona),
            // 吉祥物第一行「内置」：选了就不设（「吉祥物包」第 3 条）。
            MASCOT => Some(&texts.builtin_mascot),
            // 主题第一行「默认主题」：选了就不设（「主题」第 5 条）。
            LOOK => Some(&texts.default_look),
            _ => None,
        };
        let none = first.map(|label| Choice {
            value: Value::Null,
            label: label.clone(),
            note: None,
            off: None,
        });
        let named = list.iter().map(|p| Choice {
            value: Value::from(p.id.clone()),
            label: p.label().to_string(),
            // 只写名字，说明在人格页、预设页看（2026-10-07 项目主人：没必要）。
            note: None,
            off: p.problem.clone(),
        });
        return none.into_iter().chain(named).collect();
    }
    item.options
        .iter()
        .map(|o| Choice {
            value: o.value.clone(),
            label: own_choice(item, &o.value, texts).unwrap_or_else(|| o.name.clone()),
            note: o.note.clone(),
            // 用不了的列着、灰的、选不了，为什么照头自己的一句（核心 R-5 三补）。
            off: (!o.available).then(|| texts.unavailable.clone()),
        })
        .collect()
}

/// 终端自己的选项（`tui.theme` 这类）照界面语言的名字；没登记的是 `None`，照清单写。
fn own_choice(item: &Item, value: &Value, texts: &Texts) -> Option<String> {
    texts
        .choice_names
        .get(&item.key)
        .and_then(|names| names.get(value.as_str()?))
        .cloned()
}

#[cfg(test)]
mod tests;
