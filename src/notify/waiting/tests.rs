//! 等点击的系统通知（`waiting.rs`）：用临时目录里假的 `notify-send`（测试里只许拉起临时目录的程序，`system::guard`），它把自己的
//! 进程号和参数记进 `calls`，再 `exec sleep` 一直等着，像通知中心里没人点、没人关的那一条。

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::Waiting;
use crate::notify::click::Clicked;

/// 等假程序记下调用、等进程退掉，最多等这么久。
const PATIENCE: Duration = Duration::from_secs(10);

/// 假的 `notify-send`：`body` 是记下调用以后接着跑的那几行 `sh`。测试完（连断言失败）结束还活着的、删掉目录：`/tmp` 是内存盘。
struct Fake {
    dir: PathBuf,
    program: String,
}

impl Fake {
    fn new(name: &str, body: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("miyu-wait-{}-{name}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).expect("临时目录");
        let fake = dir.join("fake-notify-send");
        let calls = dir.join("calls");
        let script = format!(
            "#!/bin/sh\necho \"$$ $*\" >> '{}'\n{body}\n",
            calls.display()
        );
        std::fs::write(&fake, script).expect("假程序");
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("可执行");
        Self {
            dir,
            program: fake.display().to_string(),
        }
    }

    /// 像 `notify-send 0.8`：带 `-p` 的先印通知编号，再一直等着（`exec`：进程号不变，结束它就是结束等着的那个）。
    fn waiting(name: &str) -> Self {
        Self::new(
            name,
            "case \" $* \" in *\" -p \"*) echo 42 ;; esac\nexec sleep 600",
        )
    }

    /// 记下的调用：进程号和参数，照先后。等到至少 `n` 条。
    fn calls(&self, n: usize) -> Vec<(u32, String)> {
        let end = Instant::now() + PATIENCE;
        loop {
            let text = std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default();
            let calls: Vec<(u32, String)> = text
                .lines()
                .filter_map(|l| {
                    let (pid, args) = l.split_once(' ')?;
                    Some((pid.parse().ok()?, args.to_string()))
                })
                .collect();
            if calls.len() >= n || Instant::now() > end {
                return calls;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// 还活着的假 `notify-send`（收过尸的才算没了）。等到恰好 `n` 个、最多等 [`PATIENCE`]，交回那时活着的。
    fn alive(&self, n: usize) -> Vec<u32> {
        let end = Instant::now() + PATIENCE;
        loop {
            let alive: Vec<u32> = self
                .calls(0)
                .into_iter()
                .map(|(pid, _)| pid)
                .filter(|&pid| alive(pid))
                .collect();
            if alive.len() == n || Instant::now() > end {
                return alive;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        for (pid, _) in self.calls(0) {
            let pid = rustix::process::Pid::from_raw(pid as i32);
            if pid.is_some_and(|pid| {
                rustix::process::kill_process(pid, rustix::process::Signal::KILL).is_err()
            }) {}
        }
        drop(std::fs::remove_dir_all(&self.dir));
    }
}

/// 进程还在（没收尸的僵尸也算在：结束了就要收尸，不留僵尸）。
fn alive(pid: u32) -> bool {
    rustix::process::Pid::from_raw(pid as i32)
        .is_some_and(|pid| rustix::process::test_kill_process(pid).is_ok())
}

fn queue() -> (Clicked, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    let mut queue = Clicked::default();
    queue.set_wake(std::sync::Arc::new(move || if tx.send(()).is_err() {}));
    (queue, rx)
}

fn plain() -> Vec<String> {
    ["-a", "Miyu", "--", "标题", "回答完了"]
        .map(str::to_string)
        .to_vec()
}

/// 2026-11-20 量内存时撞见：每弹一条带「打开」的通知就留一个 `notify-send --wait`，十个窗口各说一句留下 90 个。同一个会话只留
/// 最新一条：新的来了结束上一条还在等的（按子进程句柄，不按名字），带 `-r` 换掉桌面上那条；别的会话的照留；界面退出时全部结束。
#[test]
fn a_new_notice_for_the_same_session_ends_the_one_still_waiting() {
    let fake = Fake::waiting("same");
    let waiting = Waiting::new(Duration::from_secs(600));
    let (queue, _rx) = queue();
    for _ in 0..3 {
        waiting.show(
            &fake.program,
            "打开",
            plain(),
            (queue.clone(), "s-1".into()),
        );
        // 一条一条来：上一条记下了才弹下一条（像隔一会儿又做完一轮）。
        fake.calls(fake.calls(0).len() + 1);
    }
    let calls = fake.calls(3);
    assert_eq!(calls.len(), 3, "{calls:?}");
    let alive = fake.alive(1);
    assert_eq!(alive, [calls[2].0], "只留最新一条在等：{calls:?}");
    assert!(
        calls[0].1.starts_with("-p -A default=打开 --wait -a Miyu"),
        "{calls:?}"
    );
    for (_, args) in &calls[1..] {
        assert!(
            args.starts_with("-p -r 42 -A default=打开 --wait"),
            "换掉桌面上那条：{calls:?}"
        );
    }
    // 别的会话的不动。
    waiting.show(
        &fake.program,
        "打开",
        plain(),
        (queue.clone(), "s-2".into()),
    );
    let calls = fake.calls(4);
    assert!(
        calls[3].1.starts_with("-p -A default=打开 --wait"),
        "别的会话不换：{calls:?}"
    );
    assert_eq!(fake.alive(2).len(), 2);
    // 界面退出：全部结束。
    drop(waiting);
    assert_eq!(fake.alive(0), Vec::<u32>::new(), "退出时不留等着的");
}

/// 等点击有时限：到时结束等着的那个（桌面上的通知留着）。
#[test]
fn waiting_for_a_click_ends_after_the_limit() {
    let fake = Fake::waiting("limit");
    let waiting = Waiting::new(Duration::from_millis(300));
    let (queue, _rx) = queue();
    waiting.show(
        &fake.program,
        "打开",
        plain(),
        (queue.clone(), "s-1".into()),
    );
    assert_eq!(fake.calls(1).len(), 1);
    assert_eq!(fake.alive(0), Vec::<u32>::new(), "到时结束");
    // 过了时限再来一条同会话的：照记着的编号换掉桌面上那条。
    waiting.show(&fake.program, "打开", plain(), (queue, "s-1".into()));
    let calls = fake.calls(2);
    assert!(calls[1].1.starts_with("-p -r 42 "), "{calls:?}");
}

/// 这一版 `notify-send` 不认 `-p`（退出码不是 0、什么都没印）：照只带动作的再弹，记下以后不再试 `-p`；照样只留最新一条。
#[test]
fn without_print_id_it_still_keeps_only_the_newest() {
    let fake = Fake::new(
        "nop",
        "case \" $* \" in *\" -p \"*) exit 1 ;; esac\nexec sleep 600",
    );
    let waiting = Waiting::new(Duration::from_secs(600));
    let (queue, _rx) = queue();
    waiting.show(
        &fake.program,
        "打开",
        plain(),
        (queue.clone(), "s-1".into()),
    );
    fake.calls(2);
    waiting.show(&fake.program, "打开", plain(), (queue, "s-1".into()));
    let calls = fake.calls(3);
    let args: Vec<&str> = calls.iter().map(|(_, a)| a.as_str()).collect();
    assert!(args[0].starts_with("-p -A "), "{args:?}");
    assert!(
        args[1].starts_with("-A default=打开 --wait -a Miyu"),
        "{args:?}"
    );
    assert!(
        args[2].starts_with("-A default=打开 --wait"),
        "认过了不再试 -p：{args:?}"
    );
    assert_eq!(fake.alive(1), [calls[2].0]);
}

/// 点了（印出 `default`）的放进队列、叫醒主循环；关掉没点的不切；连动作都不认的照不带动作再弹一次。
#[test]
fn a_click_is_queued_a_dismissal_is_not_and_a_refusal_falls_back_to_plain() {
    let (queue, rx) = queue();
    let clicked = Fake::new(
        "click",
        "case \" $* \" in *\" -p \"*) echo 42 ;; esac\necho default",
    );
    let waiting = Waiting::new(Duration::from_secs(600));
    waiting.show(
        &clicked.program,
        "打开",
        plain(),
        (queue.clone(), "s-1".into()),
    );
    rx.recv_timeout(PATIENCE).expect("点了叫醒主循环");
    assert_eq!(queue.take(), ["s-1"]);
    assert_eq!(clicked.alive(0), Vec::<u32>::new(), "点完退了、收了尸");

    let closed = Fake::new("closed", "exit 0");
    waiting.show(
        &closed.program,
        "打开",
        plain(),
        (queue.clone(), "s-2".into()),
    );
    assert!(
        rx.recv_timeout(Duration::from_millis(500)).is_err(),
        "关掉、没点的不切"
    );
    assert!(queue.take().is_empty());

    let old = Fake::new("old", "case \" $* \" in *\" -A \"*) exit 1 ;; esac");
    waiting.show(&old.program, "打开", plain(), (queue, "s-3".into()));
    let calls = old.calls(3);
    let args: Vec<&str> = calls.iter().map(|(_, a)| a.as_str()).collect();
    assert_eq!(args.len(), 3, "{args:?}");
    assert_eq!(
        args[2], "-a Miyu -- 标题 回答完了",
        "照不带动作再弹：{args:?}"
    );
}
