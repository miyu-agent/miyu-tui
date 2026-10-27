//! 出字的速度：底栏写最近一次请求的，`/status` 写这个会话里各次请求合起来的平均（蓝图 `tui.md`「会话状态 `/status`」），
//! 收尾那一行写这一轮的（「正文」，2026-10-26 项目主人）。

/// 出字的速度，每秒几个 token。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Speed {
    /// 最近一次请求的；还没有的是 `None`。
    pub last: Option<f64>,
    /// 各次请求加起来出了多少 token。
    output: u64,
    /// 各次请求加起来用了多少毫秒。
    ms: u64,
}

impl Speed {
    /// 一次请求出了 `output` 个 token、用了 `ms` 毫秒（`model.called`）；用时是 0 的不算。
    pub fn record(&mut self, output: u64, ms: u64) {
        if ms == 0 {
            return;
        }
        self.last = Some(output as f64 * 1000.0 / ms as f64);
        self.output += output;
        self.ms += ms;
    }

    /// 平均：一共出的 token ÷ 一共用的时间；还没有的是 `None`。
    pub fn average(&self) -> Option<f64> {
        (self.ms > 0).then(|| self.output as f64 * 1000.0 / self.ms as f64)
    }
}

impl super::Transcript {
    /// 一次主请求出了 `output` 个 token、用了 `ms` 毫秒：记进这个会话的、这一轮的。
    pub(super) fn sped(&mut self, output: u64, ms: u64) {
        self.speed.record(output, ms);
        self.turn_speed.record(output, ms);
    }

    /// 这一轮出字的速度写成收尾那一行接的一截（「 · 46 tok/s」）；这一轮一次都没记下的不写。
    pub(super) fn turn_rate(&self, texts: &crate::config::Texts) -> String {
        self.turn_speed.average().map_or_else(String::new, |rate| {
            let rate = format!("{rate:.0}");
            format!(
                "{}{}",
                crate::sheet::sep(),
                texts.speed.replace("{rate}", &rate)
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Speed;

    #[test]
    fn the_average_weighs_requests_by_their_time() {
        let mut speed = Speed::default();
        assert_eq!(speed.average(), None);
        speed.record(100, 1000);
        speed.record(300, 1000);
        speed.record(50, 0);
        assert_eq!(speed.last, Some(300.0), "用时是 0 的不算");
        assert_eq!(speed.average(), Some(200.0));
    }
}
