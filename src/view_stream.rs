//! 视图流的条目（蓝图 `tui.md`「照视图流画」，2026-10-22 项目主人定换；核心 9-8、V-D 的 `miyu_client::view`）：订阅回应、
//! 翻页交来的一页条目，之后推来的 `view.add`、`view.update`、`view.append`、`view.hidden`、`view.remove` 照核心
//! `tests/view_stream.rs` 的 `apply` 改到这一份上。条目的样子、字都是核心算好的，这里只管存和改，不碰界面。
//!
//! 第一步只有存储（换视图流第 1 个 PR）：还没接进连接和界面，旧的事件流照旧画。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "换视图流第 1 步只有存储，第 2 步接进连接以后去掉")
)]

use miyu_client::view::{Body, Change, Entry, EntryId};
use serde_json::Value;

/// 一个会话手里的条目，照核心排的先后。
#[derive(Debug, Default, Clone)]
pub struct Entries {
    list: Vec<Entry>,
}

/// 推来的一条改了什么（界面照它决定重画哪里）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// 这一条改了（新加、换了、接了字、藏起显示回来的是每一条）。
    Changed(Vec<EntryId>),
    /// 拿掉了这一条。
    Removed(EntryId),
    /// 对不上手里的（要改的那一条不在）：掉了队，照 `resync` 重新订阅。
    Unknown,
}

impl Entries {
    /// 订阅回应、`view.page` 交来的一页。
    pub fn page(list: Vec<Entry>) -> Self {
        Self { list }
    }

    /// 一条条，照先后。
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.list.iter()
    }

    /// 有几条。
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// 照编号找：从后往前找，正在写的都在后头。
    pub fn get(&self, id: &EntryId) -> Option<&Entry> {
        self.at(id).map(|i| &self.list[i])
    }

    fn at(&self, id: &EntryId) -> Option<usize> {
        self.list.iter().rposition(|e| &e.id == id)
    }

    /// 排在 `after` 后面（`None` 是最前；`after` 不在手里的排最后，同核心的 `apply`）。
    fn place(&mut self, entry: Entry, after: Option<&EntryId>) {
        let index = match after {
            None => 0,
            Some(id) => self.at(id).map_or(self.list.len(), |i| i + 1),
        };
        self.list.insert(index, entry);
    }

    /// 照一条变化改。
    pub fn apply(&mut self, change: Change) -> Applied {
        match change {
            Change::Add { entry, after } => {
                let id = entry.id.clone();
                self.place(entry, after.as_ref());
                Applied::Changed(vec![id])
            }
            Change::Update { entry, after } => {
                let Some(index) = self.at(&entry.id) else {
                    return Applied::Unknown;
                };
                let id = entry.id.clone();
                match after {
                    Some(after) => {
                        self.list.remove(index);
                        self.place(entry, after.as_ref());
                    }
                    None => self.list[index] = entry,
                }
                Applied::Changed(vec![id])
            }
            Change::Append { id, text } => {
                let Some(index) = self.at(&id) else {
                    return Applied::Unknown;
                };
                match &mut self.list[index].body {
                    Body::Reply(reply) => reply.text.push_str(&text),
                    Body::Thought(thought) => thought.text.push_str(&text),
                    Body::Tool(tool) => tool.args.push_str(&text),
                    _ => return Applied::Unknown,
                }
                Applied::Changed(vec![id])
            }
            Change::Hidden { ids, hidden } => {
                let mut changed = Vec::new();
                for id in ids {
                    if let Some(index) = self.at(&id) {
                        self.list[index].hidden = hidden;
                        changed.push(id);
                    }
                }
                Applied::Changed(changed)
            }
            Change::Remove { id } => match self.at(&id) {
                Some(index) => {
                    self.list.remove(index);
                    Applied::Removed(id)
                }
                // 已经没有的（重订以后又推一遍）不算掉队。
                None => Applied::Removed(id),
            },
        }
    }
}

/// 线上的一条推送读成变化（门面的 `Change::read`，核心 V-D 修）；不是视图流的、读不懂的是 `None`。
pub fn change_of(method: &str, params: &Value) -> Option<Change> {
    let op = method.strip_prefix("view.")?;
    if !matches!(op, "add" | "update" | "append" | "hidden" | "remove") {
        return None;
    }
    Change::read(method, params.clone()).ok()
}

#[cfg(test)]
mod tests;
