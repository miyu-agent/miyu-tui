//! 引导要发给核心的请求（「第一次打开的引导」第 27 条）：走配置页那条通用的 `Command::Ask`，编号从
//! [`TAG_BASE`] 起，和配置页的分开；回来的照编号找到当时在等什么。

use std::collections::HashMap;

use serde_json::Value;

/// 引导的请求编号从这里起：比它小的是配置页的。
pub const TAG_BASE: u64 = 1 << 60;

/// 一条要发的请求。
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    /// 编号。
    pub tag: u64,
    /// 方法。
    pub method: &'static str,
    /// 参数。
    pub params: Value,
}

/// 等着哪一样回来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Waiting {
    /// 写了图标那一套（`config.set tui.icons`）。
    Icons,
    /// 写了按键模式（`config.set tui.keys`）。
    Keys,
    /// 读金额用的货币（`config.get usage.currency`，`currency.rs`）。
    CurrencyRead,
    /// 写了金额用的货币（`config.set usage.currency`）。
    CurrencySave,
    /// 模型和默认用途（`model.list`）。
    Models,
    /// 环境里的 key、本机的服务（`provider.detect`）。
    Detect,
    /// 常用的几家（`provider.catalog featured`）。
    Catalog,
    /// 全目录（「更多供应商…」，`provider.catalog` 不带 `featured`）。
    AllProviders,
    /// 试一家（`provider.test`）。
    Test,
    /// 存贴的 key（`secret.set`）。
    Secret,
    /// 写这一家和 `models.chat`（`config.set`）。
    SaveModel,
    /// 默认人格是哪个（`config.get persona.default`）。
    PersonaDefault,
    /// 默认人格的样子（`persona.get`）。
    PersonaGet(String),
    /// 默认人格的一份提示词（`persona.read`）。
    PersonaRead(String, &'static str),
    /// 建、改人格（`persona.set`）。
    PersonaSave,
    /// 传头像（`blob.put`，第 21 条）。
    AvatarPut,
    /// 写默认人格（`config.set persona.default`）：写的是哪个人格。
    PersonaPick(String),
    /// 预设列表（`preset.list`）。
    Presets,
    /// 默认预设是哪个（`config.get preset.default`）。
    PresetDefault,
    /// 一个预设开了哪些功能（`preset.get`）。
    PresetGet(String),
    /// 建自定义的预设（`preset.set`）。
    PresetSave,
    /// 写默认预设（`config.set preset.default`）：写的是哪个预设。
    PresetPick(String),
    /// 写「引导走过了」（`config.set ui.welcomed`）。
    Welcomed,
    /// AI 怎么称呼你、更多描述（`config.get profile.name profile.about`）。
    ProfileRead,
    /// 写这几项（`config.set`）。
    ProfileSave,
    /// 回复语言的建议值（`config.schema`，核心 P-9 补）。
    ProfileSchema,
    /// 主题的配色、明暗（`config.get tui.theme tui.theme_mode`）。
    ThemeRead,
    /// 写这两项（`config.set`）。
    ThemeSave,
    /// 写人格的主色（`persona.set`，跟随头像挑了主色的）。
    ThemeSeed,
    /// 装没装语义模型的包（`config.schema` 里 `models.embedding` 的 `local`）。
    SemanticSchema,
    /// 现在开着还是关着（`config.get models.embedding`）。
    SemanticRead,
    /// 写开还是关（`config.set models.embedding`）。
    SemanticSave,
    /// 没装的看一眼软件源里的（`package.install {"source": "embed", "preview": true}`）。
    SemanticPreview,
    /// 当场下载、装上（`package.install {"source": "embed"}`）。
    SemanticInstall,
}

impl Waiting {
    /// 读东西的（列表、默认的、详情）：断线了忘掉、重连时再要；别的是动作（试、存、写），断线了照被拒办。
    pub fn reads(&self) -> bool {
        matches!(
            self,
            Self::Models
                | Self::Detect
                | Self::Catalog
                | Self::AllProviders
                | Self::PersonaDefault
                | Self::PersonaGet(_)
                | Self::PersonaRead(..)
                | Self::Presets
                | Self::PresetDefault
                | Self::PresetGet(_)
                | Self::SemanticSchema
                | Self::SemanticRead
                | Self::SemanticPreview
                | Self::ThemeRead
                | Self::ProfileRead
                | Self::ProfileSchema
        )
    }
}

/// 攒着要发的、等着回来的。
#[derive(Debug, Default)]
pub struct Asker {
    next: u64,
    queued: Vec<Ask>,
    waiting: HashMap<u64, Waiting>,
}

impl Asker {
    /// 攒一条，记下等的是什么。
    pub fn ask(&mut self, method: &'static str, params: Value, waiting: Waiting) {
        self.push(method, params, waiting);
    }

    fn push(&mut self, method: &'static str, params: Value, waiting: Waiting) {
        let tag = TAG_BASE + self.next;
        self.next += 1;
        self.queued.push(Ask {
            tag,
            method,
            params,
        });
        self.waiting.insert(tag, waiting);
    }

    /// 攒着的都拿走（App 交给核心）。
    pub fn take(&mut self) -> Vec<Ask> {
        std::mem::take(&mut self.queued)
    }

    /// 这个编号回来了：当时等的是什么；不是引导的、已经认过的是 `None`。
    pub fn settle(&mut self, tag: u64) -> Option<Waiting> {
        self.waiting.remove(&tag)
    }

    /// 还在等这一种。
    pub fn pending(&self, what: &Waiting) -> bool {
        self.waiting.values().any(|w| w == what)
    }

    /// 断开了：等着的都不会回来了，交回它们（引导照被拒办动作、重新要读的，`link.rs`）。
    pub fn forget(&mut self) -> Vec<Waiting> {
        self.queued.clear();
        self.waiting.drain().map(|(_, waiting)| waiting).collect()
    }
}
