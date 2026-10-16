//! `/jump`（蓝图「跳到说过的话」）：和输入历史列表同一个列表，列这个会话正文里你说的话；选中的那条顶到视口第一行，竖线
//! 亮一下。会话按页读、更早的还没读上来的，开着列表时一页页往前要（`pages.rs`）。

use std::borrow::Cow;
use std::time::{Duration, Instant, SystemTime};

use super::App;
use crate::input::{Draft, Sent};
use crate::transcript::{Entry, Kind};

impl App {
    /// 列表列的那一份，照先后：输入历史；开的是 `/jump` 时是这个会话里你说的话，另交每一句的条目编号（和前一份一一对应）。
    pub fn list_items(&self) -> (Cow<'_, [Sent]>, Vec<u64>) {
        if !self.typing.history.jump {
            return (Cow::Borrowed(self.typing.input.sent()), Vec::new());
        }
        let (sent, ids) = spoken(&self.talk.transcript.entries).into_iter().unzip();
        (Cow::Owned(sent), ids)
    }

    /// `/jump`：这个会话里还没说过话的提示一句，不开（「跳到说过的话」第 4 条）。
    pub(super) fn open_jump(&mut self) {
        if spoken(&self.talk.transcript.entries).is_empty() {
            let note = self.config.text.jump.none.clone();
            self.hint(note, false);
        } else {
            self.typing.history.open_jump();
        }
    }

    /// 跳到条目 `id`：照上一帧排好的行找它从第几行起，顶到视口第一行，竖线亮一下（第 3 条）。
    pub(super) fn jump_to(&mut self, id: u64) {
        let entries = &self.talk.transcript.entries;
        let Some(index) = entries.iter().position(|e| e.id == id) else {
            return;
        };
        let Some(row) = self.screen.view.rows.start_of(index) else {
            return;
        };
        let until = Instant::now() + Duration::from_millis(self.config.layout.jump_flash_ms);
        self.screen.view.jump_to(row, id, until);
    }
}

/// 正文里你说的话（别处来的也算，撤掉的不算），照先后：列表的一条和它的条目编号。
fn spoken(entries: &[Entry]) -> Vec<(Sent, u64)> {
    entries
        .iter()
        .filter(|e| e.kind == Kind::User && !e.hidden)
        .map(|e| {
            let at = e.at.map_or_else(SystemTime::now, SystemTime::from);
            let sent = Sent {
                draft: Draft::plain(&e.text),
                at,
            };
            (sent, e.id)
        })
        .collect()
}

#[cfg(test)]
mod tests;
