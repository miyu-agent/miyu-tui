//! 本机时区（正文收尾行的时刻、会话列表的日子、循环的时刻、用量页的日子，蓝图「量尺」的时区那一段）。
//!
//! 不直接用 `jiff::tz::TimeZone::system()`（`jiff::Zoned::now()` 走的也是它）：没设 `TZ` 时它照 `/etc/localtime` 链到的
//! 名字（`…/zoneinfo/Asia/Tokyo` 的 `Asia/Tokyo`）到 zoneinfo 库里找，那个库头一回建时把 `/usr/share/zoneinfo` 整个目录走一遍，
//! 一千多个名字连同路径留在堆上不还（2026-11-22 heaptrack 量到 111 kB，每个窗口一份）。这里照同样的链接认出名字，直接读
//! `/etc/localtime` 本身（就是链到的那个文件），起同样的名字：时刻、和 UTC 差多少、时区的简写、名字都和 jiff 找到的一样。
//!
//! 别的情况一律退回 `TimeZone::system()`，和原来一样：设了 `TZ`（要照 POSIX 规则或名字认）、设了 `TZDIR`（jiff 到那里找）、
//! `/etc/localtime` 不是链到 zoneinfo 里的链接或者读不了、非 Unix（Windows 问系统）、Android（问系统属性）。
//!
//! 记一份用五分钟，到点重认，和 jiff 自己记的一样长：改了系统时区，开着的界面照旧五分钟以内跟上。

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

/// 认过的本机时区记多久（和 jiff 记 `TimeZone::system()` 的一样）。
const KEEP: Duration = Duration::from_secs(5 * 60);

/// 本机时区。
pub fn local() -> TimeZone {
    static KEPT: Mutex<Option<(Instant, TimeZone)>> = Mutex::new(None);
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if let Some((at, tz)) = kept.as_ref()
        && now.saturating_duration_since(*at) < KEEP
    {
        return tz.clone();
    }
    let tz = find();
    *kept = Some((now, tz.clone()));
    tz
}

/// 现在，照本机时区。
pub fn now() -> Zoned {
    Timestamp::now().to_zoned(local())
}

/// 认一次本机时区。
fn find() -> TimeZone {
    #[cfg(all(unix, not(target_os = "android")))]
    if std::env::var_os("TZ").is_none()
        && std::env::var_os("TZDIR").is_none()
        && let Some(tz) = linked(std::path::Path::new("/etc/localtime"))
    {
        return tz;
    }
    TimeZone::system()
}

/// `path` 是链到 zoneinfo 里某个时区的链接：读它，名字照链接里 `zoneinfo/` 后面那一截（jiff 照同样的认法）。
/// 不是这样的链接、读不了、不是时区文件的交回 `None`。
#[cfg(all(unix, not(target_os = "android")))]
fn linked(path: &std::path::Path) -> Option<TimeZone> {
    const NEEDLE: &str = "zoneinfo/";
    let target = std::fs::read_link(path).ok()?;
    let target = target.to_str()?;
    let name = &target[target.rfind(NEEDLE)? + NEEDLE.len()..];
    let bytes = std::fs::read(path).ok()?;
    TimeZone::tzif(name, &bytes).ok()
}

#[cfg(all(test, unix, not(target_os = "android")))]
mod tests;
