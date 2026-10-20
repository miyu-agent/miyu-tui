//! `/edit`（设计 22 第七节「斜杠命令」：上一句放回 fish 的命令行）：问到终端会话以后向核心要它的最新一页（`view.page`），
//! 取最后一句人说的话，写进 `say` 交给脚本的那份文件（`--leftover`，答完放回命令行的同一条路）。只放回来：改完回车照一句
//! 新的话发，不替换上一轮（shell 里看不出你是不是放弃了编辑，替换会误伤）。
//!
//! 不照借来的 `Turn` 的正文找：终端界面读回的那一页是先停放、再由界面换上来的，借来的 `Turn` 不换，正文是空的；这条
//! 连接以外说的话（shell 里每说一句是一条新连接）在终端界面的 `last_said` 看来也是别处来的（2026-10-21 实测三回）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;
use serde_json::{Value, json};

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Command, Core, Update};
use crate::shellhook::say::turn::Turn;
use crate::ui::Chrome;

/// 要最新一页那一问的编号。
const PAGE_TAG: u64 = 95;

/// `/edit`：要终端会话的最新一页，交出上一句。
pub struct EditPick {
    /// 上一句写到哪（没给的印出来）。
    leftover: Option<std::path::PathBuf>,
    /// 要过了。
    asked: bool,
    /// 页回来了：上一句（没有的是 `None`）。
    found: Option<Option<String>>,
}

impl EditPick {
    pub fn new(leftover: Option<&std::path::Path>) -> Self {
        Self {
            leftover: leftover.map(std::path::Path::to_path_buf),
            asked: false,
            found: None,
        }
    }
}

/// 一页事件里最后一句人说的话：命令模板展开的照人打的那一行，别的照字块拼起来。
fn last_said(page: &Value) -> Option<String> {
    let events = page["events"].as_array()?;
    events
        .iter()
        .rev()
        .filter(|e| e["kind"] == "message.user" && e["by"]["kind"] == "person")
        .find_map(|e| {
            let body = &e["body"];
            let typed = body["command"]["text"].as_str().filter(|t| !t.is_empty());
            let text = typed.map(str::to_string).unwrap_or_else(|| {
                body["blocks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|b| b["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            (!text.is_empty()).then_some(text)
        })
}

impl Picker for EditPick {
    fn start(&mut self, _core: &Core) {}

    fn core(&mut self, update: &Update, _turn: &Turn) {
        if let Update::Answer { tag, result } = update
            && *tag == PAGE_TAG
        {
            self.found = Some(result.as_ref().ok().and_then(last_said));
        }
    }

    fn ready(&mut self, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        if !self.asked
            && let Some(session) = turn.transcript.session.clone()
        {
            self.asked = true;
            core.send(Command::Ask {
                tag: PAGE_TAG,
                method: "view.page",
                params: json!({ "session": session }),
            });
        }
        let Some(found) = self.found.take() else {
            return Picked::Stay;
        };
        let Some(text) = found else {
            let text = config
                .text
                .refusal_hints
                .get("not_redoable")
                .cloned()
                .unwrap_or_default();
            return Picked::Done(Some(result(&text)));
        };
        match &self.leftover {
            Some(path) => match std::fs::write(path, text) {
                Ok(()) => Picked::Done(None),
                Err(e) => Picked::Done(Some(result(&e.to_string()))),
            },
            None => Picked::Done(Some(result(&text))),
        }
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        match key.code {
            KeyCode::Esc => Picked::Done(None),
            _ => Picked::Stay,
        }
    }

    fn view(
        &self,
        _turn: &Turn,
        _config: &Config,
        _width: u16,
        _max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        (Chrome::new("", Vec::new()), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::last_said;

    #[test]
    fn the_last_thing_a_person_said_comes_back() {
        let page = json!({"events": [
            {"kind": "message.user", "by": {"kind": "person"}, "body": {"blocks": [{"type": "text", "text": "第一句"}]}},
            {"kind": "message.assistant", "by": {"kind": "model"}, "body": {}},
            {"kind": "message.user", "by": {"kind": "person"}, "body": {"blocks": [{"type": "text", "text": "第二句"}]}},
            {"kind": "message.user", "by": {"kind": "harness"}, "body": {"blocks": [{"type": "text", "text": "别处来的"}]}},
        ]});
        assert_eq!(last_said(&page).as_deref(), Some("第二句"));
        assert_eq!(last_said(&json!({"events": []})), None);
    }
}
