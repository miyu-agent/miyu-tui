//! 一次请求的用量（`model.called` 的 `usage`，`03-事件模型.md` 第三节）。

/// 一次请求的用量，四项都是 token 数（`03-事件模型.md` 第三节「usage」）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// 没命中缓存的输入。
    pub uncached: u64,
    /// 缓存读取。
    pub cache_read: u64,
    /// 缓存写入。
    pub cache_write: u64,
    /// 输出。
    pub output: u64,
    /// 辅助请求（回顾这类）用的 token，输入输出合在一起：累计里算它，缓存命中率、输入输出的拆分不算（蓝图「回顾」第 5 条）。
    /// 核心推来的一次请求里总是 0，只在界面这一头累加。
    pub aux: u64,
}

impl Usage {
    /// 这一次请求的全部输入：没命中 + 命中 + 写进缓存（照 `miyu ask` 的算法）。
    pub fn input(&self) -> u64 {
        self.uncached + self.cache_read + self.cache_write
    }
}
