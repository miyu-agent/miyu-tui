//! 出字的速度：底栏写最近一次请求的，`/status` 写这个会话里各次请求合起来的平均（蓝图 `tui.md`「会话状态 `/status`」），
//! 收尾那一行写这一轮的（「正文」，2026-10-26 项目主人）。

/// 首字到结束不到这么多毫秒的，当是网关攒齐了一口气发过来的，这一次没有速度（核心 `miyu_view::status::BURST_MS`，2026-11-11
/// 项目主人报 magpie 上 haiku 写出 1249500 tok/s：首字和结束只差几毫秒）。终端不依赖 `miyu-view`，照抄它的数，两处要一样。
const BURST_MS: u64 = 100;
/// 算出来每秒超过这么多个 token 的同样不算（核心 `BURST_RATE`；真流式的 p95 在每秒 260 到 3400 个之间）。
const BURST_RATE: u64 = 5_000;

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
    /// 一次请求出了 `output` 个 token、首字到结束用了 `ms` 毫秒（`model.called`）。一口气到的（[`BURST_MS`]、[`BURST_RATE`]）
    /// 不算：不写进平均，也不换掉上一次的数。
    pub fn record(&mut self, output: u64, ms: u64) {
        if ms < BURST_MS || output.saturating_mul(1000) > BURST_RATE.saturating_mul(ms) {
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

    /// 第 42 条（2026-11-11 项目主人：magpie 上 haiku 写出 1249500 tok/s）：一口气到的那一次不算，不换掉上一次、不进平均；
    /// 快但确实流过来的照算。同核心 `miyu_view::Speed::of` 的那几条例子。
    #[test]
    fn a_reply_that_arrived_all_at_once_has_no_speed() {
        let mut speed = Speed::default();
        speed.record(400, 2000);
        speed.record(2499, 5);
        assert_eq!(speed.last, Some(200.0), "首字到结束 5 毫秒：攒齐了才发的");
        speed.record(900, 100);
        assert_eq!(speed.last, Some(200.0), "每秒九千个的不算");
        speed.record(3000, 1000);
        assert_eq!(speed.last, Some(3000.0), "快但确实流过来的照算");
        speed.record(5000, 1000);
        assert_eq!(speed.last, Some(5000.0), "正好每秒五千个的照算");
        assert_eq!(speed.average(), Some(8400.0 * 1000.0 / 4000.0));
    }
}
