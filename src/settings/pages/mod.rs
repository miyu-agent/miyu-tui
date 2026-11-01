//! 主菜单里「供应商和模型」以外的几页（蓝图「配置页」第 5、32 到 36 条，2026-10-07 项目主人定）：通用、权限、高级
//! 照核心 `config.schema` 画，人格只看。这里是状态、主菜单的先后、值怎么写；按键在 `keys.rs`，画在
//! `ui/settings/pages.rs`。

mod avatar;
pub mod catalog;
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
mod picks;
mod popup_keys;
pub mod preset_edit;
pub mod presets;
pub mod records;
pub mod schema;
pub mod secret;
pub mod slider;
pub mod store;
pub mod target;
pub mod textarea;
pub mod ticks;
mod values;

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use choose::{Choice, Choose};
use personas::Persona;
use schema::Schema;
pub use values::{choices, current, is_named, shown};

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
    /// 软件包：下载软件、已安装、软件后台三页（第 43c 条），一直列。
    Packages,
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
    /// 「软件包」三页（「配置页」第 43c 条）：状态在 [`More::store`]；包自己那一页是 `Page("packages/<包>")`。
    Store,
    /// 「MCP」：服务的列表，选中第几个（第 43b 条）。
    Mcp(usize),
}

/// 新几页的状态。
#[derive(Debug, Default)]
pub struct More {
    /// 开机启动要在后台跑的（`settings/service.rs`，App 拿去跑）。
    pub service_runs: Vec<crate::service::Verb>,
    /// 知道核心程序在哪（`MIYU_CORE_BIN`）：不知道的不列「开机启动」。
    pub service_known: bool,
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
    /// 供应商目录里常用的几家（`provider.catalog {"featured": true}`）：开配置页时先读好，按 `a` 开目录窗当场列出来
    /// （「配置页」第 11a 条，2026-10-30 项目主人：「每次加载的画面都很短，会闪一下」）。还没读到的是 `None`。
    pub featured: Option<Vec<catalog::Entry>>,
    /// 问了常用的几家、还没回来：开配置页时重读的那几下不再问（读完三样会马上补读一次）。
    pub featured_asked: bool,
    /// 读来的清单、预设列表的原样：终端记下，下次开配置页第一帧照它画主菜单（`app/look.rs`，2026-10-21 项目主人：
    /// 「/config 打开配置菜单闪了一下」）。只有清单和名字，没有配置的值。
    pub raw: (Option<Value>, Option<Value>),
    /// 主菜单选中第几项。
    pub menu_at: usize,
    /// 每个包的一句说明（`package.list` 的 `summary`）：「软件包」的列表照它写。
    pub summaries: HashMap<String, String>,
    /// 每个包的作者、从哪来、上游（`package.list` 的 `author`、`from`、`upstream`，核心 F-12）：包页面包屑下面那一行。
    pub origins: HashMap<String, packages::Origin>,
    /// 系统配置里的 MCP 服务（`mcp.list`）；核心不认的（旧核心）是空的。
    pub mcp_home: Vec<mcp::Server>,
    /// 包里带的 MCP 服务（`package.list` 每个包的 `mcp`）。
    pub mcp_bundled: Vec<mcp::Server>,
    /// 进了哪一页。
    pub section: Option<Section>,
    /// 「软件包」三页（第 43c 条）。
    pub store: store::Store,
    /// 滑块调了还没存的（第 33 条子规则）。
    pub sliding: Option<slider::Sliding>,
    /// 知识库（`kb.list`，选知识库的项照它列）；还没读到、核心不认的是 `None`。
    pub knowledge: Option<Vec<Persona>>,
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
        // 核心以后多了的页排在人格（以后是预设）和高级中间；「软件包」在清单给的位置，清单里没有的排在高级前面（第 43c 条：
        // 一直列）。
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
                .map(|p| match *p {
                    packages::PACKAGES => Entry::Packages,
                    p => Entry::Page(p.to_string()),
                }),
        );
        if !out.contains(&Entry::Packages) {
            out.push(Entry::Packages);
        }
        // MCP 放在软件包上面（同网页，2026-10-20 项目主人）。
        if !self.servers().is_empty() {
            let at = out
                .iter()
                .position(|e| *e == Entry::Packages)
                .unwrap_or(out.len());
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
    /// 壁纸的写系统配置。
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

    /// 一项照哪份列表选、写名字：默认人格这类照键，`persona`、`knowledge` 类型的照人格、知识库（第 33 条子规则）。
    pub fn named_item(&self, item: &schema::Item) -> Option<&[Persona]> {
        self.named(&item.key).or(match item.kind.as_str() {
            "persona" => self.personas.as_deref(),
            "knowledge" => self.knowledge.as_deref(),
            _ => None,
        })
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
    /// 来自哪一层：`default`、`system`、`project`。
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
    /// 系统配置里没写这一项，按 `d`。
    pub nothing_to_unset: String,
    /// 现在的默认人格行尾写的。
    pub default_mark: String,
    /// 一个人格都没有。
    pub no_personas: String,
    /// 正在读。
    pub loading: String,
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
    /// 「开机启动」那一组（`settings/service.rs`）。
    pub service: crate::settings::service::Words,
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
    /// 记录列表（第 47b 条）。
    pub records: records::RecordTexts,
    /// 勾选窗（第 47a 条）。
    pub ticks: ticks::TicksTexts,
    /// 包从哪来（第 43a 条，包页面包屑下面那一行）：出厂自带、官方软件源、本机安装（照 `package.list` 的 `from`）。
    pub package_from: std::collections::BTreeMap<String, String>,
    /// 「MCP」那一页（第 43b 条）。
    pub mcp: mcp::McpTexts,
    /// 光标停在滑块上时的按键提示（第 33 条子规则）。
    pub slider_hints: Vec<[String; 2]>,
    /// 「软件包」三页（第 43c 条）。
    pub store: store::StoreTexts,
}

#[cfg(test)]
mod tests;
