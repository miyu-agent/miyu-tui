//! 闲下来放掉画面的缓冲（蓝图「每一帧」）：上一帧画完过了 `screen_park_ms`（`layout.json`）没再画，就 [`Screen::park`]。
//! 主循环照这里等事件：到点了放掉、接着等，不为它画一帧。

use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use ratatui::backend::Backend;

use super::Screen;

/// 什么时候放掉。
#[derive(Debug)]
pub struct Parking {
    /// 画完过多久放；`None` 是不放（配成 0），一直留着。
    after: Option<Duration>,
    /// 到点放；放过了、还没画过是 `None`。
    due: Option<Instant>,
}

impl Parking {
    /// 画完过 `ms` 毫秒放掉；0 是不放。
    pub fn new(ms: u64) -> Self {
        Self {
            after: (ms > 0).then(|| Duration::from_millis(ms)),
            due: None,
        }
    }

    /// 在 `at` 画了一帧：从这时起重新算。
    pub fn drawn(&mut self, at: Instant) {
        self.due = self.after.map(|after| at + after);
    }

    /// 到点放的时刻；不放、放过了是 `None`。
    #[cfg(test)]
    pub const fn due(&self) -> Option<Instant> {
        self.due
    }

    /// 等下一件事：`deadline` 是主循环自己要醒来的时刻，`recv` 照给的时长等一件事。到了放的点、主循环自己还不用醒的，
    /// 放掉缓冲接着等。交回等到的（`None` 是主循环自己的点到了）；通道断了交回错。
    ///
    /// # Errors
    /// 通道断了（`RecvTimeoutError::Disconnected`）。
    pub fn wait<T, B: Backend>(
        &mut self,
        screen: &mut Screen<B>,
        deadline: Option<Instant>,
        mut recv: impl FnMut(Duration) -> Result<T, RecvTimeoutError>,
    ) -> Result<Option<T>, RecvTimeoutError> {
        loop {
            let now = Instant::now();
            let until = deadline.into_iter().chain(self.due).min();
            let wait = until.map_or(Duration::from_secs(3600), |d| {
                d.saturating_duration_since(now)
            });
            match recv(wait) {
                Ok(message) => return Ok(Some(message)),
                Err(RecvTimeoutError::Timeout) => {
                    let now = Instant::now();
                    if self.due.is_some_and(|due| due <= now) {
                        self.due = None;
                        screen.park();
                        if deadline.is_none_or(|d| d > now) {
                            continue;
                        }
                    }
                    return Ok(None);
                }
                Err(RecvTimeoutError::Disconnected) => return Err(RecvTimeoutError::Disconnected),
            }
        }
    }
}

#[cfg(test)]
mod tests;
