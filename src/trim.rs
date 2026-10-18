//! 正文少了一大截以后把堆里空着的还给系统（蓝图「每一帧」，2026-10-19 项目主人「注意内存释放」）：收起点开的大块、换会话、
//! 清空以后，glibc 把放掉的留在自己手里（量过：抓三个整页全点开再收起、开新会话，终端多 0.6–0.8 MiB 不还，`malloc_trim`
//! 一下就回来）。正文的行数比攒下的最多时少了 `layout.json` 的 `trim_rows` 行以上，过 `trim_after_ms` 在后台线程里还一次
//! （要走一遍堆，不挡主循环）。只在 glibc 上有用：macOS、Windows 的分配器自己会还，`miyu_client::heap::trim` 在那里什么都不做。

use std::time::{Duration, Instant};

/// 要不要还、什么时候还。
#[derive(Debug, Default)]
pub struct Trim {
    /// 上次还以后正文最多有几行。
    peak: usize,
    /// 到点还。
    due: Option<Instant>,
}

impl Trim {
    /// 这一拍正文有 `rows` 行：比攒下的最多时少了 `drop` 行以上的，`after` 以后还（这期间又少了的往后挪）。
    pub fn rows(&mut self, rows: usize, now: Instant, drop: usize, after: Duration) {
        if rows >= self.peak {
            self.peak = rows;
        } else if self.peak - rows >= drop.max(1) {
            self.peak = rows;
            self.due = Some(now + after);
        }
    }

    /// 下一次要醒来还的时刻。
    pub fn deadline(&self) -> Option<Instant> {
        self.due
    }

    /// 到点了：交回真，记成还过了。
    pub fn take(&mut self, now: Instant) -> bool {
        if self.due.is_some_and(|due| due <= now) {
            self.due = None;
            return true;
        }
        false
    }
}

/// 在后台线程里还一次（起不来线程的就不还）。
pub fn release() {
    let spawned = std::thread::Builder::new()
        .name("heap-trim".into())
        .spawn(|| {
            miyu_client::heap::trim();
        });
    drop(spawned);
}

#[cfg(test)]
mod tests;
