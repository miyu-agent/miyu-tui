//! 终端的回话从按键里截下来（蓝图 `tui.md`「系统通知」第 8 条）：终端往输入里回的 OSC（`ESC ] … ESC \` 或 BEL 收尾），
//! crossterm 不认，拆成一串按键（Alt+`]`、几个字、Alt+`\`），原来会打进输入框（2026-10-27 伪终端实测：kitty 点通知回的
//! `]99;i=…;\` 进了输入框）。这里照这一串认：Alt+`]` 打头就先攒着，攒到收尾的整串交回去；中途来了别的、
//! [`WAIT`] 里没有下一个的，是人按的，攒着的照原样放回。开了 kitty 键盘协议的终端，人按 Alt+`]` 走 CSI u，不会这样打头。

use std::time::Duration;

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// 攒着时多久等不到下一个，就当是人按的（终端的回话是一口气写来的）。
pub const WAIT: Duration = Duration::from_millis(20);

/// 一串回话最多攒多长：再长的不是回话，照原样放回。
const LONGEST: usize = 4096;

/// 读到的一样。
#[derive(Debug, PartialEq, Eq)]
pub enum Read {
    /// 按键、鼠标、粘贴、焦点这些，照旧交给主循环。
    Event(Event),
    /// 终端的一句 OSC 回话：`ESC ]` 和收尾中间的字（例如 `99;i=miyu-s-…;`）。
    Reply(String),
}

/// 正在攒的那一串。
#[derive(Default)]
pub struct Replies {
    /// 攒着的原样的事件（不是回话时放回去）。
    held: Vec<Event>,
    /// 攒着的字；没在攒是 `None`。
    text: Option<String>,
}

impl Replies {
    /// 在攒：读的那一头要照 [`WAIT`] 等，等不到的 [`Replies::expire`]。
    pub fn holding(&self) -> bool {
        self.text.is_some()
    }

    /// 读到一个事件：交回该交给主循环的（攒着的时候是空的）。
    pub fn feed(&mut self, event: Event) -> Vec<Read> {
        let key = match &event {
            Event::Key(key) if key.kind == KeyEventKind::Press => Some(*key),
            _ => None,
        };
        let Some(text) = self.text.as_mut() else {
            if key.is_some_and(|k| is(k, ']', KeyModifiers::ALT)) {
                self.text = Some(String::new());
                self.held.push(event);
                return Vec::new();
            }
            return vec![Read::Event(event)];
        };
        if key.is_some_and(|k| is(k, '\\', KeyModifiers::ALT) || is(k, 'g', KeyModifiers::CONTROL))
        {
            let text = std::mem::take(text);
            self.text = None;
            self.held.clear();
            return vec![Read::Reply(text)];
        }
        let plain = key.and_then(|k| match k.code {
            KeyCode::Char(c) if (k.modifiers - KeyModifiers::SHIFT).is_empty() => Some(c),
            _ => None,
        });
        match plain {
            Some(c) if text.len() < LONGEST => {
                text.push(c);
                self.held.push(event);
                Vec::new()
            }
            _ => {
                let mut out = self.expire();
                out.push(Read::Event(event));
                out
            }
        }
    }

    /// 等不到下一个了：是人按的，攒着的照原样交回。
    pub fn expire(&mut self) -> Vec<Read> {
        self.text = None;
        self.held.drain(..).map(Read::Event).collect()
    }
}

/// 是这个字、这几个修饰键。
fn is(key: KeyEvent, c: char, modifiers: KeyModifiers) -> bool {
    key.code == KeyCode::Char(c) && key.modifiers == modifiers
}

#[cfg(test)]
mod tests;
