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

    /// 等到屏幕的各行合 `check`，交回那时的各行，最多 [`WAIT`]；等不到的说等的是 `what`，把屏幕打出来。
    pub fn wait_until(&mut self, what: &str, check: impl Fn(&[String]) -> bool) -> Vec<String> {
        let end = Instant::now() + WAIT;
        loop {
            let lines = self.lines();
            if check(&lines) {
                return lines;
            }
            assert!(
                Instant::now() < end,
                "等不到{what}，屏幕是：\n{}",
                lines.join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 等到 `texts` 每一串都在同一屏上，交回那时的各行：一帧可能分几次读到，等到了这一串就查同一屏的别处，读到半帧时
    /// 别处还是旧的（LiteMiyu f012974 在 CI 上红过的那种）。要查的几样都写进来，等到都在了再断言。
    pub fn wait_for_all(&mut self, texts: &[&str]) -> Vec<String> {
        let what = texts
            .iter()
            .map(|t| format!("「{t}」"))
            .collect::<Vec<_>>()
            .join("");
        self.wait_until(&what, |lines| texts.iter().all(|t| has(lines, t)))
    }

    /// 等到屏幕上有 `text`、`old` 收掉了，交回那时的各行：整页、整段换上来时新的先到了、旧的还没擦掉的半帧不算。
    pub fn wait_for_instead(&mut self, text: &str, old: &str) -> Vec<String> {
        self.wait_until(&format!("「{text}」换掉「{old}」"), |lines| {
            has(lines, text) && !has(lines, old)
        })
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

    /// 屏幕上有一轮的收尾行（[`done_in`]）。
    pub fn shows_done(&self) -> bool {
        done_in(&self.lines())
    }

    /// 等到一轮的收尾行出来，最多 [`WAIT`]。
    pub fn wait_for_done(&mut self) {
        let end = Instant::now() + WAIT;
        while !self.shows_done() {
            assert!(
                Instant::now() < end,
                "等不到收尾行，屏幕是：\n{}",
                self.lines().join("\n")
            );
            self.pump(Duration::from_millis(50));
        }
    }

    /// 同 [`Tui::wait_at_bottom_for`]，等的是一轮的收尾行。
    pub fn wait_at_bottom_for_done(&mut self) {
        let end = Instant::now() + WAIT;
        while !self.shows_done() {
            assert!(
                Instant::now() < end,
                "跟到底也等不到收尾行：\n{}",
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

/// 各行里有没有这串字（同 [`Tui::shows`]，给 [`Tui::wait_until`] 的条件用）。
pub fn has(lines: &[String], text: &str) -> bool {
    lines.iter().any(|l| l.contains(text))
}

/// 输入框里那一行的字，提示符后面的（同 [`Tui::input`]）。
pub fn input_in(lines: &[String]) -> String {
    lines
        .iter()
        .rev()
        .find(|l| l.contains('❯') && l.contains('│'))
        .and_then(|l| l.split_once('❯'))
        .map(|(_, rest)| rest.trim_end_matches('│').trim().to_string())
        .unwrap_or_default()
}

/// 有一轮的收尾行：开放权限的图标 `⏵⏵ ` 后面紧跟时间（底栏是 `⏵⏵ 开放权限`）。原来等 `▣  `（工作区的图标后面空两格），
/// LV-3 去掉了工作区一级。
pub fn done_in(lines: &[String]) -> bool {
    lines.iter().any(|l| {
        l.split("⏵⏵ ")
            .skip(1)
            .any(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    })
}
