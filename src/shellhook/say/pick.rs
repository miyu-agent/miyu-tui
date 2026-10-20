//! 在 shell 里原地选（图纸 `shellhook.md`「斜杠命令」第 3 条，2026-10-20 项目主人：「我是 shellhook 啊……每次运行都进 TUI
//! 太不合理了」）：提示符下面画一个框（同终端界面输入框上面那种框，`ui::draw_panel`），`↑` `↓` 选、`Enter` 定、`Esc` 走，
//! 定了以后框收掉、只留一行结果。接上终端会话、读配置、配色照说话那一套（借一个不说话的 [`Turn`]）。

pub mod model;

use std::io;
use std::time::Duration;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::text::Line;

use super::session::{Session, size};
use super::turn::{Incoming, Turn};
use crate::config::Config;
use crate::core::{Core, Update};
use crate::ui::Chrome;

/// 按了一个键、核心来了一件以后。
pub enum Picked {
    /// 接着选。
    Stay,
    /// 定了（留这一行）；`None` 是不选了、什么都不留。
    Done(Option<Line<'static>>),
}

/// 一种列表。
pub trait Picker {
    /// 起来时向核心要列表。
    fn start(&mut self, core: &Core);
    /// 核心来的一件（说话那一套办之前先看一眼）。
    fn core(&mut self, update: &Update, turn: &Turn);
    /// 每一拍看一次：带了参数的（`/model 名字`）列表到了就直接定。
    fn ready(&mut self, turn: &Turn, core: &Core) -> Picked;
    /// 一个键。
    fn key(&mut self, key: KeyEvent, turn: &Turn, core: &Core) -> Picked;
    /// 画成框：框上的字、最多 `max` 行。
    fn view(
        &self,
        turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>);
}

/// 框两边各让几格（同终端界面的框）。
const SIDE: u16 = 2;

impl Session {
    /// 接上终端会话，画 `picker` 到定了或者不选了；定了的那一行留在滚动区。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn pick(&mut self, turn: &mut Turn, picker: &mut dyn Picker) -> io::Result<()> {
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Hide)?;
        turn.ask_terminal(&self.core);
        picker.start(&self.core);
        let spinner = Duration::from_millis(self.config.timeline.spinner_ms.max(16));
        let result = loop {
            let first = match self.incoming.recv_timeout(spinner) {
                Ok(first) => Some(first),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break None,
            };
            let rest: Vec<Incoming> = self.incoming.try_iter().collect();
            let mut done = None;
            for incoming in first.into_iter().chain(rest) {
                if done.is_some() {
                    continue;
                }
                match incoming {
                    Incoming::Core(update) => {
                        picker.core(&update, turn);
                        turn.core(update, &mut self.config, &self.core);
                    }
                    Incoming::Key(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                        let ctrl_c = key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL);
                        if ctrl_c {
                            done = Some(None);
                        } else if let Picked::Done(line) = picker.key(key, turn, &self.core) {
                            done = Some(line);
                        }
                    }
                    Incoming::Key(_) => {}
                }
            }
            if done.is_none()
                && let Picked::Done(line) = picker.ready(turn, &self.core)
            {
                done = Some(line);
            }
            // 连不上核心、会话没了：不选了。
            if turn.done {
                break None;
            }
            if let Some(line) = done {
                break line;
            }
            turn.look.apply(&self.config);
            let (width, height) = size();
            let max = usize::from(height / 2).max(3);
            let (chrome, lines) =
                picker.view(turn, &self.config, width.saturating_sub(SIDE * 2), max);
            let rows = u16::try_from(lines.len())
                .unwrap_or(u16::MAX)
                .saturating_add(2);
            let mut live = vec![String::new()];
            live.extend(self.painter.render(width, rows, |frame| {
                let outer = frame.area();
                let text = Rect::new(
                    outer.x + SIDE,
                    outer.y + 1,
                    outer.width.saturating_sub(SIDE * 2),
                    outer.height.saturating_sub(2),
                );
                crate::ui::draw_panel(frame, outer, text, chrome, lines);
            })?);
            self.painter.frame(&[], &live)?;
        };
        self.painter.finish()?;
        let settled = match result {
            Some(line) => self.painter.lines(width_now(), &[Line::default(), line])?,
            None => Vec::new(),
        };
        self.painter.frame(&settled, &[])?;
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Show)
    }
}

fn width_now() -> u16 {
    size().0
}
