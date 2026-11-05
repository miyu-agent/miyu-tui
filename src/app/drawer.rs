//! 确认和提问的抽屉接到程序上（蓝图 `tui.md`「确认和提问的抽屉」）：按键先归它、两下 `Esc` 取消、了结以后正文里
//! 留下结果。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use crate::core::Command;
use crate::drawer::{Drawer, Edit, Outcome, Report, Step};
use crate::transcript::{JobMark, Kind};

impl App {
    /// 抽屉开着时按键先归它，带 `Ctrl` 的照旧归外面（复制、退出），编辑时的 `Ctrl+J` 换行除外（第 4 条）。
    /// 拿了交回 `true`。
    pub(super) fn drawer_key(&mut self, key: KeyEvent) -> bool {
        // PgUp、PgDn 归正文：翻上去看，抽屉让开（「确认和提问的抽屉」第 4a 条，2026-10-26 项目主人：原来翻不了页）。
        if !self.talk.drawers.open() || matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            return false;
        }
        let newline = self.talk.drawers.editing() && key.code == KeyCode::Char('j');
        // `Ctrl+↑` `Ctrl+↓`、`Ctrl+J` `Ctrl+K` 滚命令那一块（`drawer/keys.rs`）。
        let scroll = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k')
        );
        if key.modifiers.contains(KeyModifiers::CONTROL) && !newline && !scroll {
            return false;
        }
        // 抽屉滑下去了、让开了（正文翻上去看，`body_view/slide.rs`）：这一下先回到最底下、抽屉回原位，不当答案用掉，免得
        // 看不见时按到「1」就选了（「确认和提问的抽屉」第 4a 条）。
        if self.screen.view.top.is_some() || self.screen.view.slide.by > 0 {
            self.screen.view.follow();
            return true;
        }
        let window = self.esc_window();
        let step = self
            .talk
            .drawers
            .current
            .as_mut()
            .map_or(Step::Stay, |d| match d.key(key) {
                Step::Escape => d.escape(Instant::now(), window),
                step => step,
            });
        self.drawer_step(step);
        true
    }

    /// 抽屉开着时粘贴：正在写的（「其他」、补充、理由）接进去；没在编辑不收。
    pub(super) fn drawer_paste(&mut self, text: &str) {
        let Some(d) = self.talk.drawers.current.as_mut() else {
            return;
        };
        let tab = d.tab;
        match d.editing {
            Some(Edit::Notes) => d.notes[tab].push_str(text),
            Some(Edit::Other | Edit::Reason) => d.typed[tab].push_str(text),
            None => {}
        }
    }

    /// 点中抽屉的第几项。
    pub(super) fn drawer_click(&mut self, index: usize) {
        let step = self
            .talk
            .drawers
            .current
            .as_mut()
            .map_or(Step::Stay, |d| d.click(index));
        self.drawer_step(step);
    }

    /// 按过一下 `Esc` 的提示什么时候到点消失（主循环照它醒来重画）。
    pub(super) fn drawer_deadline(&self) -> Option<Instant> {
        let at = self.talk.drawers.current.as_ref()?.armed?;
        Some(at + self.esc_window())
    }

    /// 到点了：按过一下 `Esc` 过了时限，提示换回按键提示。
    pub(super) fn drawer_tick(&mut self) {
        let window = self.esc_window();
        if let Some(d) = self.talk.drawers.current.as_mut()
            && !d.armed(Instant::now(), window)
        {
            d.armed = None;
        }
    }

    /// 两下 `Esc` 之间的时限：和别处的两下 `Esc` 一样。
    fn esc_window(&self) -> Duration {
        Duration::from_millis(self.config.layout.esc_window_ms)
    }

    /// 按了一下以后：了结的留下结果。
    fn drawer_step(&mut self, step: Step) {
        if let Step::Done(outcome) = step {
            self.settle_drawer(&outcome);
        }
    }

    /// 了结：交了的经 `session.answer` 交给问的那个会话，正文的结果等核心推来回答再写（`asking.rs`）；演示的、取消的
    /// 当场写：提问答了是引用块，不允许、取消是一行，允许了什么都不留（第 6 条）；取消的在回答时顺带打断这一轮（第 5 条）。
    fn settle_drawer(&mut self, outcome: &Outcome) {
        let Some(drawer) = self.talk.drawers.finish() else {
            return;
        };
        let answered = match outcome {
            Outcome::Answered(a) => Some(serde_json::json!({"answers": a.answers})),
            Outcome::Decided(d) => {
                Some(serde_json::json!({"decision": d.decision, "reason": d.reason}))
            }
            Outcome::Cancelled => None,
        };
        match answered {
            Some(mut body) => {
                if body["reason"].is_null()
                    && let Some(fields) = body.as_object_mut()
                {
                    fields.remove("reason");
                }
                // 交给问的那个会话：`/new`、切会话以后还开着的抽屉也不会交错地方。
                let owner = Some(drawer.owner.clone()).filter(|o| !o.is_empty());
                self.link.core.send(Command::Answer {
                    session: owner.or_else(|| drawer.session.clone()),
                    call: drawer.call_id.clone(),
                    body,
                    files: answer_files(outcome),
                });
            }
            None => self.write_report(&drawer, outcome),
        }
        if *outcome == Outcome::Cancelled
            && self.talk.transcript.running.is_some()
            && !self.switching()
        {
            self.link.core.send(Command::Interrupt { send: true });
        }
        // 轮到下一个抽屉的弹「在等你」，都了结了回到在做或空闲（「系统通知」第 1、6 条）。
        if self.talk.drawers.open() {
            self.notify_drawer();
        } else {
            self.settle_state();
        }
    }
}

impl App {
    /// 正文末尾写一问的结果（第 6 条）。
    pub(super) fn write_report(&mut self, drawer: &crate::drawer::Drawer, outcome: &Outcome) {
        report_into(
            &mut self.talk.transcript,
            drawer,
            outcome,
            &self.config.text.drawer,
        );
    }
}

/// 在 `transcript` 里写一问的结果：回答不进时间线，另起一块照你说的话的样子画（`ui/answered_rows.rs`，2026-10-26 项目
/// 主人定）；不允许的末尾一行红 `✗`（第 6 条）。按页读更早的一页时写进那一份临时的正文（`pages.rs`）。
pub(crate) fn report_into(
    transcript: &mut crate::transcript::Transcript,
    drawer: &Drawer,
    outcome: &Outcome,
    texts: &crate::drawer::Texts,
) {
    match drawer.report(outcome, texts) {
        Report::Answers(lines) => {
            let head = texts.answered_head.replace("{n}", &lines.len().to_string());
            let text = std::iter::once(head)
                .chain(lines)
                .collect::<Vec<_>>()
                .join("\n");
            transcript.note(Kind::Answered, text);
        }
        Report::Denied(text, full) => transcript.job(JobMark::Failed, text, full),
        Report::Nothing => {}
    }
}

/// 回答里贴的图：第几道的（拒绝的是外面的）、本机的文件，照贴的先后。shellhook 的抽屉也照它交。
pub(crate) fn answer_files(outcome: &Outcome) -> Vec<(Option<usize>, std::path::PathBuf)> {
    match outcome {
        Outcome::Answered(a) => a
            .answers
            .iter()
            .enumerate()
            .flat_map(|(i, answer)| answer.files.iter().map(move |f| (Some(i), f.file.clone())))
            .collect(),
        Outcome::Decided(d) => d.files.iter().map(|f| (None, f.file.clone())).collect(),
        Outcome::Cancelled => Vec::new(),
    }
}
