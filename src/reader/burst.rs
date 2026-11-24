//! 不发括号粘贴的终端，粘贴进来的是一串按键（蓝图 `tui.md`「输入框」第 11 条）：crossterm 在 Windows 上读控制台的输入
//! 记录，根本不产生 `Event::Paste`，粘进来的每一行末尾的回车就是按了回车，多行的字被拆成好几条消息发出去（2026-11-24
//! 用户报）。Unix 上不发括号粘贴的终端也一样。
//!
//! 这里照到达的快慢认：一个写字的键（字、回车、Tab）先攒着，[`CHAR_GAP`] 里来了下一个的接着攒，一串里相邻两个都在
//! [`STREAM_GAP`] 以内就算一口气到的；停下来以后，攒了至少 [`LEAST`] 个、里面有回车的，整串当一次粘贴交回（回车是换行、
//! 不是发送），别的照原样一个一个交回。人打字两个键之间少有快过 8 毫秒的，回车更不会紧跟在字后面这么快，正常打字认不成
//! 粘贴；一串不从回车开始（打头的回车是发送）；没有回车的一串照原样交回，打得快的、输入法一次上屏的字都和原来一样。每个写字的键最多晚 [`CHAR_GAP`] 交出去。
//!
//! 门槛照 Codex 的同一处（`codex-rs/tui/src/bottom_pane/paste_burst.rs`：相邻 8 毫秒、至少 3 个，Windows 上一串的间隔放宽
//! 到 60 毫秒，「Windows 上见过更慢的粘贴」）。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::Read;

/// 第一个键之后多久来了下一个，才算开始一口气到的一串。
pub const CHAR_GAP: Duration = Duration::from_millis(8);

/// 已经是一串了，相邻两个最多隔多久还算这一串。
pub const STREAM_GAP: Duration = Duration::from_millis(if cfg!(windows) { 60 } else { 8 });

/// 一串至少几个键才可能是粘贴。
const LEAST: usize = 3;

/// 正在攒的一串。
#[derive(Default)]
pub struct Burst {
    /// 攒着的原样的按键（不当粘贴时放回去）。
    held: Vec<Event>,
    /// 这一串写出来的字。
    text: String,
    /// 有没有回车。
    newline: bool,
    /// 上一个是回车（认 `\r\n`）。
    returned: bool,
    /// 最后一个键什么时候到的。
    last: Option<Instant>,
}

impl Burst {
    /// 在攒的话，到什么时候还等不到下一个就收（[`Burst::expire`]）。
    pub fn deadline(&self) -> Option<Instant> {
        let last = self.last?;
        Some(
            last + if self.held.len() > 1 {
                STREAM_GAP
            } else {
                CHAR_GAP
            },
        )
    }

    /// 读到一个事件（`now` 是它到的时刻）：交回该交给主循环的。
    pub fn feed(&mut self, read: Read, now: Instant) -> Vec<Read> {
        let Read::Event(event) = read else {
            return vec![read];
        };
        // 松开（Windows 上每个键都报）不打断一串，照旧交出去，主循环本来就不理。
        if matches!(&event, Event::Key(k) if k.kind == KeyEventKind::Release) {
            return vec![Read::Event(event)];
        }
        let Some(piece) = written(&event) else {
            let mut out = self.expire();
            out.push(Read::Event(event));
            return out;
        };
        let mut out = Vec::new();
        if self.deadline().is_some_and(|until| now > until) {
            out = self.expire();
        }
        // 一串不从回车开始：打头的回车是发送，当场交出去（发完马上又有字进来的，例如别的程序替人打的下一句，不并成一段）。
        if self.held.is_empty() && matches!(piece, Piece::Return | Piece::Newline) {
            out.push(Read::Event(event));
            return out;
        }
        self.add(event, piece);
        self.last = Some(now);
        out
    }

    /// 到点了等不到下一个（或者要停下读）：这一串收掉，交回该交的。
    pub fn expire(&mut self) -> Vec<Read> {
        self.last = None;
        self.returned = false;
        let text = std::mem::take(&mut self.text);
        let held = std::mem::take(&mut self.held);
        if std::mem::take(&mut self.newline) && held.len() >= LEAST {
            return vec![Read::Event(Event::Paste(text))];
        }
        held.into_iter().map(Read::Event).collect()
    }

    /// 攒进一个键。Windows 上粘进来的 `\r\n` 是一个回车跟一个 `\n`：只算一个换行。
    fn add(&mut self, event: Event, piece: Piece) {
        match piece {
            Piece::Char(c) => self.text.push(c),
            Piece::Newline if self.returned => {}
            Piece::Return | Piece::Newline => {
                self.text.push('\n');
                self.newline = true;
            }
        }
        self.returned = matches!(piece, Piece::Return);
        self.held.push(event);
    }
}

/// 一个写字的键写出来的。
#[derive(Clone, Copy)]
enum Piece {
    /// 字、Tab。
    Char(char),
    /// 回车键。
    Return,
    /// 一个 `\n`（`Ctrl+J`）：紧跟在回车后面的和它算一个。
    Newline,
}

/// 写字的键写出来的；别的事件（方向键、带 Ctrl、Alt 的、鼠标、括号粘贴）是 `None`。
fn written(event: &Event) -> Option<Piece> {
    let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event
    else {
        return None;
    };
    let plain = (*modifiers - KeyModifiers::SHIFT).is_empty();
    match code {
        KeyCode::Char('\n' | 'j') if *modifiers == KeyModifiers::CONTROL => Some(Piece::Newline),
        KeyCode::Char('\n') if plain => Some(Piece::Newline),
        KeyCode::Char('\r') if plain => Some(Piece::Return),
        KeyCode::Char(c) if plain => Some(Piece::Char(*c)),
        KeyCode::Enter if modifiers.is_empty() => Some(Piece::Return),
        KeyCode::Tab if modifiers.is_empty() => Some(Piece::Char('\t')),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
