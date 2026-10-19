//! 等着回应、回应要另外办的请求：`serve.rs` 照请求编号记着，`asides.rs` 发不对着会话的请求时写上。

/// 等着回应、回应要另外办的请求（记着请求的编号）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Awaiting {
    /// 撤销：回应里给人看的几样交给界面。
    Revert,
    /// 恢复：同上。
    Unrevert,
    /// 说一句话：成了不用管，拒了要撤掉先画上的那句。
    Send,
    /// 重做：同上。
    Redo,
    /// 订阅另一个会话：回应里的限额交给界面，说明是哪个会话。
    Watch(String),
    /// 读一条后台命令的输出：哪个会话、任务编号、是不是接着读的一段。
    Output(String, String, bool),
    /// 要回顾：交回的是上一句的，界面照回应画（写成了的照推送画）。
    Recap,
    /// 改名：成了弹一句（`None` 是去掉标题）。
    Rename(Option<String>),
    /// 带 `after` 订阅：回应到了算补完（「会话列表」第 5 条）。
    Replay(String),
    /// 重连以后带 `after` 重订主会话（review B7，2026-10-11）：回应照补完办，只是不算接上一个老会话（工作区不另写一句）。
    Rejoin(String),
    /// 要最新一页（核心 9-6 下）：到了画进去，再带 `after` 订阅；核心旧的照旧补整份。
    Page(String),
    /// 要更早的一页：到了交给界面拼在前面。
    Older(String),
    /// 列出会话：回应交给界面。
    List,
    /// 订阅会话列表（核心 9-5）：回应的整张列表交给界面；老核心不认的不说。
    SessionsStream,
    /// 订阅软件包列表（只听工具目录变了，`packages.rs`）：回应不用；老核心不认的不说。
    PackagesStream,
    /// 读界面语言：回应交给界面。
    UiLanguage,
    /// 要给人看的字：回应交给界面。
    Human,
    /// 要模型资料：冷却着的最早恢复交给界面。
    Models,
    /// 要 `/model` 框里的一行行。
    Choices,
    /// 要 `/effort` 框里的几级。
    Efforts,
    /// 要一个网址的卡片。
    LinkPreview(String),
    /// 一段段读一个 blob：哪个、读回来的。
    Blob(String, Vec<u8>),
    /// 画一张 mermaid 图：哪份源码。
    Mermaid(String),
    /// `@` 文件列表问的：哪个词。
    Files(crate::mention::Word),
    /// 换模型：成了交给界面（引用）。
    Configure(String),
    /// 配置页的一条请求：界面给的编号。
    Ask(u64),
    /// 列人格：回应交给界面。
    Personas,
    /// 列预设：回应交给界面。
    Presets,
    /// 列软件包：回应交给界面。
    Packages,
    /// 查用量（`/usage`）：哪一样。
    Usage(super::UsageKind),
    /// 斜杠命令（`command.run`）：回执交给界面。
    CommandRun,
    /// 核心清单里的一条斜杠命令（[`super::Command::Slash`]）：办了照 `CommandRun`，被拒另报。
    Slash,
    /// 要核心认的斜杠命令。
    Catalog,
    /// 订阅界面块：回应交给界面。
    Widgets,
    /// 订阅记忆流（`/dream`）：回应、拒绝都不用看。
    Dream,
    /// 整理时取记忆的正文：哪个会话。
    DreamTexts(String),
    /// 叫停整理：回应、拒绝都不用看（整理那条 `/dream` 自己回「整理已中断」）。
    DreamStop,
    /// 调界面块的程序登记的方法：拒了的告诉界面。
    PackageCall,
    /// 回答确认、提问：问的会话、调用编号（拒了要告诉界面）。
    Answer(String, String),
}
