//! 等点击的系统通知（蓝图 `tui.md`「系统通知」第 8 条）：Linux 的 `notify-send` 带默认动作「打开」加 `--wait`，点了印出
//! `default`，叫醒主循环切到那个会话。`--wait` 要等通知被点开或关掉才退，通知中心里一直挂着的就一直等着（2026-11-20 量内存时
//! 撞见：十个窗口各说一句，留下 90 个等着的 `notify-send`，一个约 0.85 MB）。所以：
//! - 同一个会话只留最新一条：新的来了先结束上一条还在等的（按子进程句柄 `kill` 加 `wait`，不按名字找），`-p` 印出的通知编号
//!   配 `-r` 把桌面上那条换掉；这一版不认 `-p` 的照只带动作的弹，不认动作的照不带动作的弹（认不认记下，下一条不再试）。
//! - 等点击最多等 [`CLICK_WAIT`]：到时结束等着的那个，桌面上的通知留着，点了不再切。
//! - [`Waiting`] 丢掉时（界面退出）把还在等的全部结束。

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use super::click::Clicked;

/// 等点击最多等多久：照核心等人确认的出厂时长（`permission.confirm.timeout`，5 分钟，「确认和提问的抽屉」）——这么久没回来
/// 的人回来时看侧边栏的圆点、会话列表，不靠点这一条。
pub const CLICK_WAIT: Duration = Duration::from_secs(5 * 60);

/// 等点击的系统通知：会话 → 它最新的那一条。只有一个主人（[`Notifier`](super::Notifier)），丢掉时全部结束。
pub struct Waiting {
    shared: Arc<Shared>,
    limit: Duration,
}

struct Shared {
    state: Mutex<State>,
    /// 叫醒看时限的线程：要收工了。
    tick: Condvar,
}

#[derive(Default)]
struct State {
    slots: HashMap<String, Slot>,
    /// 下一条的序号：跟着会话那一格认是不是还是自己（被新的顶掉、被结束的就不是了）。
    next: u64,
    /// 这一版 `notify-send` 认不认 `-p`、`-r`：没试过的是 `None`。
    numbered: Option<bool>,
    /// 看时限的线程在跑：没有在等的就退。
    sweeping: bool,
    /// 收工了（界面退出）：不再弹、不再拉起。
    closed: bool,
}

/// 一个会话最新的那一条。
struct Slot {
    turn: u64,
    /// 等着的 `notify-send`；结束了（到时、点了、关了）的是 `None`。
    child: Option<Child>,
    /// 桌面上那条的编号（`-p` 印的）：同会话的下一条用 `-r` 换掉它。
    id: Option<u32>,
    deadline: Instant,
}

/// `notify-send` 的哪种写法，照先后试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// `-p [-r 编号] -A default=打开 --wait`。
    Numbered,
    /// `-A default=打开 --wait`：不认 `-p` 的。
    Action,
    /// 不带动作：连动作都不认的，丢出去不等。
    Plain,
}

impl Default for Waiting {
    fn default() -> Self {
        Self::new(CLICK_WAIT)
    }
}

impl Waiting {
    /// 等点击最多等 `limit`。
    pub fn new(limit: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State::default()),
                tick: Condvar::new(),
            }),
            limit,
        }
    }

    /// 弹一条点了切到 `session` 的通知：`program` 是 `notify-send`，`plain` 是不带动作时的参数（`-a … -- 标题 正文`），`label`
    /// 是按钮上的字。同会话上一条还在等的先结束。
    pub fn show(
        &self,
        program: &str,
        label: &str,
        plain: Vec<String>,
        (clicked, session): (Clicked, String),
    ) {
        if !super::system::guard(program) {
            return;
        }
        let mut state = self.shared.lock();
        if state.closed {
            return;
        }
        let replace = state.slots.remove(&session).and_then(|old| {
            end(old.child);
            old.id
        });
        state.next += 1;
        let job = Job {
            shared: Arc::clone(&self.shared),
            program: program.to_string(),
            label: label.to_string(),
            plain,
            session,
            turn: state.next,
            clicked,
        };
        let form = match state.numbered {
            Some(false) => Form::Action,
            _ => Form::Numbered,
        };
        let slot = Slot {
            turn: job.turn,
            child: None,
            id: replace,
            deadline: Instant::now() + self.limit,
        };
        state.slots.insert(job.session.clone(), slot);
        let Some(stdout) = job.launch(&mut state, form) else {
            job.forget(&mut state);
            return;
        };
        if !state.sweeping {
            state.sweeping = true;
            let shared = Arc::clone(&self.shared);
            std::thread::spawn(move || sweep(&shared));
        }
        drop(state);
        std::thread::spawn(move || job.follow(stdout, form));
    }
}

impl Drop for Waiting {
    /// 界面退出：还在等的全部结束，看时限的线程收工。
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        state.closed = true;
        for slot in state.slots.values_mut() {
            end(slot.child.take());
        }
        drop(state);
        self.shared.tick.notify_all();
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 结束一个等着的 `notify-send`（没有的什么都不做）：按它自己的句柄，结束了收尸（不留僵尸）。
fn end(child: Option<Child>) {
    if let Some(mut child) = child {
        drop(child.kill());
        drop(child.wait());
    }
}

/// 一条通知：在另一个线程里读 `notify-send` 印的，等它退出。
struct Job {
    shared: Arc<Shared>,
    program: String,
    label: String,
    plain: Vec<String>,
    session: String,
    turn: u64,
    clicked: Clicked,
}

impl Job {
    /// 照 `form` 拉起，放进会话那一格（还是自己的才放），交回它印字的那一头。拉不起来的、`Plain` 的（丢出去不等）是 `None`。
    fn launch(&self, state: &mut State, form: Form) -> Option<ChildStdout> {
        let slot = state
            .slots
            .get_mut(&self.session)
            .filter(|s| s.turn == self.turn)?;
        let mut args: Vec<String> = Vec::new();
        if form == Form::Numbered {
            args.push("-p".into());
            if let Some(id) = slot.id {
                args.extend(["-r".into(), id.to_string()]);
            }
        }
        if form == Form::Plain {
            super::system::spawn(&self.program, &self.plain);
            return None;
        }
        args.extend([
            "-A".into(),
            format!("default={}", self.label),
            "--wait".into(),
        ]);
        args.extend(self.plain.iter().cloned());
        let mut child = Command::new(&self.program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take();
        slot.child = Some(child);
        stdout
    }

    /// 读它印的：数字是通知编号（`-p`），`default` 是点了。读完（退了、被结束了）照怎么退的办：点了的叫醒主循环；关掉的
    /// 不管；不认这种写法的（退出码不是 0、什么都没印）换下一种再弹。被顶掉、被结束的什么都不做。
    fn follow(self, mut stdout: ChildStdout, mut form: Form) {
        loop {
            let (said, clicked) = self.read(stdout);
            let mut state = self.shared.lock();
            let Some(slot) = state
                .slots
                .get_mut(&self.session)
                .filter(|s| s.turn == self.turn)
            else {
                return;
            };
            let Some(mut child) = slot.child.take() else {
                return;
            };
            drop(state);
            let fine = child.wait().is_ok_and(|s| s.success());
            let mut state = self.shared.lock();
            let refused = !fine && !said;
            // 认不认 `-p`：带 `-p` 的不认、只带动作的接着弹的（它一直等着，等不到它退）就记成不认；只带动作的也不认，是连
            // 不上通知服务这类，说不准，照没试过。
            match (form, refused) {
                (Form::Numbered, false) => state.numbered = Some(true),
                (Form::Numbered, true) if state.numbered.is_none() => {
                    state.numbered = Some(false);
                }
                (Form::Action, true) if state.numbered == Some(false) => state.numbered = None,
                _ => {}
            }
            if !refused || state.closed {
                // 点了、关了：桌面上那条没了，这一格不留。
                self.forget(&mut state);
                drop(state);
                if clicked {
                    self.clicked.push(self.session.clone());
                }
                return;
            }
            form = match form {
                Form::Numbered => Form::Action,
                Form::Action | Form::Plain => Form::Plain,
            };
            match self.launch(&mut state, form) {
                Some(next) => stdout = next,
                None => {
                    self.forget(&mut state);
                    return;
                }
            }
        }
    }

    /// 会话那一格还是自己的就拿掉（等着的时候被同会话的新一条顶掉的，那一格是新的，不碰）。
    fn forget(&self, state: &mut State) {
        if state
            .slots
            .get(&self.session)
            .is_some_and(|s| s.turn == self.turn)
        {
            state.slots.remove(&self.session);
        }
    }

    /// 读到它不再印（退了、被结束了）：印过什么没有、点了没有。印了编号的记进这一格。
    fn read(&self, stdout: ChildStdout) -> (bool, bool) {
        let (mut said, mut clicked) = (false, false);
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let line = line.trim();
            said |= !line.is_empty();
            if line == "default" {
                clicked = true;
            } else if let Ok(id) = line.parse::<u32>() {
                let mut state = self.shared.lock();
                if let Some(slot) = state
                    .slots
                    .get_mut(&self.session)
                    .filter(|s| s.turn == self.turn)
                {
                    slot.id = Some(id);
                }
            }
        }
        (said, clicked)
    }
}

/// 看时限：到时的结束（编号留着，同会话的下一条照它换掉桌面上那条），睡到最早的那个到时；没有在等的就退（下一条再起）。
fn sweep(shared: &Shared) {
    let mut state = shared.lock();
    loop {
        if state.closed {
            return;
        }
        let now = Instant::now();
        for slot in state.slots.values_mut() {
            if slot.deadline <= now {
                end(slot.child.take());
            }
        }
        let Some(next) = state
            .slots
            .values()
            .filter(|s| s.child.is_some())
            .map(|s| s.deadline)
            .min()
        else {
            state.sweeping = false;
            return;
        };
        state = shared
            .tick
            .wait_timeout(state, next.saturating_duration_since(now))
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
}

#[cfg(all(test, unix))]
mod tests;
