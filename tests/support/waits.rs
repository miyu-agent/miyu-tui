//! 等屏幕上出现某样东西的几种等法（从 `mod.rs` 拆出来，那边过了 500 行）：照字等、照整行等、一路跟到底等、照字节到的时刻等。

use std::time::{Duration, Instant};

use super::{Tui, WAIT};

impl Tui {
    /// 等到屏幕上出现这串字，最多 [`WAIT`]；等不到的把屏幕打出来再报错。
    pub fn wait_for(&mut self, text: &str) {
        let end = Instant::now() + WAIT;
        while !self.shows(text) {
            assert!(
                Instant::now() < end,
                "等不到「{text}」，屏幕是：\n{}",
                self.lines().join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 一路按 `Ctrl+End` 跟到底，等到屏幕上出现 `text`（长回答的最后一行、这一轮的收尾行）。长过视口的回答写着时正文停在
    /// 它的开头（蓝图「正文」第 1 条）：慢的机器上回答分几次到、中间画过一帧，写完了也照旧停在开头，等不到最后一行（11-05、
    /// 11-06 macOS CI 三次等不到「第60行」）。要用长回答把正文撑高的测试，接着往下做之前都照这个等。
    pub fn wait_at_bottom_for(&mut self, text: &str) {
        let end = Instant::now() + WAIT;
        while !self.shows(text) {
            assert!(
                Instant::now() < end,
                "跟到底也等不到「{text}」：\n{}",
                self.lines().join("\n")
            );
            self.send(b"\x1b[1;5F");
            self.pump(Duration::from_millis(50));
        }
    }

    /// 等到屏幕上有一整行（去掉两头的空白）就是 `text`，最多 [`WAIT`]。别处带着这几个字的不算：比如首页的工作目录放不下时
    /// 以 `…` 打头（macOS 的临时目录很长），等她吐出的那个 `…` 就要只认整行。
    pub fn wait_for_line(&mut self, text: &str) {
        let end = Instant::now() + WAIT;
        while !self.lines().iter().any(|l| l.trim() == text) {
            assert!(
                Instant::now() < end,
                "等不到整行「{text}」，屏幕是：\n{}",
                self.lines().join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 等到屏幕上出现这串字，交回带出它的那一块字节到的时刻：一块一块地看，不等满一拍（量首帧用）。
    pub fn wait_for_at(&mut self, text: &str) -> Instant {
        let end = Instant::now() + WAIT;
        while !self.shows(text) {
            let left = end.saturating_duration_since(Instant::now());
            let data = self.bytes.recv_timeout(left).unwrap_or_else(|_| {
                panic!("等不到「{text}」，屏幕是：\n{}", self.lines().join("\n"))
            });
            let at = Instant::now();
            self.feed(&data);
            if self.shows(text) {
                return at;
            }
        }
        Instant::now()
    }
}
