//! 要结束的信号（`SIGTERM`、`SIGINT`、`SIGHUP`、`SIGQUIT`）：还原终端，再照信号的惯例结束（2026-11-28 审计第 4 条：原来
//! 一个都没接，`pkill miyu`、`kill`、`timeout` 结束界面时终端留在原始模式、备用屏和鼠标上报里，人回到 shell 一动鼠标就是
//! 一串乱码）。照 LiteMiyu 的做法（`lm2-ui` 的 `term/signals.rs`）：
//!
//! - 处理函数里只往 signal-hook 自己的管道写一个字节，另起一条线程睡着等它（不定时醒，蓝图「空闲不醒」）。
//! - 全屏：先叫主循环照常退出（输入法切回原样、记下这一次的样子、还原终端），退完了照这个信号的默认动作结束（shell 看到的是
//!   被这个信号结束的，同没接它一样）；过了 [`QUIT_BY`] 主循环还没退完的（卡住了），等信号的这条线程自己还原终端、结束。
//!   shell 集成画一轮的那几种没有能叫的主循环：当场还原、结束。
//! - 开始收尾以后再来一个同样的信号：照系统默认的动作当场结束（`flag::register_conditional_default`），不再收拾。
//! - 让出终端给编辑器的时候（[`yielded`]），`SIGINT`、`SIGQUIT` 不算：图形编辑器（`code --wait`）等着时在终端按 `Ctrl+C`、
//!   `Ctrl+\`，终端发给整个前台进程组，照 `system(3)` 的规矩交给编辑器办（它照默认的退出，界面回来），界面不退。编辑器是
//!   另起的程序，处理函数不跟过去。
//!
//! 只在 Unix 上有；Windows 关窗、`Ctrl+C` 走的是控制台事件，别的系统上这几个都什么都不做。

use std::io::Write;

/// 收尾的总期限：从收到信号起，到点主循环还没退完的不等，等信号的线程自己还原终端、结束。照常的退出连同收尾在一两百毫秒
/// 以内（LiteMiyu 量过的同一类收尾，`term/signals.rs` 的 `QUIT_BY`）。
pub const QUIT_BY: std::time::Duration = std::time::Duration::from_secs(1);

/// 还原终端的那一件：收到信号以后（或者主循环到点没退完）在等信号的线程里调，写进交来的标准输出。
pub type Restore = Box<dyn Fn(&mut dyn Write) + Send>;

/// 叫主循环照常退出：交回叫到了没有（主循环已经不在了的是假，当场还原）。
pub type Ask = Box<dyn Fn() -> bool + Send>;

#[cfg(unix)]
mod unix {
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    use signal_hook::iterator::Signals;

    use super::{Ask, QUIT_BY, Restore};

    /// 接的这几个。
    const ENDING: [i32; 4] = [SIGTERM, SIGINT, SIGHUP, SIGQUIT];

    /// 收到的、要照它结束的那个信号（没有是 0）：主循环退完了照它结束（[`end`]）。
    static RECEIVED: AtomicI32 = AtomicI32::new(0);
    /// 让出终端给编辑器（[`yielded`]）。
    static YIELDED: AtomicBool = AtomicBool::new(false);

    /// 接上这几个信号，起等它们的线程（一个进程只接一次；接不上的照原样不接）。`ask` 是叫主循环照常退出（没有的当场还原），
    /// `restore` 是还原终端。
    pub fn install(ask: Option<Ask>, restore: Restore) {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            // 收尾开始以后置上：再来的同一类照系统默认的动作当场结束。要在登记等它们的那一件以前登记（signal-hook 照登记的先后跑，
            // LiteMiyu 2026-11-28 量到：顺序反了头一次信号就可能被当成第二次）。
            let hard = Arc::new(AtomicBool::new(false));
            for signal in ENDING {
                if signal_hook::flag::register_conditional_default(signal, Arc::clone(&hard))
                    .is_err()
                {}
            }
            let Ok(mut signals) = Signals::new(ENDING) else {
                return;
            };
            let spawned = std::thread::Builder::new()
                .name("signals".into())
                .stack_size(64 * 1024)
                .spawn(move || {
                    for signal in signals.forever() {
                        if YIELDED.load(Ordering::SeqCst) && matches!(signal, SIGINT | SIGQUIT) {
                            continue;
                        }
                        hard.store(true, Ordering::SeqCst);
                        RECEIVED.store(signal, Ordering::SeqCst);
                        quit(signal, ask.as_ref(), &restore);
                    }
                });
            if spawned.is_err() {
                // 起不了盯信号的线程：照旧跑，只是被信号结束时不收拾（同改之前）。
            }
        });
    }

    /// 收到 `signal`：叫得到主循环的叫它退，到点还在的、叫不到的自己还原终端，照默认动作结束。
    fn quit(signal: i32, ask: Option<&Ask>, restore: &Restore) {
        if ask.is_some_and(|ask| ask()) {
            std::thread::sleep(QUIT_BY);
        }
        // 拿着标准输出的锁到结束：主循环这时写不进半帧。
        let mut out = std::io::stdout().lock();
        restore(&mut out);
        if out.flush().is_err() {
            // 写不出去的不管：接着照信号结束。
        }
        die(signal);
    }

    /// 照 `signal` 的默认动作结束；结束不了的（不该有）照 shell 的惯例以 128 加信号编号退出。
    fn die(signal: i32) -> ! {
        if signal_hook::low_level::emulate_default_handler(signal).is_err() {
            // 照默认动作结束不了：下面照 shell 的惯例退出。
        }
        signal_hook::low_level::exit(128 + signal)
    }

    /// 主循环照常退完了：是因为收到信号退的，照那个信号的默认动作结束；不是的什么都不做。
    pub fn end() {
        let signal = RECEIVED.load(Ordering::SeqCst);
        if signal != 0 {
            die(signal);
        }
    }

    /// 让出终端给编辑器跑 `run`：这期间的 `SIGINT`、`SIGQUIT` 交给编辑器办，界面不退。
    pub fn yielded<T>(run: impl FnOnce() -> T) -> T {
        YIELDED.store(true, Ordering::SeqCst);
        let ran = run();
        YIELDED.store(false, Ordering::SeqCst);
        ran
    }
}

#[cfg(unix)]
pub use unix::{end, install, yielded};

/// 别的系统：不接。
#[cfg(not(unix))]
pub fn install(_ask: Option<Ask>, _restore: Restore) {}

/// 别的系统：什么都不做。
#[cfg(not(unix))]
pub fn end() {}

/// 别的系统：照常跑。
#[cfg(not(unix))]
pub fn yielded<T>(run: impl FnOnce() -> T) -> T {
    run()
}
