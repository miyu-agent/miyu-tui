//! 确认和提问的抽屉接到程序上（蓝图 `tui.md`「确认和提问的抽屉」）：按键先归它、两下 `Esc` 取消、了结以后正文里
//! 留下结果。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use crate::core::Command;
use crate::drawer::{Drawer, Outcome, Report, Step};
use crate::transcript::{JobMark, Kind};

impl App {
    /// 抽屉开着时按键先归它，带 `Ctrl` 的照旧归外面（复制、退出），编辑时的编辑键除外（第 4 条，[`crate::drawer::ctrl_edit`]）。
    /// 拿了交回 `true`。
    pub(super) fn drawer_key(&mut self, key: KeyEvent) -> bool {
        // PgUp、PgDn 归正文：翻上去看，抽屉让开（「确认和提问的抽屉」第 4a 条，2026-10-26 项目主人：原来翻不了页）。
        if !self.talk.drawers.open() || matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            return false;
        }
        let editing = self.talk.drawers.editing() && crate::drawer::ctrl_edit(key.code);
        // `Ctrl+↑` `Ctrl+↓`、`Ctrl+J` `Ctrl+K` 滚命令那一块（`drawer/keys.rs`）。
        let scroll = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k')
        );
        if key.modifiers.contains(KeyModifiers::CONTROL) && !editing && !scroll {
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

    /// 抽屉开着时粘贴：正在写的（「其他」、补充、理由）插在光标处，大段的照输入框的规矩收成一块（2026-11-27）；没在编辑不收。
    pub(super) fn drawer_paste(&mut self, text: &str) {
        let rule = self.typing.input.paste_rule();
        if let Some(d) = self.talk.drawers.current.as_mut() {
            d.paste(text, rule);
        }
    }

    /// 鼠标在抽屉里按下：落在正在写的那几行字上的，光标挪过去（同输入框），交回 `true`；别处交回 `false`，照点中第几项算。
    pub(super) fn drawer_field_click(&mut self, at: ratatui::layout::Position) -> bool {
        self.talk
            .drawers
            .current
            .as_mut()
            .and_then(crate::drawer::Drawer::field_mut)
            .is_some_and(|field| field.click(at))
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
        let d = self.talk.drawers.current.as_ref()?;
        let armed = d.armed.map(|at| at + self.esc_window());
        // 确认的倒计时跳到下一个整秒时醒来重画（核心 CD-1）。
        let tick = d
            .expires_in(std::time::SystemTime::now())
            .and_then(crate::drawer::expiry::next_tick)
            .map(|wait| Instant::now() + wait);
        armed.into_iter().chain(tick).min()
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
        Report::Denied(text, full) => transcript.job_line(JobMark::Failed, text, full),
        Report::Nothing => return,
    }
    // 记在这一轮名下：撤掉这一轮时跟着藏，恢复时跟着回来。
    transcript.tie_last_to_turn();
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

#[cfg(test)]
mod tests {
    use super::report_into;
    use crate::config::Config;
    use crate::core::{Push, Update};
    use crate::drawer::{Approval, Decided, Decision, Drawer, Outcome};
    use crate::transcript::{Kind, Transcript};

    /// 一轮里确认时选了「不允许」：撤掉这一轮，那一行「× 不允许」跟着藏；恢复，跟着回来。原来那一行不带轮次，撤销以后还在。
    #[test]
    fn a_denial_goes_away_with_its_turn_and_comes_back_with_it() {
        let config = Config::builtin().unwrap();
        let mut t = Transcript::default();
        let push = |t: &mut Transcript, p: Push| t.update(Update::Push(p), &config.text);
        push(&mut t, Push::TurnStarted(7, None));
        let approval: Approval = serde_json::from_value(serde_json::json!({
            "call_id": "c1", "access": "exec",
            "detail": {"tool": "shell", "command": "rm -rf build"}
        }))
        .unwrap();
        let deny = Outcome::Decided(Decided {
            call_id: "c1".into(),
            decision: Decision::Deny,
            reason: Some("不要".into()),
            files: Vec::new(),
        });
        let drawer = Drawer::approval(None, approval);
        report_into(&mut t, &drawer, &deny, &config.text.drawer);
        push(&mut t, Push::TurnEnded(crate::core::EndReason::Completed));
        let denied = |t: &Transcript| {
            t.entries
                .iter()
                .find(|e| e.kind == Kind::Job && e.text.contains("不要"))
                .map(|e| e.hidden)
        };
        assert_eq!(denied(&t), Some(false), "写了那一行：{:?}", t.entries);
        push(&mut t, Push::Reverted(vec![7]));
        assert_eq!(denied(&t), Some(true), "撤掉那一轮，那一行跟着藏");
        push(&mut t, Push::Unreverted(vec![7]));
        assert_eq!(denied(&t), Some(false), "恢复，那一行跟着回来");
    }
}
