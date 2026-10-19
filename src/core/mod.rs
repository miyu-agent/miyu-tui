//! 连核心：找数据根、连上（给了 `MIYU_CORE_BIN` 的，没在跑就拉起来）、握手、开会话、订阅，然后收发。
//!
//! 走法照 `miyu ask`（`docs/blueprint/cli/ask.md`「怎么走」）。界面的主循环是同步的，这里在单独的线程里跑
//! 一个 tokio 运行时；两边只靠通道说话：界面发 [`Command`]，这里回 [`Update`]。
//!
//! 拉起核心只认 `MIYU_CORE_BIN`，不去 PATH 里找 `miyu`：装着旧版的机器上，PATH 里的 `miyu` 是旧版，
//! 给它 `core` 这个参数，它会把这个词当成一句话发给旧版的后台。

mod abilities;
mod asides;
mod awaiting;
mod backoff;
mod catalog;
mod config;
mod connect;
mod cost;
mod efforts;
mod kinds;
mod limits;
mod link;
mod links;
mod memory;
mod mermaid;
mod models;
mod output;
mod packages;
pub use packages::{PackageEntry, Progress};
mod page;
mod personas;
mod push;
mod replay;
mod request;
mod rpc;
mod serve;
mod sessions;
mod snapshot;
mod spawn;
mod switch;
mod take;
mod todos;
mod types;
mod undo;
mod update;
pub use update::Update;
mod upload;
mod widgets;
mod workspace;

use std::thread;

use tokio::sync::mpsc;

use backoff::Backoff;
use connect::connect;

pub use abilities::Abilities;
pub use catalog::CoreCommand;
pub use config::HeadConfig;
pub use cost::{Bill, Cost, UsageAsk, UsageKind, UsageRow};
pub use efforts::{EffortList, Efforts, wanted as wanted_models};
pub use kinds::{EndReason, Level, ToolStatus};
pub use limits::Limits;
pub use links::{Card, CardKind, blob_path, cards_dir};
pub use mermaid::{Marks, Rendered};
pub use models::{Choice, ChoiceState, Current};
pub use output::JobOutput;
pub use personas::{Persona, Preset, read_personas, read_presets};
pub use push::{
    Asking, Attached, Block, CallError, Compaction, Goal, GoalState, JobEnd, JobReason, JobStart,
    Push, Sender, Sent, Shown, Usage,
};
use rpc::Rpc;
pub use sessions::{
    Change as SessionChange, Deleting as DeletingSessions, SessionInfo,
    apply as apply_session_change, latest as latest_session,
};
pub use snapshot::{Snapshot, Spent};
pub use spawn::spawn;
pub use todos::TodoItem;
pub use types::{OutputRead, Refusal, Start};
pub use undo::{Report, UndoFile};

/// 界面要核心做的事。
#[derive(Debug)]
pub enum Command {
    /// 说一句话，带着附件（本机的文件，发之前先 `blob.put`；蓝图「输入框」第 12 条）。
    Send {
        /// 说的字。
        text: String,
        /// 附件：照先后。
        files: Vec<std::path::PathBuf>,
    },
    /// 打断在进行的这一轮。`send` 为真时排着队的消息接着发（两下 Esc），为假时退回来（Ctrl+C）。
    Interrupt {
        /// 排着队的消息接着发。
        send: bool,
    },
    /// 撤销最后一轮（`session.revert`，不写 `turn`）。
    Revert,
    /// 恢复最近一次撤销（`session.unrevert`）。
    Unrevert,
    /// 现在就压缩上下文（`session.compact`，施工 6-8），带着给摘要的要求。
    Compact(Option<String>),
    /// 开新会话（`/new`）：等第一句话再开（蓝图「斜杠命令」`/new`）。`keep`：旧的还有后台任务在跑，照样订阅着，等界面
    /// 说它的任务都报完了再退订（「后台命令、子代理和侧边栏」）；不然马上退订。
    New {
        /// 旧会话照样订阅着。
        keep: bool,
    },
    /// 切到别的会话（`/sessions`，蓝图「会话列表」第 4、5 条）：它成了主会话；原来的 `keep` 为真时照样订阅着（还忙着），
    /// 不然退订。没订阅着的带 `after: 0` 订阅，以前的事件补发过来。
    Open {
        /// 切到哪个会话。
        session: String,
        /// 原来那个照样订阅着。
        keep: bool,
    },
    /// 列出会话（`session.list`），交回 [`Update::Sessions`]。
    ListSessions,
    /// 置顶、取消置顶一个会话（`session.set_meta` 的 `pinned`），不管对着哪个会话。
    Pin {
        /// 哪个会话。
        session: String,
        /// 置顶。
        pinned: bool,
    },
    /// 删掉一个会话（`session.delete`），不管对着哪个会话。
    Delete(String),
    /// 要 `/model` 框里的一行行（`model.list`），交回 [`Update::Choices`]。
    ListChoices,
    /// 要一个网址的链接卡片（`link.preview`，核心 W-7）。
    LinkPreview(String),
    /// 读回一个 blob 存成文件（卡片的封面图、图标；`blob.get`，核心 W-6）。
    FetchBlob(String),
    /// 把一张 mermaid 图的源码交给核心画成 SVG（`mermaid.render`，核心 W-4）。
    RenderMermaid(String),
    /// 手动换的模型也记成新会话的默认：写个人设置的 `models.chat`（2026-10-02 项目主人定，`models.md`「头的约定」）。
    SetChat(String),
    /// `/effort`、底栏要的：每个模型有哪几级、配置的默认、配置键（`model.list`，核心 8-18 补）。给了的只要这几个模型
    /// （`models`，核心 V-6 上），`None` 要整份。
    ListEfforts(Option<Vec<String>>),
    /// 写个人设置里一个模型的思考强度：配置键、哪一级（`None` 是去掉这一项，回到供应商定）。
    SetEffort {
        /// 配置键（照 `facts.effort.key` 抄）。
        key: String,
        /// 哪一级。
        level: Option<String>,
    },
    /// `@` 文件列表问核心（`fs.list`、`fs.find`，核心 W-2）：哪个词问的、方法、参数。
    Files {
        /// 哪个词问的：回应照它认。
        word: crate::mention::Word,
        /// `fs.list` 或 `fs.find`。
        method: &'static str,
        /// 参数。
        params: serde_json::Value,
    },
    /// 这个会话换模型（`session.configure`，下一个回合开始生效，核心 8-10）：引用。还没开会话的记着，开会话时带上。
    Configure(String),
    /// 要模型资料（`model.list`），交回冷却着的最早什么时候恢复（[`Update::CoolingUntil`]，「配置与模型」第 7 条）。
    ListModels,
    /// 要这种语言的给人看的字（`human.get`，核心 W-1）：语言代码。回 [`Update::Human`]。
    FetchHuman(String),
    /// 界面语言写进个人设置（`ui.language`，`auto` 或者语言代码；蓝图「界面语言」）。
    SetLanguage(String),
    /// 写个人设置的配色（`tui.theme`，`/theme` 的框里定下的）。
    SetTheme(String),
    /// 另外订阅一个会话：子代理的会话（「后台命令、子代理和侧边栏」、「切进子会话」）。它推来的包成 [`Update::Elsewhere`]。
    Watch(String),
    /// 退订另外订阅着的一个会话。
    Unwatch(String),
    /// 命令对着哪个会话：切进了的子会话；`None` 是主会话（「切进子会话」第 3 条）。
    View(Option<String>),
    /// 停掉一个后台任务（`job.stop`）：任务表是哪个会话的（`None` 是命令正对着的那个），任务编号。子代理状态行列的是主会话的
    /// 任务表，切进了子会话也要对着主会话停（review B4，2026-10-11）。
    Stop {
        /// 派它的会话。
        session: Option<String>,
        /// 任务编号。
        job: String,
    },
    /// 读一条后台命令到这时为止的输出（`job.output`）：哪个会话派的、任务编号、怎么读。
    Output {
        /// 哪个会话派的。
        session: String,
        /// 任务编号。
        job: String,
        /// 要最后几行，还是接着读。
        read: OutputRead,
    },
    /// 切权限级别（`session.set_permission_level`）：切到这一级。会话还没开的记着，开了再发。
    Level(Level),
    /// 查用量（`usage.query`，`/usage`）。
    Usage(UsageAsk),
    /// 列人格（`persona.list`）：新会话的人格框用。
    ListPersonas,
    /// 列软件包（`package.list`，核心 9-1 上）：终端照 `tui` 那一项的 `state` 放输入历史。
    ListPackages,
    /// 下一个新会话用这个人格（开会话时 `session.create` 带上；会话开着时不管，开了不能换）。
    Persona(String),
    /// 列预设（`preset.list`，核心 P-2 上）：新会话的预设框用。
    ListPresets,
    /// 下一个新会话用这个预设，和 [`Command::Persona`] 一样。
    Preset(String),
    /// 要这个会话更早的一页（`view.page {before}`，核心 9-6 下）：往上滚到顶、`more` 是真的。
    Older(String),
    /// 开着的会话换工作区（`command.run "/workspace <路径>"`，核心 9-7 下）：打的路径、终端所在的目录（接相对的）。
    Workspace {
        /// 打的路径。
        path: String,
        /// 终端所在的目录。
        cwd: String,
    },
    /// 还没开会话时验一个目录（`fs.realpath` 再 `fs.list`）：打的路径、终端所在的目录。
    CheckDir {
        /// 打的路径。
        path: String,
        /// 终端所在的目录。
        cwd: String,
    },
    /// 新会话开在哪（`session.create` 的 `cwd`）：验过的目录；`None` 回到终端所在的目录。
    NewWorkspace(Option<String>),
    /// 一条斜杠命令交给核心办（`command.run`，核心 O-6）：`/stop`、`/clear`，原文照规范的名字。
    Run(String),
    /// 核心清单里的、终端不认的一条斜杠命令（命令模板、扩展声明的，核心 X-14）：人打的那一行原样交给 `command.run`；
    /// 还没开会话的先开会话（模板是当人说的一句发出去的）。被拒的回 [`Update::SlashRefused`]。
    Slash(String),
    /// 要核心认的斜杠命令（`command.catalog`）：带会话的只列这个会话里能用的。
    Catalog(Option<String>),
    /// 订阅界面块（`subscribe {stream: widgets}`，核心 X-15）：屏上有哪几个位置、正在看的会话、开着的整页；`None` 是不再
    /// 订阅。记在连接上，重连、掉队时照它重新订阅（蓝图「界面块」第 8 条）。
    Widgets(Option<crate::widgets::View>),
    /// 人打的 `/dream`（`memory.rs`）：`true` 是下一条斜杠命令交出去以前，订阅它对着的会话那一间的记忆流（还没开会话的开了再订，
    /// 开头那一批不漏）；`false` 是整理完了、退订。
    Dream(bool),
    /// 点了界面块里调方法的动作（`package.call`）：回应不看，拒了的回 [`Update::PackageCallRefused`]。
    PackageCall {
        /// 哪个包。
        package: String,
        /// 它的程序登记的方法。
        method: String,
        /// 参数：一个对象。
        params: serde_json::Value,
    },
    /// `/goal` 交给核心办（`command.run`，核心 X-8），带着后面的字；会话还没开时先开会话（蓝图「长期目标」第 3 条）。
    Goal(String),
    /// 要一段回顾（`session.recap`，`/recap`）。
    Recap,
    /// 改名（`session.set_meta`，`/rename`）：`None` 是去掉标题。
    Rename(Option<String>),
    /// 回答一次确认或提问（`session.answer`，核心 D-1）：问的那个会话（主会话的是 `None`）、调用编号、`decision` 或
    /// `answers`（连同 `reason`）那几格。
    Answer {
        /// 问的那个会话；`None` 是主会话。
        session: Option<String>,
        /// 调用编号。
        call: String,
        /// `{"decision": …}` 或 `{"answers": […]}`。
        body: serde_json::Value,
    },
    /// 不对着会话的一条请求，回应原样交回（[`Update::Answer`]）：配置页用（蓝图「配置页」第 23 条），不为每个方法再加一对命令。
    Ask {
        /// 界面自己的编号：回应照它认。
        tag: u64,
        /// 方法。
        method: &'static str,
        /// 参数。
        params: serde_json::Value,
    },
    /// 重做最后一轮（`session.redo`，`/redo`、`/edit`）：`text` 换开这一轮的那句字，`files` 换附件（空的是不要附件），
    /// 都是 `None` 的原样重来。
    Redo {
        /// 改过的字。
        text: Option<String>,
        /// 换了的附件：本机的文件，发之前先 `blob.put`。
        files: Option<Vec<std::path::PathBuf>>,
    },
}

/// 连着核心的这一头，界面拿着它发命令。克隆一份交给后台线程，做完了照样发（缩头像，`app/avatars.rs`）。
#[derive(Clone)]
pub struct Core {
    commands: mpsc::UnboundedSender<Command>,
}

impl Core {
    /// 发一个命令。连上之前发的排着，连上再发（`13-终端界面.md` 第九节「先画后连」）。
    pub fn send(&self, command: Command) {
        // 收的一头只在连核心的线程退出时关掉，那时已经报过「断开了」，丢了也无妨。
        if self.commands.send(command).is_err() {}
    }
}

#[cfg(test)]
mod tests;
