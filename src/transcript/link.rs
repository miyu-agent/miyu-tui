//! 连核心的状态（从 `mod.rs` 挪出来：那里放不下了）。

/// 连核心的状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// 还在连。
    Connecting,
    /// 连上了。
    Ready,
    /// 断开了，正在重新连接（「连核心」第 7 条）。
    Reconnecting,
    /// 连不上：给人看的一句（第 8 条）。
    Down(String),
    /// 核心是人停的（核心 CS-5：`miyu core stop`、别的头的「停止」、托盘的「退出」），断了线不自己拉起：核心的那一句
    /// 「核心已停止」，首页下面一行「Enter 启动」（「连核心」第 9 条）。
    Stopped(String),
}

impl Link {
    /// 连不上（连不上的、人停的）时给人看的那一句，首页红字照它写。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Link::Down(reason) | Link::Stopped(reason) => Some(reason),
            _ => None,
        }
    }
}
