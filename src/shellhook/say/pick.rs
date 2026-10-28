//! 在 shell 里原地选（图纸 `shellhook.md`「斜杠命令」第 3 条，2026-10-20 项目主人：「我是 shellhook 啊……每次运行都进 TUI
//! 太不合理了」）：提示符下面画一个框（同终端界面输入框上面那种框，`ui::draw_panel`），`↑` `↓` 选、`Enter` 定、`Esc` 走，
//! 定了以后框收掉、只留一行结果。接上终端会话、读配置、配色照说话那一套（借一个不说话的 [`Turn`]）。

pub mod edit;
pub mod effort;
pub mod fresh;
pub mod help;
pub mod model;
pub mod persona;
pub mod run;
pub mod sessions;
pub mod status;
pub mod theme;
pub mod usage;

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
    /// 只看的（`/status`）：印这一个框就完。
    Show(Chrome, Vec<Line<'static>>),
    /// 照借来的那一轮的正文印（`/undo` 的撤销那一行、`/recap` 的回顾：核心推进正文的，照说话时的样子）。
    Print,
}

/// 一种列表。
pub trait Picker {
    /// 起来时向核心要列表。
    fn start(&mut self, core: &Core);
    /// 核心来的一件（说话那一套办之前先看一眼）。
    fn core(&mut self, update: &Update, turn: &Turn);
    /// 每一拍看一次：带了参数的（`/model 名字`）列表到了就直接定；要等终端会话读回来再问的，这时问。
    fn ready(&mut self, turn: &mut Turn, core: &Core, config: &Config) -> Picked;
    /// 一个键（挪到哪一种当场换样子的，改 `turn.look`）。
    fn key(&mut self, key: KeyEvent, turn: &mut Turn, core: &Core, config: &Config) -> Picked;
    /// 在打搜索的字（按键模式按了 `/`）：这时 `j`、`k` 是字，不当上下。
    fn typing(&self) -> bool {
        false
    }
    /// 要这个终端已经有终端会话（撤销、看用量这些）：还没说过话的不办（核心 T-22 只问不开）。列表、配色、帮助、`/new` 不用。
    fn needs_session(&self) -> bool {
        true
    }
    /// 画成框：框上的字、最多 `max` 行。
    fn view(
        &self,
        turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>);
}

/// 定了以后留的那一行：缩进同她的回答，暗色。
pub fn result(text: &str) -> Line<'static> {
    Line::from(ratatui::text::Span::styled(
        format!("  {text}"),
        crate::theme::dim(),
    ))
}

/// 留的那一行放不下的折成几行，续行和第一行一样缩进两格（2026-10-21 项目主人：长的那一行原来不折，截掉了）。
fn wrapped(line: &Line<'static>, width: u16) -> Vec<Line<'static>> {
    use unicode_width::UnicodeWidthChar;
    let style = line.spans.first().map(|s| s.style).unwrap_or_default();
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let body = text.strip_prefix("  ").unwrap_or(&text);
    let room = usize::from(width).saturating_sub(2).max(1);
    let mut out = Vec::new();
    // 字里的换行照换（一行一个的列表）。
    for part in body.split('\n') {
        let mut row = String::new();
        let mut used = 0;
        for c in part.chars() {
            let w = c.width().unwrap_or(0);
            if used + w > room && !row.is_empty() {
                out.push(Line::styled(format!("  {row}"), style));
                row.clear();
                used = 0;
            }
            row.push(c);
            used += w;
        }
        out.push(Line::styled(format!("  {row}"), style));
    }
    out
}

/// 框两边各让几格（同终端界面的框）。
const SIDE: u16 = 2;

impl Session {
    /// 接上终端会话，画 `picker` 到定了或者不选了；定了的那一行（只看的那个框）留在滚动区。
    ///
    /// # Errors
    ///
    /// 写终端失败。
    pub fn pick(&mut self, turn: &mut Turn, picker: &mut dyn Picker) -> io::Result<()> {
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Hide)?;
        turn.ask_terminal(&self.core);
        picker.start(&self.core);
        let spinner = Duration::from_millis(self.config.timeline.spinner_ms.max(16));
        let end = loop {
            let first = match self.incoming.recv_timeout(spinner) {
                Ok(first) => Some(first),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break Picked::Done(None),
            };
            let rest: Vec<Incoming> = self.incoming.try_iter().collect();
            let mut end = Picked::Stay;
            for incoming in first.into_iter().chain(rest) {
                if !matches!(end, Picked::Stay) {
                    continue;
                }
                match incoming {
                    Incoming::Core(update) => {
                        // 终端会话的推送包着会话编号来（`Elsewhere`），说话那一套拆开了办：列表也照拆开的看。
                        let inner = match &update {
                            Update::Elsewhere { session, update }
                                if turn.transcript.session.as_deref() == Some(session.as_str()) =>
                            {
                                update.as_ref()
                            }
                            other => other,
                        };
                        picker.core(inner, turn);
                        turn.core(update, &mut self.config, &self.core);
                    }
                    Incoming::Key(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                        let ctrl_c = key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL);
                        // `Ctrl+J`、`Ctrl+K` 当 `↓`、`↑`（同终端界面的列表，vim 的上下）；按键模式开着、没在打搜索的字，
                        // `j`、`k` 也是。
                        let key = crate::app::vim_step(key, true);
                        let key =
                            crate::app::vim::plain(key, self.config.layout.vim && !picker.typing());
                        end = if ctrl_c {
                            Picked::Done(None)
                        } else {
                            picker.key(key, turn, &self.core, &self.config)
                        };
                    }
                    Incoming::Key(_) => {}
                }
            }
            if matches!(end, Picked::Stay) {
                end = picker.ready(turn, &self.core, &self.config);
            }
            // 先看定了没有（核心拒了的原话也在这里），再看连不上核心、会话没了。
            if !matches!(end, Picked::Stay) {
                break end;
            }
            // 这个终端还没说过话（只问不开）：用不着会话的照常办。
            if turn.done && !(turn.terminal.none && !picker.needs_session()) {
                break Picked::Done(None);
            }
            turn.look.apply(&self.config);
            let (width, height) = size();
            let max = usize::from(height / 2).max(3);
            let (chrome, lines) =
                picker.view(turn, &self.config, width.saturating_sub(SIDE * 2), max);
            // 按键模式开着的，提示里的「↑/↓」写成「j/k」（同终端界面的框，`panels::content`）。
            let chrome = chrome.vim(self.config.layout.vim);
            // 还在等的（`/status` 等终端会话读回来）什么都不画。
            let mut live = Vec::new();
            if !lines.is_empty() {
                live.push(String::new());
                live.extend(self.boxed(width, chrome, lines)?);
            }
            self.painter.frame(&[], &live)?;
        };
        self.painter.finish()?;
        turn.look.apply(&self.config);
        let width = size().0;
        let settled = match end {
            Picked::Done(Some(line)) => {
                let mut lines = vec![Line::default()];
                lines.extend(wrapped(&line, width));
                self.painter.lines(width, &lines)?
            }
            Picked::Show(chrome, lines) => {
                let mut settled = vec![String::new()];
                settled.extend(self.boxed(width, chrome, lines)?);
                settled
            }
            Picked::Print => {
                // 只印核心刚推进来的那一条（撤销那一行、回顾）：读回来的那一页前面几轮不再印一遍。
                let last = turn.transcript.entries.pop();
                turn.transcript.entries = last.into_iter().collect();
                let (_, height) = size();
                let view = self.view(turn, width, height, 0);
                let lines = view.lines(&turn.transcript, super::view::Scope::All, false);
                self.painter.lines(width, &lines)?
            }
            Picked::Done(None) | Picked::Stay => Vec::new(),
        };
        self.painter.frame(&settled, &[])?;
        if self.hooked {
            return Ok(());
        }
        ratatui::crossterm::execute!(io::stdout(), ratatui::crossterm::cursor::Show)
    }

    /// 几行画进一个框（同终端界面由命令打开的框：框线里上下各空一行，`panel::PAD`），交回画好的几行。
    fn boxed(
        &self,
        width: u16,
        chrome: Chrome,
        lines: Vec<Line<'static>>,
    ) -> io::Result<Vec<String>> {
        let rows = u16::try_from(lines.len())
            .unwrap_or(u16::MAX)
            .saturating_add(2 + crate::ui::PANEL_PAD);
        self.painter.render(width, rows, |frame| {
            let outer = frame.area();
            let text = crate::ui::panel_padded(Rect::new(
                outer.x + SIDE,
                outer.y + 1,
                outer.width.saturating_sub(SIDE * 2),
                outer.height.saturating_sub(2),
            ));
            crate::ui::draw_panel(frame, outer, text, chrome, lines);
        })
    }
}
