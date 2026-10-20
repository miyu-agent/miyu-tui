//! 侧边栏的「压缩 N 次」「缓存断裂 N 次」（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条）。
//!
//! 断裂不靠命中率猜：核心每次请求记下这一次的前缀和上一次比变没变（`first_difference`），变了就是断了。
//! 只数意外的：压缩、撤销以后的那一次本来就会断，不算；压缩的摘要请求只带尾巴前面那一段，看到的比之前少，
//! 也不算。

/// 压过几次、意外断过几次缓存。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CacheWatch {
    /// 压缩了几次。
    pub compactions: u64,
    /// 意外断了几次。
    pub breaks: u64,
    /// 刚压缩、撤销过：下一次发出去的主请求本来就会断，不算。
    excused: bool,
    /// 之前的请求最多看到第几条。
    furthest: Option<u64>,
    /// 有没有能压缩的（核心 W-12）：订阅回应给的；之后发过一次主请求当有，压完当没有（核心在 events 流里推以前先这么顶着，
    /// 2026-10-20 和核心会话对的）。不知道的是 `None`，照有算。
    pub compactable: Option<bool>,
}

impl CacheWatch {
    /// 压缩了一次。
    pub fn compacted(&mut self) {
        self.compactions += 1;
        self.excused = true;
        self.compactable = Some(false);
    }

    /// 撤销了几轮。
    pub fn reverted(&mut self) {
        self.excused = true;
    }

    /// 发出去一次请求：看到第 `seen` 条为止，前缀变没变，是不是压缩的摘要请求（`summary`）。
    pub fn sent(&mut self, seen: u64, changed: bool, summary: bool) {
        // 摘要请求不算，也不用掉压缩给主请求的那一次免数：6-6 上起记录带 `compaction` 认得出；以前的日志没有，
        // 照「看到的比之前少」（往回看）认。
        if summary || self.furthest.is_some_and(|f| seen < f) {
            return;
        }
        if changed && !self.excused {
            self.breaks += 1;
        }
        self.compactable = Some(true);
        self.excused = false;
        self.furthest = Some(seen);
    }
}

#[cfg(test)]
mod tests {
    use super::CacheWatch;

    #[test]
    fn compactable_turns_on_after_a_request_and_off_after_compacting() {
        // 核心 W-12：订阅回应给的值；之后发过一次主请求当有，压完当没有（核心在 events 流里推以前先这么顶着）。
        let mut cache = CacheWatch::default();
        assert_eq!(cache.compactable, None);
        cache.sent(3, false, true);
        assert_eq!(cache.compactable, None, "摘要请求不算");
        cache.sent(3, false, false);
        assert_eq!(cache.compactable, Some(true));
        cache.compacted();
        assert_eq!(cache.compactable, Some(false));
    }
}
