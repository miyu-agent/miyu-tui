//! 会话卡住了没有（蓝图 `tui.md`「框下面那一行」用量那一格，反馈 1102 第 22 条）：框下面那一格照它变红、给「压缩」。

use super::Transcript;

/// 卡住的两种。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stuck {
    /// 最近一次压缩失败了、或者自动压缩暂停了，后来没压好、没清空过，用量到了压缩线：`used`、压缩线。
    Failed { used: u64, line: u64 },
    /// 用量大于窗口（换到了窗口小的模型这类）：`used`、窗口。
    Over { used: u64, window: u64 },
}

impl Transcript {
    /// 卡住了是哪一种；两样都是的照压缩失败。没卡住是 `None`。
    pub fn stuck(&self) -> Option<Stuck> {
        let used = self.context;
        let failed = self
            .limits
            .compaction_line
            .filter(|line| self.compaction_trouble && used >= *line)
            .map(|line| Stuck::Failed { used, line });
        let over = self
            .limits
            .window
            .filter(|window| used > *window)
            .map(|window| Stuck::Over { used, window });
        failed.or(over)
    }
}
