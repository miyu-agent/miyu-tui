//! 协议里几样有固定取值的字：权限级别、工具结果的状态、一轮结束的原因。读推送时就换成类型，
//! 之后代码里只比类型，不拿字符串比（`00-设计理念.md` 第四节「依赖与接口的规矩」第 5 条）。

use serde::Deserialize;

/// 实际的权限级别：完全放开，或者开着只读（`kernel/events-bodies.md` 的 `permission`）。工作区那一级去掉了（LV-3，
/// 2026-11-09 项目主人定：只要只读和完全放开两种），旧的 `workspace` 读成完全放开。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// 完全放开（界面上叫「开放权限」）；旧的 `workspace` 也读成它。
    #[serde(alias = "workspace")]
    Full,
    /// 只读。
    ReadOnly,
}

impl Level {
    /// 协议里常用的那一级的写法：`full`，旧的 `workspace` 也当 `full`（核心 LV-1 一样读）；认不出的是 `None`。
    pub fn parse(text: &str) -> Option<Level> {
        match text {
            "full" | "workspace" => Some(Level::Full),
            _ => None,
        }
    }

    /// 切到这一级要给 `session.set_permission_level` 写的几格：完全放开写 `level: full` 和关掉只读；只读只开只读（蓝图
    /// `tui.md`「权限级别」第 2 条）。不再发 `workspace`（LV-3）。
    pub fn permission(self) -> serde_json::Value {
        match self {
            Level::Full => serde_json::json!({"level": "full", "read_only": false}),
            Level::ReadOnly => serde_json::json!({"read_only": true}),
        }
    }
}

/// 工具结果的状态（`tool.result` 的 `status`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// 成了。
    Ok,
    /// 出错。
    Error,
    /// 没做：被拒了。
    Denied,
    /// 打断了。
    Cancelled,
    /// 跳过了。
    Skipped,
    /// 新版本才有的取值：照「成了」画。
    Other,
}

impl ToolStatus {
    /// 照协议的写法读。
    pub fn parse(text: &str) -> ToolStatus {
        match text {
            "ok" => ToolStatus::Ok,
            "error" => ToolStatus::Error,
            "denied" => ToolStatus::Denied,
            "cancelled" => ToolStatus::Cancelled,
            "skipped" => ToolStatus::Skipped,
            _ => ToolStatus::Other,
        }
    }
}

/// 一轮结束的原因（`turn.ended` 的 `reason`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndReason {
    /// 照常结束。
    Completed,
    /// 打断了。
    Interrupted,
    /// 出错了。
    Error,
    /// 别的原因，照原样写出来。
    Other(String),
}

impl EndReason {
    /// 照协议的写法读。
    pub fn parse(text: &str) -> EndReason {
        match text {
            "completed" => EndReason::Completed,
            "interrupted" => EndReason::Interrupted,
            "error" => EndReason::Error,
            other => EndReason::Other(other.to_string()),
        }
    }
}
