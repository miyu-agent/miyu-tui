//! 出字的速度：底栏写最近一次请求的，`/status` 写这个会话里各次请求合起来的平均（蓝图 `tui.md`「会话状态 `/status`」）。

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
