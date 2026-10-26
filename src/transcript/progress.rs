//! 在跑的那一步的进度（蓝图 `tui.md`「时间线」第 8a 条，同核心视图投影 `projector/progress.rs`、网页 W-372）：
//! `tool.progress` 推来的字里一整行以 [`PREFIX`] 开头的，去了前缀、首尾空白，记进那一步的 `progress`。
//!
//! - 照调用编号攒半行：一段的末尾没有换行的留着等下一段。只留可能是进度行的：开头和前缀对不上的当场扔掉、丢到换行；
//!   最多留 [`HELD`] 个字节，内存不涨。
//! - 一段里几行取最后一条；空的不算；最长 [`LONGEST`] 个字。
//! - 结果到了（[`Lines::over`]），攒着的扔掉。

use std::collections::HashMap;

/// 进度行的开头：照字面，分大小写，后面带一个空格。
const PREFIX: &str = "miyu-progress: ";

/// 一行进度最长几个字（Unicode 字符），多的截掉。
const LONGEST: usize = 200;

/// 半行最多留几个字节：够前缀加 [`LONGEST`] 个四字节的字。
const HELD: usize = 1024;

/// 每个在跑的调用攒着的半行。
#[derive(Debug, Default, Clone)]
pub struct Lines {
    calls: HashMap<String, Line>,
}

impl Lines {
    /// 调用 `call` 推来一段字：交回这一段里收全的最后一行进度；没有的是 `None`。
    pub fn feed(&mut self, call: &str, text: &str) -> Option<String> {
        self.calls.entry(call.to_string()).or_default().feed(text)
    }

    /// 调用 `call` 有了结果：攒着的扔掉。
    pub fn over(&mut self, call: &str) {
        self.calls.remove(call);
    }
}

/// 一个调用攒着的半行。
#[derive(Debug, Default, Clone)]
struct Line {
    half: String,
    /// 这一行看出来不是进度行：丢到换行。
    skip: bool,
}

impl Line {
    fn feed(&mut self, text: &str) -> Option<String> {
        let mut latest = None;
        let mut rest = text;
        while let Some(end) = rest.find('\n') {
            self.take(&rest[..end]);
            if let Some(value) = self.line_ends() {
                latest = Some(value);
            }
            rest = &rest[end + 1..];
        }
        self.take(rest);
        latest
    }

    /// 接上一截（不带换行）：开头和前缀对不上的丢到换行，留着的不过 [`HELD`]。
    fn take(&mut self, piece: &str) {
        if self.skip {
            return;
        }
        let mut room = HELD.saturating_sub(self.half.len()).min(piece.len());
        while !piece.is_char_boundary(room) {
            room -= 1;
        }
        self.half.push_str(&piece[..room]);
        let n = self.half.len().min(PREFIX.len());
        if self.half.as_bytes()[..n] != PREFIX.as_bytes()[..n] {
            self.skip = true;
            self.half.clear();
        }
    }

    /// 一行收全了：是进度行的交回它的值。
    fn line_ends(&mut self) -> Option<String> {
        let line = std::mem::take(&mut self.half);
        if std::mem::take(&mut self.skip) {
            return None;
        }
        let value = line.strip_prefix(PREFIX)?.trim();
        (!value.is_empty()).then(|| value.chars().take(LONGEST).collect())
    }
}

#[cfg(test)]
mod tests;
