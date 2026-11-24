//! 读按键的线程（蓝图 `tui.md`「她的回答：Markdown」第 10 条）：一直读终端的事件送进主循环；让出终端给编辑器时能
//! 停下，不跟编辑器抢键。没有事件时一直睡着，不定时醒来看要不要停（蓝图「空闲不醒」第 2 条）：照 crossterm
//! 的事件流等，要它停下时敲一下；扔掉事件流时 crossterm 用它自带的叫醒，让在等终端的那一下也收手。终端往输入里回的话
//! （OSC）不当按键，截下来另交（`reader/replies.rs`）；不发括号粘贴的终端粘进来的一串按键认成粘贴（`reader/burst.rs`）。

use std::future::poll_fn;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use futures_core::Stream;
use ratatui::crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::Notify;

mod burst;
#[cfg(target_os = "linux")]
mod hangup;
mod replies;
use burst::Burst;
pub use replies::Read;
use replies::Replies;

/// 读按键的线程的开关。
#[derive(Clone, Default)]
pub struct Reader {
    /// 要它停下。
    paused: Arc<AtomicBool>,
    /// 要它停下、接着读时敲一下。
    knock: Arc<Notify>,
    /// 它停下了（不在读）。
    idle: Arc<AtomicBool>,
}

impl Reader {
    /// 起一个线程读终端的事件，交给 `send`；`send` 交回假（主循环不收了）或者终端读不了时收工。
    pub fn spawn(send: impl Fn(Read) -> bool + Send + 'static) -> Self {
        // 终端挂断了 crossterm 读不出来也不报错，原地空转：另起一条看挂断的（`reader/hangup.rs`）。
        #[cfg(target_os = "linux")]
        hangup::watch();
        let reader = Self::default();
        let (paused, knock, idle) = (
            reader.paused.clone(),
            reader.knock.clone(),
            reader.idle.clone(),
        );
        thread::spawn(move || {
            // 只用来同时等两样（终端的事件、敲门），不带网络：没事时一直睡着。计时器只在攒着一串像回话的按键时等（`replies.rs`）。
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                loop {
                    if paused.load(Ordering::Acquire) {
                        idle.store(true, Ordering::Release);
                        knock.notified().await;
                        continue;
                    }
                    idle.store(false, Ordering::Release);
                    if !read_until_knocked(&send, &knock).await {
                        return;
                    }
                }
            });
        });
        reader
    }

    /// 停下：等它不在读了才交回（最多等一秒）。
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
        self.knock.notify_one();
        let until = Instant::now() + Duration::from_secs(1);
        while !self.idle.load(Ordering::Acquire) && Instant::now() < until {
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// 接着读。
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Release);
        self.knock.notify_one();
    }
}

/// 读到有人敲门为止，交回真；`send` 不收了、终端读不了交回假。终端的回话先截下（`reader/replies.rs`），一口气到的
/// 一串按键再认是不是粘贴（`reader/burst.rs`）。
async fn read_until_knocked(send: &impl Fn(Read) -> bool, knock: &Notify) -> bool {
    let mut events = EventStream::new();
    let mut replies = Replies::default();
    let mut burst = Burst::default();
    let hand = |out: Vec<Read>| out.into_iter().all(send);
    let through = |burst: &mut Burst, reads: Vec<Read>| {
        let now = Instant::now();
        reads
            .into_iter()
            .flat_map(|read| burst.feed(read, now))
            .collect::<Vec<_>>()
    };
    loop {
        let holding = replies.holding();
        let until = burst.deadline();
        tokio::select! {
            got = poll_fn(|cx| std::pin::Pin::new(&mut events).poll_next(cx)) => match got {
                Some(Ok(event)) => {
                    if !hand(through(&mut burst, replies.feed(backspace_from_ctrl_h(event)))) {
                        return false;
                    }
                }
                // 上一份事件流扔掉时的叫醒没人收，留到了这一份：不是终端出错，接着等。
                Some(Err(e)) if e.kind() == ErrorKind::Interrupted => {}
                _ => return false,
            },
            () = tokio::time::sleep(replies::WAIT), if holding => {
                if !hand(through(&mut burst, replies.expire())) {
                    return false;
                }
            }
            () = sleep_until(until), if until.is_some() => {
                if !hand(burst.expire()) {
                    return false;
                }
            }
            () = knock.notified() => {
                let mut out = through(&mut burst, replies.expire());
                out.extend(burst.expire());
                return hand(out);
            }
        }
    }
}

/// 有的终端把退格键发成 `Ctrl+H`（0x08），crossterm 读成 `Ctrl` 加 `h`，原来哪都不认、删不了字（2026-11-25 查出）：
/// 当成退格（readline 的老规矩），输入框、shell 里答话、配置页的框照退格办，碰到块整块删。开了 kitty 键盘协议的终端
/// 退格另有一码，有意按 `Ctrl+H` 的也是退格。
fn backspace_from_ctrl_h(event: Event) -> Event {
    match event {
        Event::Key(key)
            if key.code == KeyCode::Char('h') && key.modifiers == KeyModifiers::CONTROL =>
        {
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..key
            })
        }
        other => other,
    }
}

/// 等到 `until`；没有的当场就到（只在有的时候等）。
async fn sleep_until(until: Option<Instant>) {
    if let Some(until) = until {
        tokio::time::sleep_until(tokio::time::Instant::from_std(until)).await;
    }
}
