//! 核心那边交给界面的消息（从 `mod.rs` 挪出来：那里放不下了）。

use super::*;

/// 握手回应里说的这个人（核心 Y-12）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Account {
    /// 还没有机主（`setup: true`）：这时只能注册机主（`account.setup`），别的请求核心回 `setup_first`。
    pub setup: bool,
    /// 这个账号是不是管理员（机主，和管理员组里的人；`admin`）。
    pub admin: bool,
}

/// 核心那边的消息，交给界面。
#[derive(Debug)]
pub enum Update {
    /// 连上了，会话开好了，带着会话编号。
    Ready(String),
    /// 每次连上、握完手先说一声这个人是谁（在 [`Update::Ready`]、[`Update::Reconnected`] 前面）：还没有机主的进引导注册
    /// （「第一次打开的引导」第 1、5a 条）。
    Account(Account),
    /// 核心没在跑，也没给 `MIYU_CORE_BIN`，拉不起来。
    NoCoreBin,
    /// `MIYU_CORE_BIN` 指的程序不存在：路径（蓝图「连核心」第 8 条）。
    Missing(String),
    /// 断开以后又连上了，接着订阅着原来那个会话（第 7 条）。
    Reconnected,
    /// 主会话不存在了（在别处删了；`--resume` 的编号不对）：读页、补发、重连订阅时核心说的。不再对着它，下一句话开新会话
    /// （review B3，2026-10-11 项目主人定）。
    SessionGone(String),
    /// 连不上、握手或开会话被拒：原因。
    Failed(String),
    /// 一条请求被拒绝：原因码（`data.reason`，没有的是 `None`），和核心照握手时的语言说的原话。
    Refused {
        /// 原因码，例如 `nothing_to_unrevert`（`protocol/undo.md`）。
        reason: Option<String>,
        /// 核心的原话。
        message: String,
    },
    /// 核心断开了。
    Disconnected,
    /// 会话的限额（订阅的回应里的 `limits`）：窗口、压缩线，侧边栏和框下面那一行照它写上下文。
    Limits(Limits),
    /// 会话里的事。
    Push(Push),
    /// `/model` 框里的一行行（[`Command::ListChoices`] 的回应）；要不到的是空的。
    Choices(Vec<Choice>),
    /// 一个网址的卡片（[`Command::LinkPreview`]）；要不到的是 `None`。
    LinkCard {
        /// 网址。
        url: String,
        /// 卡片。
        card: Option<Card>,
    },
    /// 一个 blob 存成了文件（[`Command::FetchBlob`]）；读不成的是 `None`。
    BlobSaved {
        /// 哪个 blob。
        blob: String,
        /// 存在哪。
        path: Option<std::path::PathBuf>,
    },
    /// 一张 mermaid 图画好了（[`Command::RenderMermaid`]）；核心拒了、没编进 mermaid 的是 `None`。
    Mermaid {
        /// 源码。
        source: String,
        /// 画好的。
        svg: Option<Rendered>,
    },
    /// `/effort` 框里的几级（[`Command::ListEfforts`] 的回应）；要不到的是空的。
    Efforts(EffortList),
    /// `@` 文件列表的回应（[`Command::Files`]）：哪个词问的、回应的 `result`（回了错的是 `None`）。
    Files {
        /// 哪个词问的。
        word: crate::mention::Word,
        /// 回应。
        result: Option<serde_json::Value>,
    },
    /// 换模型成了（[`Command::Configure`] 的回应）：引用。
    Configured(String),
    /// 会话现在用的模型（订阅的回应里的 `model`）。
    CurrentModel(Current),
    /// 冷却着的模型里最早什么时候恢复（[`Command::ListModels`] 的回应）；没有的是 `None`。
    CoolingUntil(Option<jiff::Timestamp>),
    /// 给人看的字（[`Command::FetchHuman`] 的回应）。
    Human(crate::human::Human),
    /// 工具目录变了（`package.changed`、`mcp.changed`）：界面照这种语言重取给人看的字。
    ToolsChanged,
    /// 装包的下载进度（`package.progress`，核心 G-4 上）。
    PackageProgress(Progress),
    /// 配置里头要的几项（界面语言、金额排最前的币种）：连上时读一次，别处改了再读（「界面语言」）。
    HeadConfig(HeadConfig),
    /// 会话列表（[`Command::ListSessions`] 的回应）：只有主会话，照核心交回的先后。
    Sessions(Vec<SessionInfo>),
    /// 改名成了（`None` 是去掉了标题）：弹一句提示，标题照推送换（蓝图「改名」第 3 条）。
    Renamed(Option<String>),
    /// 软件包（[`Command::ListPackages`] 的回应）：编号、放状态的目录、图标；读不了、核心不认的是空的。
    Packages(Vec<PackageEntry>),
    /// 会话用哪个人格（订阅的回应里的 `persona`，核心 P-1 下）：人格编号。
    SessionPersona(String),
    /// 会话用哪个预设（订阅的回应里的 `preset`，核心 P-2 上）：预设编号。
    SessionPreset(String),
    /// 预设一行行（[`Command::ListPresets`] 的回应）；读不了、核心不认的是空的。
    Presets(Vec<Preset>),
    /// 人格一行行（[`Command::ListPersonas`] 的回应）；读不了、核心不认的是空的。
    Personas(Vec<Persona>),
    /// `usage.query` 的回应（`/usage`）：哪一样、一行行；拒了的是核心的原话。
    UsageRows {
        /// 哪一样。
        kind: UsageKind,
        /// 一行行，或者拒绝的原话。
        rows: Result<Vec<UsageRow>, String>,
        /// 用到的要署名的汇率源（核心 UC-1 的 `rates`）。
        rates: Vec<String>,
    },
    /// 核心认的斜杠命令（[`Command::Catalog`] 的回应）。
    Catalog(Vec<CoreCommand>),
    /// 订阅界面块的回应：装着、开着的扩展声明的全部块。
    Widgets(Vec<crate::widgets::Block>),
    /// 一块界面块变了（`widget.changed`）；`None` 是关掉、卸掉了。
    WidgetChanged {
        /// 哪个包。
        package: String,
        /// 包里的编号。
        id: String,
        /// 现在的样子。
        block: Option<crate::widgets::Block>,
    },
    /// `command.run` 回了 `opens`：打开那一整页（扩展的整页带的命令）。
    OpenPage(crate::widgets::PageRef),
    /// [`Command::PackageCall`] 被拒：原话。
    PackageCallRefused {
        /// 原话。
        message: String,
    },
    /// [`Command::Slash`] 被拒：照连接语言写的原话（`data.why` 有的照它；`command_failed` 的扩展原话接在下一行）。
    SlashRefused {
        /// 原话。
        message: String,
        /// 原因码（`data.reason`）：状态不对的不把打的那一行放回输入框。
        reason: Option<String>,
    },
    /// 斜杠命令办了（`command.run` 的回应）：规范的命令名、照连接语言写好的回执。
    CommandRan {
        /// `stop`、`clear`。
        command: String,
        /// 回执那一句；读不出来的是空的。
        said: String,
    },
    /// 回顾交回的是上一句（`cached`：上次回顾以后没有新内容，核心不推 `session.recapped`），照它画（蓝图「回顾」第 3 条）。
    Recap(String),
    /// 撤销（`restore` 为假）或恢复成了：核心算好的给人看的几样（`protocol/undo.md`）。
    Undone {
        /// 是恢复。
        restore: bool,
        /// 回应里给人看的几样。
        report: Report,
    },
    /// 订阅回应里「当前的」累计用量、权限、还在跑的（核心 9-6 上）：整个换掉事件算出来的。
    Snapshot(Snapshot),
    /// 最新一页读进来了：更早的还有没有（核心 9-6 下）。
    Paged(bool),
    /// [`Command::CheckDir`] 验完了：真实的位置，或者核心拒绝的原因。
    DirChecked(Result<String, Refusal>),
    /// 订阅回应里会话的工作区（核心 9-7 上）：侧边栏「工作区」照它写。接上老会话时终端在别的目录也不写那一句（2026-10-18
    /// 项目主人：resume 不换工作区，也不用提示）。
    Workspace {
        /// 会话在哪个目录干活。
        cwd: String,
    },
    /// 更早的一页（[`Command::Older`] 的回应）：读成的推送、再往前还有没有；读不成的是核心的原话。
    Older {
        /// 照补发的读成的推送。
        pushes: Vec<Push>,
        /// 再往前还有。
        more: bool,
        /// 读不成：核心的原话。
        failed: Option<String>,
    },
    /// 另外订阅着的会话推来的（推送、限额），带着是哪个会话。
    Elsewhere {
        /// 哪个会话。
        session: String,
        /// 推来的。
        update: Box<Update>,
    },
    /// 说的话、重做没发出去（核心拒了、附件传不上）：和 [`Update::Refused`] 一样的两样；先画进正文的那句要撤掉，
    /// 字放回输入框（「输入框」第 12、13 条）。
    /// 回答里贴的图传不上（核心的原话）：回答照样交了，只是不带图（抽屉已经关了，不交她会一直等）。
    AnswerFilesLost(String),
    Unsent {
        /// 原因码，例如 `not_redoable`。
        reason: Option<String>,
        /// 核心的原话。
        message: String,
    },
    /// [`Command::Ask`] 的回应：成了是 `result`，拒了是原因码和原话。
    Answer {
        /// 界面发的时候给的编号。
        tag: u64,
        /// 回应。
        result: Result<serde_json::Value, Refusal>,
    },
    /// 回答确认、提问被拒（[`Command::Answer`]）：调用编号、原因码、原话。`not_asking` 是已经答过、了结了。
    AnswerRefused {
        /// 问的那个会话（整个编号）。
        session: String,
        /// 调用编号。
        call: String,
        /// 原因码。
        reason: Option<String>,
        /// 核心的原话。
        message: String,
    },
    /// 会话列表变了（`sessions.changed`，核心 9-5）：整项换、删掉。
    SessionChanged(SessionChange),
    /// 配置变了（推来的 `config.changed`，哪个头、哪一层改的都算）：开着配置页的重读（「配置页」第 25 条）。
    ConfigChanged,
    /// 一条后台命令的输出（[`Command::Output`] 的回应）；读不了的（任务没了、是子代理）是 `None`。
    Output {
        /// 哪个会话派的。
        session: String,
        /// 任务编号。
        job: String,
        /// 读到的。
        output: Option<JobOutput>,
        /// 是接着读的一段（带 `after`）：接在已有的后面，不整份换。
        after: bool,
    },
    /// 人打的 `/dream` 正在整理：记忆流推来的一样（`memory.rs`，蓝图「整理记忆」）。
    Dream {
        /// 哪个会话订阅的。
        session: String,
        /// 推来的那一样。
        change: crate::dream::Change,
    },
}
