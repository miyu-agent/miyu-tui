//! 斜杠命令：清单住在 `resources/commands.json`，一条一个名字、一句说明、做什么。核心认的（命令模板、扩展声明的，
//! `command.catalog`，核心 X-14）另放一份 [`Commands::core`]，接在终端自己的后面，同名的照终端自己的。
//!
//! 加命令只登记：写进清单，`run` 选下面几种之一；要新的一种才动代码。

use serde::Deserialize;

/// 全部斜杠命令，照清单里的先后。
#[derive(Debug, Clone, Deserialize)]
pub struct Commands {
    /// 一条条。
    pub commands: Vec<Spec>,
    /// 命令被拒时，这几种原因码弹核心的原话（带 `data.why` 的照它），不写进正文（蓝图「长期目标」第 2 条）。
    #[serde(default)]
    pub hinted_refusals: Vec<String>,
    /// 核心认的、终端自己没有的那几条（`command.catalog` 读来的，`app/catalog.rs` 放进来）：接在后面列、照样能打。
    #[serde(skip)]
    pub core: Vec<Spec>,
    /// 核心的清单到过了。没到之前认不出的 `/名字 字` 也交给核心认，不当一句话发给她（2026-10-30：刚连上就打 `/loop 10m`，
    /// 清单还没回来，整句当话发出去了）。
    #[serde(skip)]
    pub core_known: bool,
    /// 能用 `$` 点名的技能（`command.catalog` 顶层的 `skills`，核心 SK-1，发版前 check 26）：不在命令列表里。
    #[serde(skip)]
    pub skills: Vec<crate::core::CoreSkill>,
    /// 总线上可用的名字（`bus.list`，核心 BUS-2）：写了 `needs` 的命令照它列（语音，设计 20）。
    #[serde(skip)]
    pub bus: Vec<String>,
}

/// 一条命令。
#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    /// 名字，不带 `/`。
    pub name: String,
    /// 别名：打它一样执行，筛的时候也算（`/undo` 的 `rewind`）。
    #[serde(default)]
    pub aliases: Vec<String>,
    /// 列表里名字后面那一句（终端自己的说法；核心清单也列了这个名字的照核心的，见 [`Spec::said`]）。
    pub summary: String,
    /// 核心清单里同名那一条的说明（`command.catalog` 的 `summary`，照连接的语言）：属于核心的命令（`/stop`、`/clear`、`/goal`……）
    /// 照它写，终端不另写一份（2026-11-13 主人验收 1113：`/stop` 写的还是终端自己那句旧说明，核心 11-09 早改了）。
    #[serde(skip)]
    pub core_summary: Option<String>,
    /// 做什么。
    pub run: Run,
    /// 名字后面空一格能接参数（`/compact 要求`，蓝图「斜杠命令列表」第 5 条）。
    #[serde(default)]
    pub args: bool,
    /// 名字后面跟什么的提示（核心清单里模板的 `argument-hint`，例如 `<路径>`）：列表里名字后面暗色写着。
    #[serde(default)]
    pub argument: Option<String>,
    /// 扩展的整页带的命令：打了就开这一页（核心清单的 `opens`，蓝图「界面块」第 10 条）。
    #[serde(skip)]
    pub opens: Option<crate::widgets::PageRef>,
    /// 几种写法（核心清单的 `usage`，O-6 四补）：`/help` 一行一种；命令列表照旧一条一行（2026-10-17 项目主人定）。
    #[serde(skip)]
    pub usage: Vec<(String, String)>,
    /// 跟着核心的包的命令（`/goal` 跟着长期目标那个包）：核心清单回来了却没列它（包卸掉了，核心 BS-1下下），列表里不列，
    /// 打了交给核心说不认得（同别的不存在的命令，不放回输入框）。
    #[serde(default)]
    pub follows_core: bool,
    /// 照上一条藏起来了（[`Commands::take_core`]）。
    #[serde(skip)]
    pub gone: bool,
    /// 总线上要有这个名字可用才列、才能打（`/voice-input` 要 `stt`，设计 20 第七节）。
    #[serde(default)]
    pub needs: Option<String>,
}

/// 回车时输入框里那一行是什么（蓝图「斜杠命令列表」第 4、5 条）。
#[derive(Debug)]
pub enum Line<'a> {
    /// 一条命令，带着名字后面的字（没写的是 `None`）。
    Command(&'a Spec, Option<&'a str>),
    /// 像命令，终端没有这个命令：交给核心，由它说行不行（`app/catalog.rs`）。
    Unknown,
    /// 一句话，发给她。
    Talk,
}

/// 命令做什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Run {
    /// 撤销上一轮（`session.revert`）。
    Revert,
    /// 恢复刚才的撤销（`session.unrevert`）。
    Unrevert,
    /// 这一轮原样重来一遍（`session.redo`，蓝图「斜杠命令」`/redo`）。
    Redo,
    /// 编辑上一句：放进输入框改，回车照改过的重来（蓝图「输入框」第 13 条）。
    Edit,
    /// 现在就压缩上下文（`session.compact`），后面的字是给摘要的要求。
    Compact,
    /// 要一段回顾（`session.recap`，蓝图「回顾」）。
    Recap,
    /// 改名（`session.set_meta`）、只打命令让模型重起标题（`session.retitle`；蓝图「改名」）。
    Rename,
    /// 长期目标：交给核心（`command.run`，蓝图「长期目标」）。
    Goal,
    /// 会话列表，切过去（蓝图「会话列表 `/sessions`」）。
    Sessions,
    /// 列这个会话里你说的话，跳过去（蓝图「跳到说过的话」）。
    Jump,
    /// 新会话用哪个人格：开会话以前开人格框（蓝图「新会话：人格、工作区」第 2 条）。
    Persona,
    /// 新会话用哪个预设：开会话以前开预设框（同上第 3 条）。
    Preset,
    /// 换工作区：带路径的直接换，不带的开工作区框（同上第 4 条，核心 9-7 下）。
    Workspace,
    /// 换这个会话的模型（蓝图「配置与模型」第 1 条）。
    Model,
    /// 全屏的配置页（蓝图「配置页」）。
    Config,
    /// 配置页直接进「供应商和模型」（蓝图「配置页」第 1 条）。
    Connect,
    /// 换正在用的模型的思考强度（蓝图「配置与模型」思考强度）。
    Effort,
    /// 开新会话：清界面回首页，第一句话时再开（蓝图「斜杠命令」`/new`）。
    New,
    /// 删除当前会话，回首页（发版前 check 12）。
    Delete,
    /// 清空上下文，会话不变（核心 `command.run` 的 `/clear`，蓝图「正文」第 9 条）。
    Clear,
    /// 全停：打断这一轮、排着的留着、停掉后台的（核心 `command.run` 的 `/stop`，蓝图「斜杠命令」`/stop`）。
    Stop,
    /// 这台机器上的用量和花费（`usage.query`，蓝图「配置与模型」第 5 条）。
    Usage,
    /// 这个会话的状态（蓝图「会话状态 `/status`」）。
    Status,
    /// 这个会话（还没开的照选的人格、预设）能用哪些工具（`/tool-list`，`tool_list.rs`）。
    ToolList,
    /// 这个会话的上下文一块块各占多少、按用途的累计（`/context`，`context_box.rs`）。
    Context,
    /// 这个会话的压缩触发线（`/compact-line 70`、`off`，核心 CT-1，`compact_line.rs`）。
    CompactLine,
    /// 复制她上一轮的回答（蓝图「斜杠命令」`/copy`）。
    Copy,
    /// 换下一套主题。
    Theme,
    /// 换下一套图标（蓝图「图标」）。
    Icons,
    /// 这次运行贴不贴图（`/images on|off`，蓝图「图片、公式和 mermaid 图」第 10 条）。
    Images,
    /// 退出程序。
    Quit,
    /// 帮助：输入框上面开一个框，列命令和按键（蓝图「帮助 `/help`」）。
    Help,
    /// 在中英文之间切换界面语言（蓝图「界面语言」）。
    Language,
    /// 把正在看的会话导出来（`/export`，蓝图「导出 `/export`」）。
    Export,
    /// 说话代替打字：开始听，再打一次停（`stt.record`、`stt.stop`，设计 20 第七节）。
    VoiceInput,
    /// 朗读她上一轮的回答，在念的再打一次停（`tts.play`、`tts.stop`）。
    ReadAloud,
    /// 开关自动朗读（只在这个终端里）。
    AutoRead,
    /// 核心认的命令（命令模板、扩展声明的）：整行交给核心（`command.run`，核心 X-14）。
    Core,
}

impl Spec {
    /// 列表、`/help` 里名字后面写的那一句：核心清单列了这个名字的照核心的说明，别的照终端自己的。
    pub fn said(&self) -> &str {
        self.core_summary.as_deref().unwrap_or(&self.summary)
    }
}

impl Commands {
    /// 照打的字筛：名字（或别名）以它开头的排前面，照清单的先后；别的模糊对得上的（`fuzzy.rs`，2026-10-18）排后面，越像越前。
    pub fn filter(&self, typed: &str) -> Vec<&Spec> {
        let typed = typed.to_lowercase();
        let (mut starts, mut rest) = (Vec::new(), Vec::new());
        for spec in self.all() {
            let names = std::iter::once(&spec.name).chain(&spec.aliases);
            if names.clone().any(|n| n.starts_with(&typed)) {
                starts.push(spec);
            } else {
                rest.push(spec);
            }
        }
        let fuzzy = crate::fuzzy::rank(&typed, rest, |spec| {
            std::iter::once(spec.name.as_str())
                .chain(spec.aliases.iter().map(String::as_str))
                .collect()
        });
        if typed.is_empty() {
            return starts;
        }
        starts.extend(fuzzy);
        starts
    }

    /// 名字或别名正好是 `name` 的那一条。
    pub fn find(&self, name: &str) -> Option<&Spec> {
        self.all()
            .find(|s| s.name == name || s.aliases.iter().any(|a| a == name))
    }

    /// 全部：终端自己的在前，核心认的接在后面。要总线上的名字、那个名字不可用的不算。
    pub fn all(&self) -> impl Iterator<Item = &Spec> + Clone {
        self.commands
            .iter()
            .filter(|s| !s.gone && s.needs.as_ref().is_none_or(|n| self.bus.contains(n)))
            .chain(&self.core)
    }

    /// 照核心认的那一份换上（`command.catalog` 的回应）：终端自己有的名字、别名不要，能接参数。终端自己有、又交给核心办
    /// 的同名命令（`/goal`），参数提示、几种写法照核心的（2026-10-17 项目主人：菜单照 shell 的语法提要，两个头别各拼各的）。
    pub fn take_core(&mut self, listed: &[crate::core::CoreCommand]) {
        self.core_known = true;
        for spec in &mut self.commands {
            let same = listed.iter().find(|c| c.name == spec.name);
            spec.gone = spec.follows_core && same.is_none();
            spec.argument = same.and_then(|c| c.argument.clone());
            spec.core_summary = same.map(|c| c.summary.clone()).filter(|s| !s.is_empty());
            spec.usage = same.map(|c| c.usage.clone()).unwrap_or_default();
        }
        let own = |name: &str| {
            self.commands
                .iter()
                .any(|s| s.name == name || s.aliases.iter().any(|a| a == name))
        };
        let core = listed
            .iter()
            .filter(|c| !own(&c.name) && !c.aliases.iter().any(|a| own(a)))
            .map(|c| Spec {
                name: c.name.clone(),
                aliases: c.aliases.clone(),
                summary: c.summary.clone(),
                core_summary: None,
                run: Run::Core,
                args: true,
                argument: c.argument.clone(),
                opens: c.opens.clone(),
                usage: c.usage.clone(),
                follows_core: false,
                gone: false,
                needs: None,
            })
            .collect();
        self.core = core;
    }
}

impl Commands {
    /// 回车时这一行是什么：`/名字` 是命令；能带参数的命令后面空一格接着的字是参数；别的后面跟了字的是一句话。
    pub fn read<'a>(&'a self, text: &'a str) -> Line<'a> {
        let Some(name) = typed(text) else {
            return Line::Talk;
        };
        // 名字后面的字：`typed` 认过的名字就在 `/` 后面。
        let words = text[1 + name.len()..].trim();
        // 藏起来的那几条带着参数也交给核心：不当一句话发给她（核心 BS-1下下回 `unknown_command`）。
        let gone = self.commands.iter().any(|s| s.gone && s.name == name);
        match (self.find(name), words.is_empty()) {
            (Some(spec), true) => Line::Command(spec, None),
            (Some(spec), false) if spec.args => Line::Command(spec, Some(words)),
            (None, true) => Line::Unknown,
            (None, false) if gone || !self.core_known => Line::Unknown,
            _ => Line::Talk,
        }
    }
}

/// 命令列表要不要开、照什么筛：像命令名，名字后面还没打空格（打了空格是在写参数，名字已经打全了）。
pub fn menu_typed(text: &str) -> Option<&str> {
    let name = typed(text)?;
    let after = &text[1 + name.len()..];
    after.is_empty().then_some(name)
}

/// 输入框里的字要不要当命令：`/` 开头，后面像个命令名（英文字母打头，只有英文字母、数字、`-`、`_`，中间最多一个
/// `:`；刚打一个 `/` 也算）。`/你吃了吗`、路径这些不算（`tui.md`「斜杠命令列表」第 1、4 条）。`:` 给 MCP 服务的提示词
/// （核心 X-39：`<服务名>:<提示词名>`，同 Claude Code，例如 `/github:review-pr 123`；同网页：两个 `:` 不算）。打到
/// 一半、`:` 在最后的（`/github:`）也算，列表不在打字中途收起又弹出来。交回命令名。
pub fn typed(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('/')?;
    // 名字紧挨着 `/`：`/ compact` 这种的名字是空的。
    let name = rest.split(char::is_whitespace).next().unwrap_or_default();
    let starts = name.chars().next().is_none_or(|c| c.is_ascii_alphabetic());
    let rest_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'));
    let colons = name.matches(':').count() <= 1;
    (starts && rest_ok && colons).then_some(name)
}

#[cfg(test)]
mod tests;
