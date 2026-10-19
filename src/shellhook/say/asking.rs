//! 确认和提问（图纸 `shellhook.md`「确认和提问」，设计 22 第七节「当场在 shell 里问你」）：抽屉照终端界面那一套（数据
//! `drawer/`、画法 `ui/drawer/`），画在活动区最下面的圆角框里；答完收起，结果照全屏写进正文（蓝图 `tui.md`「确认和提问的
//! 抽屉」第 6 条）。只认终端会话自己问的，子代理问的照全屏也只在它自己的会话里弹。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::{Block, BorderType};
use serde_json::json;

use crate::config::Config;
use crate::core::{Asking as Asked, Command, Core, Push};
use crate::drawer::{Answered, Approval, Asks, Decided, Drawer, Drawers, Outcome, Report, Step};
use crate::transcript::{JobMark, Kind, Transcript};

/// 开着的、排着的抽屉，问过了哪些。
#[derive(Default)]
pub struct Asking {
    drawers: Drawers,
    asks: Asks,
}

impl Asking {
    /// 有开着的抽屉。
    pub fn open(&self) -> bool {
        self.drawers.open()
    }

    /// 开着的抽屉是谁、问的什么：`call_id` 和是不是提问（新开了一个时弹通知）。
    pub fn current(&self) -> Option<(String, bool)> {
        self.drawers
            .current
            .as_ref()
            .map(|d| (d.call_id.clone(), d.is_question()))
    }

    /// 终端会话 `owner` 推来的：确认、提问开抽屉（交回 `true`，不再交给正文）；落了盘的回答收起抽屉、写进正文；工具有了
    /// 结果的收起它的抽屉（照常交给正文）。
    pub fn push(
        &mut self,
        owner: &str,
        push: &Push,
        transcript: &mut Transcript,
        config: &Config,
    ) -> bool {
        match push {
            Push::Asking(asked) => {
                self.asked(owner, asked, transcript, config);
                true
            }
            Push::ToolResult { call_id, .. } => {
                self.asks.settle(owner, call_id);
                self.drawers.settle(owner, call_id);
                false
            }
            _ => false,
        }
    }

    fn asked(&mut self, owner: &str, asked: &Asked, transcript: &mut Transcript, config: &Config) {
        let drawer = match asked {
            Asked::Asked(body) => serde_json::from_value(body.clone())
                .ok()
                .map(|a| Drawer::question(None, a)),
            Asked::Approval(body) => serde_json::from_value::<Approval>(body.clone())
                .ok()
                .map(|a| Drawer::approval(None, a)),
            Asked::Answered(body) => {
                if let Ok(a) = serde_json::from_value::<Answered>(body.clone()) {
                    let call = a.call_id.clone();
                    self.answered(owner, &call, &Outcome::Answered(a), transcript, config);
                }
                return;
            }
            Asked::Decided(body) => {
                if let Ok(d) = serde_json::from_value::<Decided>(body.clone()) {
                    let call = d.call_id.clone();
                    self.answered(owner, &call, &Outcome::Decided(d), transcript, config);
                }
                return;
            }
        };
        let Some(mut drawer) = drawer else { return };
        drawer.owner = owner.to_string();
        // 跑命令的确认：核心没带短标题、命令的，照时间线里这一步的参数写（同全屏 `app/asking.rs`）。
        if drawer.command.is_none()
            && let Some(args) = transcript.tool_args(&drawer.call_id)
            && let Some(command) = args["command"].as_str()
        {
            let title = args["description"]
                .as_str()
                .filter(|t| !t.trim().is_empty());
            drawer.command = Some((title.map(str::to_string), command.to_string()));
        }
        if self.asks.asked(&drawer) {
            self.drawers.push(drawer);
        }
    }

    /// 落了盘的回答（哪个头答的都算）：收起抽屉，结果写进正文。
    fn answered(
        &mut self,
        owner: &str,
        call: &str,
        outcome: &Outcome,
        transcript: &mut Transcript,
        config: &Config,
    ) {
        let asked = self.asks.settle(owner, call);
        self.drawers.settle(owner, call);
        if let Some(drawer) = asked {
            report(transcript, &drawer, outcome, config);
        }
    }

    /// 抽屉开着时按键先归它，带 Ctrl 的照旧归外面（Ctrl+C 打断），编辑时的 Ctrl+J 换行除外。拿了交回 `true`。
    pub fn key(
        &mut self,
        key: KeyEvent,
        core: &Core,
        transcript: &mut Transcript,
        config: &Config,
    ) -> bool {
        if !self.drawers.open() {
            return false;
        }
        let newline = self.drawers.editing() && key.code == KeyCode::Char('j');
        if key.modifiers.contains(KeyModifiers::CONTROL) && !newline {
            return false;
        }
        let window = Duration::from_millis(config.layout.esc_window_ms);
        let step = self
            .drawers
            .current
            .as_mut()
            .map_or(Step::Stay, |d| match d.key(key) {
                Step::Escape => d.escape(Instant::now(), window),
                step => step,
            });
        if let Step::Done(outcome) = step {
            self.settle(&outcome, core, transcript, config);
        }
        true
    }

    /// 粘贴：正在写的（其他答案、补充、理由）接进去。
    pub fn paste(&mut self, text: &str) -> bool {
        let Some(d) = self.drawers.current.as_mut() else {
            return false;
        };
        let tab = d.tab;
        match d.editing {
            Some(crate::drawer::Edit::Notes) => d.notes[tab].push_str(text),
            Some(crate::drawer::Edit::Other | crate::drawer::Edit::Reason) => {
                d.typed[tab].push_str(text)
            }
            None => {}
        }
        true
    }

    /// 了结：交了的经 `session.answer` 交给问的会话，结果等核心推回来再写；取消的当场写，顺带打断这一轮（同全屏
    /// `app/drawer.rs`）。
    fn settle(
        &mut self,
        outcome: &Outcome,
        core: &Core,
        transcript: &mut Transcript,
        config: &Config,
    ) {
        let Some(drawer) = self.drawers.finish() else {
            return;
        };
        let body = match outcome {
            Outcome::Answered(a) => Some(json!({"answers": a.answers})),
            Outcome::Decided(d) => Some(match &d.reason {
                Some(reason) => json!({"decision": d.decision, "reason": reason}),
                None => json!({"decision": d.decision}),
            }),
            Outcome::Cancelled => None,
        };
        match body {
            Some(body) => core.send(Command::Answer {
                session: Some(drawer.owner.clone()).filter(|o| !o.is_empty()),
                call: drawer.call_id.clone(),
                body,
            }),
            None => report(transcript, &drawer, outcome, config),
        }
        if *outcome == Outcome::Cancelled && transcript.running.is_some() {
            core.send(Command::Interrupt { send: true });
        }
    }

    /// 开着的抽屉画进 `width` 宽的圆角框，最多 `max` 行高；交回画布要多高、怎么画（交给 `Painter::render`）。
    pub fn height(&self, config: &Config, width: u16, max: u16) -> Option<u16> {
        let d = self.drawers.current.as_ref()?;
        let inner = crate::ui::drawer_rows(
            d,
            &config.text.drawer,
            &config.layout.drawer_preview,
            width.saturating_sub(4),
            max.saturating_sub(2),
        );
        Some(inner + 2)
    }

    /// 画进 `area`：圆角框、框里的抽屉（同全屏 `ui/input_box.rs`）。
    pub fn draw(&mut self, frame: &mut ratatui::Frame, area: Rect, config: &Config) {
        let Some(d) = self.drawers.current.as_mut() else {
            return;
        };
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(crate::theme::dim()),
            area,
        );
        let text = Rect::new(
            area.x + 2,
            area.y + 1,
            area.width.saturating_sub(4),
            area.height.saturating_sub(2),
        );
        let mut caret = crate::caret::Caret::default();
        crate::ui::draw_drawer(
            frame,
            text,
            d,
            &config.text.drawer,
            &config.layout.drawer_preview,
            &mut caret,
        );
    }
}

/// 一问的结果写进正文：回答挂到「提问」那一步下面，找不到那一步的写在末尾；不允许的末尾一行红 ✗（同全屏 `app/drawer.rs`
/// 的 `report_into`）。
fn report(transcript: &mut Transcript, drawer: &Drawer, outcome: &Outcome, config: &Config) {
    match drawer.report(outcome, &config.text.drawer) {
        Report::Answers(lines) => {
            if !transcript.answered(&drawer.call_id, lines.clone()) {
                transcript.note(Kind::Answered, lines.join("\n"));
            }
        }
        Report::Denied(text) => transcript.job(JobMark::Failed, text, String::new()),
        Report::Nothing => {}
    }
}
