//! 点了通知切到那个会话（蓝图 `tui.md`「系统通知」第 8 条，核心 J-3 中）：每次新开会话的定时跑完了，弹的通知点开切过去。
//! kitty 照 OSC 99 的 `a=focus,report` 回一句 `99;i=<编号>;`（读按键那一头截下来，`reader/replies.rs`），编号里写着会话；
//! Linux 的 `notify-send` 带一个默认动作、`--wait` 等它，点了印出动作名。macOS 的 `osascript`、OSC 9 没有回话，点了不切。

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

/// 叫醒主循环。
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 点了要切过去的通知，编号的打头（后面接会话编号）。kitty 的编号只认字母、数字和 `-_+.`，会话编号都在里面。
const PREFIX: &str = "miyu-s-";

/// 要切到 `session` 的那条通知的 kitty 编号。
pub fn kitty_id(session: &str) -> String {
    format!("{PREFIX}{session}")
}

/// kitty 回的一句（`99;i=<编号>[:…];…`）是点了哪个会话的通知；别的回话、别的通知是 `None`。
pub fn clicked(reply: &str) -> Option<String> {
    let meta = reply.strip_prefix("99;")?.split(';').next()?;
    meta.split(':')
        .find_map(|kv| kv.strip_prefix("i="))?
        .strip_prefix(PREFIX)
        .filter(|session| !session.is_empty())
        .map(str::to_string)
}

/// 点了的会话：等点击的线程放进来、叫醒主循环，主循环拿走。
#[derive(Clone, Default)]
pub struct Clicked {
    sessions: Arc<Mutex<Vec<String>>>,
    wake: Option<Wake>,
}

impl Clicked {
    pub fn set_wake(&mut self, wake: Wake) {
        self.wake = Some(wake);
    }

    /// 点了 `session` 那条。
    pub fn push(&self, session: String) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.push(session);
        }
        if let Some(wake) = &self.wake {
            wake();
        }
    }

    /// 拿走点了的（最后点的在最后）。
    pub fn take(&self) -> Vec<String> {
        self.sessions
            .lock()
            .map(|mut s| std::mem::take(&mut *s))
            .unwrap_or_default()
    }
}

/// `notify-send` 带默认动作的参数：插在 `--` 前面。
pub fn action_args(label: &str) -> [String; 3] {
    ["-A".into(), format!("default={label}"), "--wait".into()]
}

/// 跑带动作的 `notify-send`，另起一个线程等：点了（印出 `default`）放进 `clicked`。拉不起来的、这一版 `notify-send` 不认
/// 动作（退出码不是 0、什么都没印）的照不带动作再弹一次（`plain`）。
pub fn spawn_waiting(
    program: String,
    args: Vec<String>,
    plain: Vec<String>,
    (clicked, session): (Clicked, String),
) {
    std::thread::spawn(move || {
        let child = Command::new(&program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            return;
        };
        let mut said = String::new();
        if let Some(out) = child.stdout.as_mut() {
            drop(out.read_to_string(&mut said));
        }
        let fine = child.wait().is_ok_and(|s| s.success());
        if said.trim() == "default" {
            clicked.push(session);
        } else if !fine && said.trim().is_empty() {
            super::system::spawn(&program, &plain);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{Clicked, clicked, kitty_id};

    #[test]
    fn a_clicked_kitty_notice_names_its_session() {
        let id = kitty_id("01a17cd2-0852-7199-a84f-f1b4584073fe");
        assert_eq!(id, "miyu-s-01a17cd2-0852-7199-a84f-f1b4584073fe");
        assert_eq!(
            clicked(&format!("99;i={id};")).as_deref(),
            Some("01a17cd2-0852-7199-a84f-f1b4584073fe")
        );
        assert_eq!(
            clicked(&format!("99;i={id}:p=title;")).as_deref(),
            Some("01a17cd2-0852-7199-a84f-f1b4584073fe"),
            "带别的几格的"
        );
        // 平常的通知（不切会话）、别的回话不认。
        assert_eq!(clicked("99;i=miyu-4242;"), None);
        assert_eq!(clicked("11;rgb:1a1a/1b1b/2626"), None);
        assert_eq!(clicked("99;i=miyu-s-;"), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_click_on_a_waiting_system_notice_is_queued_and_a_refusal_is_not() {
        // 冒充 `notify-send`：点了的印 `default`，关掉的什么都不印。
        let (tx, rx) = std::sync::mpsc::channel();
        let mut queue = Clicked::default();
        queue.set_wake(std::sync::Arc::new(move || if tx.send(()).is_err() {}));
        let sh = |script: &str| vec!["-c".to_string(), script.to_string()];
        super::spawn_waiting(
            "sh".into(),
            sh("echo default"),
            sh("true"),
            (queue.clone(), "s-1".into()),
        );
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .expect("点了叫醒主循环");
        assert_eq!(queue.take(), ["s-1"]);
        super::spawn_waiting(
            "sh".into(),
            sh("exit 0"),
            sh("true"),
            (queue.clone(), "s-2".into()),
        );
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(500))
                .is_err(),
            "关掉、没点的不切"
        );
        assert!(queue.take().is_empty());
    }

    #[test]
    fn clicks_from_another_thread_wake_the_loop_and_are_taken_once() {
        let mut queue = Clicked::default();
        let woken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = woken.clone();
        queue.set_wake(std::sync::Arc::new(move || {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }));
        let sender = queue.clone();
        std::thread::spawn(move || sender.push("s-1".into()))
            .join()
            .unwrap();
        assert_eq!(woken.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(queue.take(), ["s-1"]);
        assert!(queue.take().is_empty());
    }
}
