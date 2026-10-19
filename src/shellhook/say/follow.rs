//! 盯后台（设计 22 第七节「后台任务和回报」，图纸 `shellhook.md`「后台回报」）：一轮答完还有派出去没结束的后台任务时，
//! `say` 起一个这个，它订阅终端会话的推送，只认交给它的那几个任务编号：结束了的、报回来叫醒她的那一轮，记进这个 shell 的
//! 信箱（`spool.rs`），给 shell 发 `SIGUSR1`，shell 回到提示符时印。不定时去查：等核心的推送、等 shell 退出（Linux 上
//! `pidfd`），没事的时候一直睡着。shell 退出了就收工，不再回报。

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use super::spool::{self, Record};
use crate::core::{Command, JobStart, Push, Start, Update};

/// 交给盯后台的：终端会话、派出去还没结束的任务、哪个 shell。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watch {
    pub session: String,
    pub jobs: Vec<JobStart>,
    pub shell: u32,
}

/// 起一个盯后台的：写在信箱目录里交给它，自己一个进程组，不接终端（不碰 shell 的前台）。
///
/// # Errors
///
/// 找不到缓存目录、写不进、起不来。
pub fn spawn(watch: &Watch) -> io::Result<()> {
    let dir = spool::dir().ok_or_else(|| io::Error::other("no cache directory"))?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("watch-{}-{}.json", watch.shell, std::process::id()));
    std::fs::write(&path, serde_json::to_vec(watch).map_err(io::Error::other)?)?;
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .args(["shellhook", "follow", "--watch"])
        .arg(&path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command.spawn().map(drop)
}

/// 主循环等的东西。
enum Incoming {
    Core(Box<Update>),
    /// shell 退出了。
    Gone,
}

/// 盯到没有要等的、或者 shell 退出了为止。
pub fn run(path: &Path) -> ExitCode {
    let Some(watch) = read(path) else {
        return ExitCode::FAILURE;
    };
    let table = crate::language::LanguageTable::builtin();
    let language = table.map(|t| t.detect(|name| std::env::var(name).ok()));
    let config = match language.and_then(|l| crate::config::Config::load(&l, true)) {
        Ok(config) => config,
        Err(_) => return ExitCode::FAILURE,
    };
    let (sender, incoming) = mpsc::channel();
    let to_core = sender.clone();
    let core = crate::core::spawn(config.layout.reconnect_ms, Start::Bare, move |update| {
        to_core.send(Incoming::Core(Box::new(update))).is_ok()
    });
    if !wait_for_exit(watch.shell, move || sender.send(Incoming::Gone).is_ok()) {
        return ExitCode::FAILURE;
    }
    core.send(Command::Open {
        session: watch.session.clone(),
        keep: false,
    });
    let mut follow = Follow::new(&watch);
    while let Ok(event) = incoming.recv() {
        let update = match event {
            Incoming::Gone => break,
            Incoming::Core(update) => match *update {
                Update::Elsewhere { session, update } if session == watch.session => *update,
                _ => continue,
            },
        };
        for record in follow.update(update) {
            if spool::push(watch.shell, &record).is_ok() {
                signal(watch.shell);
            }
        }
        if follow.finished() {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// 读交来的那份，读完删掉。
fn read(path: &Path) -> Option<Watch> {
    let bytes = std::fs::read(path).ok()?;
    drop(std::fs::remove_file(path));
    serde_json::from_slice(&bytes).ok()
}

/// 盯着的几样。
struct Follow {
    session: String,
    /// 还没结束的任务：编号到它开始时的样子。
    jobs: HashMap<String, JobStart>,
    /// 最新一页补完了（以前的不算）。
    paged: bool,
    /// 你说的话的序号：照它开的轮不是回报叫醒的。
    said: HashSet<u64>,
    /// 子代理报回来了几份、还没等到它叫醒的那一轮。
    owed: usize,
    /// 正在跑的、回报叫醒的那一轮：这一轮里派出去的也接着盯。
    current: Option<u64>,
}

impl Follow {
    fn new(watch: &Watch) -> Self {
        Self {
            session: watch.session.clone(),
            jobs: watch
                .jobs
                .iter()
                .map(|j| (j.job.clone(), j.clone()))
                .collect(),
            paged: false,
            said: HashSet::new(),
            owed: 0,
            current: None,
        }
    }

    /// 收一条终端会话的推送，交回要印的。
    fn update(&mut self, update: Update) -> Vec<Record> {
        let push = match update {
            Update::Paged(_) | Update::Older { .. } => {
                self.paged = true;
                return Vec::new();
            }
            Update::Push(push) if self.paged => push,
            _ => return Vec::new(),
        };
        match push {
            Push::Said { seq, .. } | Push::UserMessage(seq) => {
                self.said.insert(seq);
            }
            Push::JobEnded(end) => {
                if let Some(start) = self.jobs.remove(&end.job) {
                    // 结束了都会叫醒她：子代理报回来、后台命令跑完都是（2026-10-20 真模型实测）。
                    self.owed += 1;
                    return vec![Record::Ended { start, end }];
                }
            }
            // 不是你说的话开的那一轮：是回报叫醒的。
            Push::TurnStarted(turn, trigger)
                if self.owed > 0 && trigger.is_none_or(|t| !self.said.contains(&t)) =>
            {
                self.owed -= 1;
                self.current = Some(turn);
                return vec![Record::Turn {
                    session: self.session.clone(),
                    turn,
                }];
            }
            Push::JobStarted(start) if self.current.is_some() && !start.foreground => {
                self.jobs.insert(start.job.clone(), start);
            }
            Push::TurnEnded(_) => self.current = None,
            _ => {}
        }
        Vec::new()
    }

    /// 没有要等的了：任务都结束了，叫醒的那一轮也都等到了、跑完了。
    fn finished(&self) -> bool {
        self.paged && self.jobs.is_empty() && self.owed == 0 && self.current.is_none()
    }
}

/// 给 shell 发 `SIGUSR1`：它回到提示符时来拿（fish 的脚本）。
fn signal(shell: u32) {
    #[cfg(unix)]
    if let Some(pid) = i32::try_from(shell)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        && rustix::process::kill_process(pid, rustix::process::Signal::USR1).is_err()
    {}
}

/// shell 退出时叫 `gone`：Linux 上照 `pidfd` 等（不轮询）。别的平台还没做（macOS 的 kqueue），不叫：任务都结束了照样收工。
/// shell 已经不在了交回 `false`。
fn wait_for_exit(shell: u32, gone: impl FnOnce() -> bool + Send + 'static) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Some(pid) = i32::try_from(shell)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        else {
            return false;
        };
        let Ok(fd) = rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()) else {
            return false;
        };
        std::thread::spawn(move || {
            let mut fds = [rustix::event::PollFd::new(
                &fd,
                rustix::event::PollFlags::IN,
            )];
            while rustix::event::poll(&mut fds, None).is_err() {}
            gone();
        });
        true
    }
    #[cfg(not(target_os = "linux"))]
    {
        drop(gone);
        shell > 0
    }
}

#[cfg(test)]
mod tests;
