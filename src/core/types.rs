//! 连核心那一层交给界面的几个小类型（从 `mod.rs` 拆出来，2026-10-15）：怎么读后台命令的输出、拒绝、启动时进哪个会话。

/// 怎么读一条后台命令的输出（`job.output`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRead {
    /// 要最后几行。
    Tail(usize),
    /// 从第 `size` 个字节接着读；`wait`：没有新的、还在跑，等长了、结束了再答（核心 V-6 上）。
    After { size: u64, wait: bool },
}

/// 核心拒了一条请求：原因码（`data.reason`，没有的是 `None`）、原话，和 `data` 整个（`config_conflict` 的 `current` 在里面）。
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    /// 原因码。
    pub reason: Option<String>,
    /// 核心照握手时的语言说的原话。
    pub message: String,
    /// 错误的 `data`。
    pub data: serde_json::Value,
}

/// 启动时进哪个会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// 照配置 `ui.startup`：`recent` 的进最近的那个，别的等第一句话再开。
    Usual,
    /// 进指定的会话（`--resume`）。
    Resume(String),
    /// 不进任何会话、不看 `ui.startup`（`--page config`，蓝图「配置页」第 1 条）。
    Bare,
}

/// 这一头是什么（握手的 `head.kind`，核心 VN-1）：核心把它记在人说的话、打的命令上（`by.head`），AI 照它知道话从哪来。
/// 全屏的终端界面是 `tui`，终端无缝集成（`miyu-tui shellhook …` 起的每一条连接）是 `shellhook`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    /// 全屏的终端界面。
    Tui,
    /// 终端无缝集成。
    Shellhook,
}

impl Head {
    /// 握手时写的 `head.kind`。
    pub fn kind(self) -> &'static str {
        match self {
            Head::Tui => "tui",
            Head::Shellhook => "shellhook",
        }
    }
}
