//! 只办一条命令（蓝图 `tui.md`「只办一条命令」，2026-10-20 和 shellhook 会话商定）：`--resume <会话> --command <一行>`
//! 起来，进那个会话、连上以后等于在输入框里敲了这一行回车（走 `keys.rs` 的 `run()`，不另开一条路）；这条命令开的框关掉
//! 就退出，开框以前等着。会话换了（`/sessions` 选了别的）的，把最后所在的会话编号写进 `--report` 给的文件，shellhook 照它
//! 指终端会话（`session.set_terminal`）。界面往标准输出画，编号不能打在那里。

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::App;
use crate::commands::Line;

/// 命令跑了以后等框开多久：开框的命令要先等核心回一份（模型列表、会话列表）。过了还没开的照办完了退出。
const OPEN_WAIT: Duration = Duration::from_secs(10);

/// 一条命令办到哪了。
#[derive(Debug)]
pub struct OneShot {
    /// 要办的那一行（`/model`）。
    line: String,
    /// 会话编号写进哪个文件。
    report: Option<PathBuf>,
    /// 起来时进的会话。
    from: Option<String>,
    /// 什么时候办的；还没办的是 `None`。
    ran: Option<Instant>,
    /// 框开过了。
    opened: bool,
    /// 没开框、弹了一句提示（核心拒了、「先选模型」这类）：提示到这一刻消失，就退出。
    hinted: Option<Instant>,
}

impl App {
    /// 起来时记下要办的那一行：连上、进了会话再办（[`App::one_shot_tick`]）。
    pub fn one_shot(&mut self, line: String, report: Option<PathBuf>, from: Option<String>) {
        self.one_shot = Some(OneShot {
            line,
            report,
            from,
            ran: None,
            opened: false,
            hinted: None,
        });
    }

    /// 只办一条命令的：每一拍看一次。连上、进了会话就办；办了以后框开了又关了（或者等了一阵也没开），退出。
    pub fn one_shot_tick(&mut self) {
        let Some(shot) = self.one_shot.as_ref() else {
            return;
        };
        let Some(ran) = shot.ran else {
            let ready = matches!(self.talk.transcript.link, crate::transcript::Link::Ready)
                && self.talk.opening.is_none()
                && self.talk.main_session().is_some();
            if ready {
                self.one_shot_run();
            }
            return;
        };
        let open = self.any_box_open();
        let opened = shot.opened || open;
        // 没开框、弹了提示的（同终端界面的提示，停 `layout.json` 的那么久）：提示看完就退，不干等（2026-10-20 终端界面会话审）。
        let hinted = shot
            .hinted
            .or_else(|| (!opened).then(|| self.typing.notice.as_ref().map(|n| n.until))?);
        if let Some(shot) = self.one_shot.as_mut() {
            shot.opened = opened;
            shot.hinted = hinted;
        }
        let now = Instant::now();
        // 框关了还在切会话的（`/sessions` 选了别的）：切完再走，退出时才报得出新的编号。
        let done = if opened {
            !open && self.talk.opening.is_none()
        } else {
            hinted.is_some_and(|until| now >= until) || ran.elapsed() >= OPEN_WAIT
        };
        if done {
            self.quit = true;
        }
    }

    /// 办那一行：是终端的命令的照 `run()` 办；不是的（话、核心的命令模板）不办，退出。
    fn one_shot_run(&mut self) {
        let Some(line) = self.one_shot.as_ref().map(|s| s.line.clone()) else {
            return;
        };
        if let Some(shot) = self.one_shot.as_mut() {
            shot.ran = Some(Instant::now());
        }
        let commands = self.config.commands.clone();
        match commands.read(line.trim()) {
            Line::Command(spec, words) => {
                self.run(spec, words);
                // 当场开的框（`/status`）这一拍就记下：下一次看可能已经是按了 Esc 以后（测出来的）。
                let open = self.any_box_open();
                if let Some(shot) = self.one_shot.as_mut() {
                    shot.opened = open;
                }
            }
            Line::Unknown | Line::Talk => {
                // 标准输出是画界面的：为什么没办写在标准错误，shellhook 那边好查。
                eprintln!("not a terminal command: {line}");
                self.quit = true;
            }
        }
    }

    /// 开着框、整屏页：命令开的那几样；提问、确认的抽屉开着也算（她正好来问一句，答了再走）。
    fn any_box_open(&self) -> bool {
        let b = &self.boxes;
        self.talk.drawers.open()
            || b.panel.is_some()
            || b.session_list.is_some()
            || b.model_list.is_some()
            || b.usage.is_some()
            || b.settings.is_some()
            || b.theme_box.is_some()
            || b.widget_page.is_some()
    }

    /// 办了还没开框的：到这一刻照办完了退出。
    pub(super) fn one_shot_deadline(&self) -> Option<Instant> {
        let shot = self.one_shot.as_ref()?;
        let ran = shot.ran.filter(|_| !shot.opened)?;
        Some(shot.hinted.unwrap_or(ran + OPEN_WAIT))
    }

    /// 只办一条命令的不进引导（同 `--page config`）。
    pub(super) fn one_shot_mode(&self) -> bool {
        self.one_shot.is_some()
    }

    /// 退出时：会话换了的，把最后所在的会话编号写进 `--report` 给的文件。
    pub fn one_shot_report(&self) -> std::io::Result<()> {
        let Some(shot) = self.one_shot.as_ref() else {
            return Ok(());
        };
        let now = self.talk.main_session();
        match (&shot.report, now) {
            (Some(path), Some(now)) if Some(&now) != shot.from.as_ref() => {
                std::fs::write(path, now)
            }
            _ => Ok(()),
        }
    }
}
