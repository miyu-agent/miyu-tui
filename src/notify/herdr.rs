//! herdr（蓝图 `tui.md`「系统通知」第 6 条，照旧版 09-20 的集成）：在 herdr 的一个窗格里时，把在做、在等、空闲报给
//! herdr，由它在侧栏上显示、由它来响。不在 herdr 里这一段什么都不做。

use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::mpsc;
use std::thread;

/// 报给 herdr 的来源、名字：herdr 按来源记序号水位。
const SOURCE: &str = "custom:miyu";
const AGENT: &str = "miyu";

/// 报的状态（herdr 的 `--state`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// 在做：一轮在跑。
    Working,
    /// 在等你：确认、提问的抽屉开着。
    Blocked,
    /// 空闲：等你打字。
    Idle,
}

impl State {
    fn word(self) -> &'static str {
        match self {
            State::Working => "working",
            State::Blocked => "blocked",
            State::Idle => "idle",
        }
    }
}

/// 坐在 herdr 的哪个窗格里、怎么叫它。
pub struct Herdr {
    reports: Option<mpsc::Sender<Vec<String>>>,
    worker: Option<thread::JoinHandle<()>>,
    session: Option<String>,
    /// 恢复命令打头的那几个（后面接 `--resume <编号>`）：PATH 上找不到的是 `None`，只报会话编号、不报恢复命令。
    resume: Option<Vec<String>>,
    pane: String,
    /// 下一个序号：从进程启动那一刻的毫秒数起往上加。herdr 丢掉比水位小的，每次从 1 数的话第二次开界面的上报
    /// 全被丢掉（旧版 09-20 踩过）。
    next: u64,
    /// 上一次报的：一样的不再报。
    last: Option<State>,
}

impl Herdr {
    /// 照环境变量认：`HERDR_ENV` 是 `1`、有 `HERDR_PANE_ID` 和 `HERDR_BIN_PATH` 才算（用它注入的绝对路径，不受 PATH
    /// 影响）。
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Option<Self> {
        if var("HERDR_ENV").as_deref() != Some("1") {
            return None;
        }
        let pane = var("HERDR_PANE_ID").filter(|p| !p.is_empty())?;
        let binary = var("HERDR_BIN_PATH").filter(|b| !b.is_empty())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        // 单个工作线程按顺序发：resume 需要先取得窗格，release 不能被新状态越过。
        let (reports, receiver) = mpsc::channel::<Vec<String>>();
        let worker = thread::spawn(move || {
            // 报不上去的不管：herdr 是可有可无的，下一次照报。不往 stderr 写：那是全屏界面所在的终端（review B7）。
            for args in receiver {
                let reported = Command::new(&binary)
                    .args(&args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                if reported.is_err() {}
            }
        });
        Some(Self {
            reports: Some(reports),
            worker: Some(worker),
            session: None,
            resume: resume_program(&var),
            pane,
            next: now,
            last: None,
        })
    }

    /// 报一次状态，丢出去不等；和上一次一样的不报。
    pub fn report(&mut self, state: State) {
        if self.last == Some(state) {
            return;
        }
        self.last = Some(state);
        let args = self.report_args(state);
        self.send(args);
    }

    /// 更新主会话恢复命令；清空时先释放旧绑定，再取得窗格状态。
    pub fn session(&mut self, session: Option<&str>) {
        if self.session.as_deref() == session {
            return;
        }
        self.session = session.map(str::to_string);
        let state = self.last.unwrap_or(State::Idle);
        if session.is_none() {
            let args = self.release_args();
            self.send(args);
        }
        let args = self.report_args(state);
        self.send(args);
    }

    /// 人真退出了：退还这个窗格，herdr 当场清掉名字、状态和恢复命令（它的文档：只在人真退出时退还）。
    pub fn release(&mut self) {
        let args = self.release_args();
        self.send(args);
    }

    /// 交给工作线程；它没了（报的程序把它弄崩了）的丢掉，同上不写 stderr。
    fn send(&self, args: Vec<String>) {
        if let Some(tx) = &self.reports
            && tx.send(args).is_err()
        {}
    }

    fn report_args(&mut self, state: State) -> Vec<String> {
        let seq = self.seq();
        let mut args: Vec<String> = [
            "pane",
            "report-agent",
            &self.pane,
            "--source",
            SOURCE,
            "--agent",
            AGENT,
            "--state",
            state.word(),
            "--seq",
            &seq,
        ]
        .map(str::to_string)
        .to_vec();
        // 会话编号登记成这个窗格的会话（`--agent-session-id`）：herdr 重启以后只给登记了会话的窗格跑恢复命令（2026-11-02
        // 项目主人：重启以后 Claude Code 的 tab 回来了、接着原来的会话，Miyu 的没有）。
        if let Some(session) = &self.session {
            args.extend(["--agent-session-id".into(), session.clone()]);
            if let Some(program) = &self.resume {
                args.push("--".into());
                args.extend(program.iter().cloned());
                args.extend(["--resume".into(), session.clone()]);
            }
        }
        args
    }

    /// 退还这个窗格：带序号（不带的 herdr 不认，侧栏一直挂着）。
    fn release_args(&mut self) -> Vec<String> {
        let seq = self.seq();
        [
            "pane",
            "release-agent",
            &self.pane,
            "--source",
            SOURCE,
            "--agent",
            AGENT,
            "--seq",
            &seq,
        ]
        .map(str::to_string)
        .to_vec()
    }

    fn seq(&mut self) -> String {
        let seq = self.next;
        self.next += 1;
        seq.to_string()
    }
}

/// 恢复命令打头的那几个（2026-11-02 项目主人：herdr 重启以后 Miyu 没回来）：herdr 要「PATH 上的一个命令名，不是路径」。
/// 核心转交 `miyu tui` 来的（`MIYU_PROGRAM` 是核心的路径）照人敲的那个命令写 `<名字> tui`，不然写终端自己的名字；PATH 上找不到
/// 那个名字的不报恢复命令（报了 herdr 重启时也跑不起来，原来写死 `miyu-tui`，装的是 `miyu`、`miyu-remake` 的都跑不起来）。
fn resume_program(var: &impl Fn(&str) -> Option<String>) -> Option<Vec<String>> {
    let (name, args) = match var("MIYU_PROGRAM").filter(|p| !p.is_empty()) {
        Some(program) => {
            let name = std::path::Path::new(&program)
                .file_name()?
                .to_string_lossy()
                .into_owned();
            (name, vec!["tui".to_string()])
        }
        None => (env!("CARGO_PKG_NAME").to_string(), Vec::new()),
    };
    let path = var("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path).any(|dir| dir.join(&name).is_file());
    found.then(|| std::iter::once(name).chain(args).collect())
}

/// 界面退出（崩了也算）：等排着的报完，进程一没，丢出去的子进程也跟着没了。退还只在人真退出时（[`Herdr::release`]）：
/// 终端被挂断（herdr 关了、关机）退出的不退还，留着恢复命令给 herdr 重启时用（2026-11-02 项目主人）。
impl Drop for Herdr {
    fn drop(&mut self) {
        self.reports.take();
        if let Some(worker) = self.worker.take() {
            let _joined = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Herdr, State};

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    #[test]
    fn outside_herdr_nothing_happens() {
        assert!(Herdr::detect(env(&[])).is_none());
        assert!(Herdr::detect(env(&[("HERDR_ENV", "1"), ("HERDR_PANE_ID", "p1")])).is_none());
        assert!(
            Herdr::detect(env(&[
                ("HERDR_PANE_ID", "p1"),
                ("HERDR_BIN_PATH", "/bin/herdr")
            ]))
            .is_none()
        );
    }

    #[test]
    fn reports_carry_a_rising_sequence_from_the_clock() {
        let mut h = Herdr::detect(env(&[
            ("HERDR_ENV", "1"),
            ("HERDR_PANE_ID", "p1"),
            ("HERDR_BIN_PATH", "/nonexistent/herdr"),
        ]))
        .unwrap();
        let first = h.report_args(State::Working);
        assert_eq!(
            first[..9],
            [
                "pane",
                "report-agent",
                "p1",
                "--source",
                "custom:miyu",
                "--agent",
                "miyu",
                "--state",
                "working"
            ]
        );
        let seq: u64 = first[10].parse().unwrap();
        assert!(seq > 1_700_000_000_000, "从毫秒数起：{seq}");
        let release = h.release_args();
        assert_eq!(release[1], "release-agent");
        assert_eq!(
            release[8].parse::<u64>().unwrap(),
            seq + 1,
            "退还也带序号，往上加"
        );
    }
}

#[cfg(test)]
mod resume_tests {
    use super::{Herdr, State};
    use std::sync::mpsc;

    #[test]
    fn switches_clear_and_exit_are_queued_in_order() {
        let (tx, rx) = mpsc::channel();
        let mut h = Herdr {
            reports: Some(tx),
            worker: None,
            session: None,
            resume: Some(vec!["miyu".into(), "tui".into()]),
            pane: "p1".into(),
            next: 100,
            last: None,
        };
        h.report(State::Idle);
        h.session(Some("first"));
        h.report(State::Working);
        h.session(Some("second"));
        h.session(None);
        h.release();
        drop(h);
        let reports: Vec<_> = rx.into_iter().collect();
        assert_eq!(reports.len(), 7);
        fn tail(id: &str) -> [&str; 7] {
            [
                "--agent-session-id",
                id,
                "--",
                "miyu",
                "tui",
                "--resume",
                id,
            ]
        }
        assert_eq!(&reports[1][11..], tail("first"));
        assert_eq!(&reports[2][11..], tail("first"));
        assert_eq!(&reports[3][11..], tail("second"));
        assert_eq!(reports[4][1], "release-agent");
        assert_eq!(reports[5].len(), 11);
        assert_eq!(reports[6][1], "release-agent");
        let seq: Vec<u64> = reports
            .iter()
            .map(|r| {
                r[r.iter().position(|s| s == "--seq").unwrap() + 1]
                    .parse()
                    .unwrap()
            })
            .collect();
        assert!(seq.windows(2).all(|s| s[0] < s[1]));
    }
}

#[cfg(test)]
mod program_tests {
    use super::resume_program;

    /// 恢复命令照人敲的那个命令名写，PATH 上找不到的不报（2026-11-02 项目主人：herdr 重启以后 Miyu 没回来）。
    #[test]
    fn the_resume_command_is_the_name_the_person_typed_and_must_be_on_path() {
        let dir = std::env::temp_dir().join(format!("miyu-herdr-path-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("miyu-remake"), b"").unwrap();
        std::fs::write(dir.join(env!("CARGO_PKG_NAME")), b"").unwrap();
        let path = dir.to_string_lossy().into_owned();
        let var = |program: Option<&str>| {
            let path = path.clone();
            let program = program.map(str::to_string);
            move |name: &str| match name {
                "PATH" => Some(path.clone()),
                "MIYU_PROGRAM" => program.clone(),
                _ => None,
            }
        };
        assert_eq!(
            resume_program(&var(Some("/home/a/.local/bin/miyu-remake"))),
            Some(vec!["miyu-remake".to_string(), "tui".to_string()]),
            "核心转交来的写核心的名字加 tui"
        );
        assert_eq!(
            resume_program(&var(Some("/opt/x/miyu"))),
            None,
            "PATH 上没有的不报"
        );
        assert_eq!(
            resume_program(&var(None)),
            Some(vec![env!("CARGO_PKG_NAME").to_string()]),
            "直接开的写终端自己的名字"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
