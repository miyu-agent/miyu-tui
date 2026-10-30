//! 连接（蓝图 `app.md` 的 `link` 块）：连着核心的一头，和核心给的资料：能做什么、工具给人看的名字、金额排最前的
//! 币种、默认的聊天模型、启动时认出的界面语言。

use crate::core::Core;
use crate::human::Human;

/// 连接。
pub struct Link {
    /// 连着核心的一头。
    pub core: Core,
    /// 每个模型、池收哪几种附件（`attachments.rs`，照 `model.list`）。
    pub abilities: crate::core::Abilities,
    /// 工具给人看的显示名（仓库 `resources/software/basesystem/human/`）。
    pub human: Human,
    /// 金额排最前的币种（`usage.currency`，连上时读）。
    pub currency: String,
    /// 默认的聊天模型（`uses.chat`）：还没开会话的照它认收哪几种附件。
    pub default_chat: Option<String>,
    /// 要过给人看的字了：连上头一次读到头的配置时要（排在读配置、人格列表后面，「连核心」第 9 条）。
    pub human_asked: bool,
    /// 读到的、画首页要用的几样，退出时记下、下次起来先照它画（`look.rs`）。
    pub look: super::look::Look,
    /// 读配置攒着合成一条问的（`config_asks.rs`）。
    pub config_asks: super::config_asks::ConfigAsks,
    /// 启动时照系统认出来的界面语言：`/language` 选回自动用它，框的第一行写它（蓝图「界面语言」）。
    pub system_language: crate::language::Language,
    /// 扩展包放的界面块（蓝图「界面块」，`app/widgets.rs`）。
    pub widgets: crate::widgets::Widgets,
    /// 开机启动核心：在后台跑 `miyu service`（配置页「开机启动」，`crate::service`）。
    pub service: crate::service::Service,
    /// 一轮结束时问了这个会话换算好的金额（`bill_refresh.rs`）：问的是哪个会话。
    pub bill_asked: Option<String>,
    /// 包的名字、在哪个组（`package.list`）：预设的功能照组、包分组时写组名（`features.rs`）。
    pub package_names: crate::features::Packages,
    /// 唤醒表里跑在已有会话上的循环（`schedules` 流，蓝图「循环」）：输入框上面、侧边栏照看着的会话挑。
    pub loops: Vec<crate::core::Loop>,
}
