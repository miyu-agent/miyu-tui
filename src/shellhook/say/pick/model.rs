//! `/model`（同终端界面「配置与模型」第 1 条，列表、筛、画法用它的 `model_list`）：换终端会话的模型，也记成新会话的默认
//! （同终端界面，2026-10-02 项目主人定）。`/model 名字` 不弹列表，写全的名字、引用（不分大小写）直接换；没写全的不猜，列几个对得上的。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;
use serde_json::json;

use super::{Picked, Picker, result};
use crate::config::Config;
use crate::core::{Choice, Command, Core, Update};
use crate::model_list::ModelList;
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::{Words, fill};
use crate::ui::Chrome;

/// 带的名字没写全时列几个对得上的。
const NEAR: usize = 5;

/// 换模型那一问的编号（说话那一套用的是个位数）。
const CONFIGURE_TAG: u64 = 90;

/// `/model` 的列表。
pub struct ModelPick {
    list: ModelList,
    /// 带了参数的：照它筛，列表到了直接定。
    given: Option<String>,
    /// 定了以后留的那一行、没对上的那一行。
    words: Words,
    /// 已经停到正在用的那个上了（终端会话的最新一页读回来才知道是哪个）。
    placed: bool,
}

impl ModelPick {
    /// `given` 是 `/model` 后面的字。
    pub fn new(given: Option<&str>, words: &Words) -> Self {
        let mut list = ModelList::default();
        if let Some(given) = given {
            list.typed(given.to_string());
        }
        Self {
            list,
            given: given.map(str::to_string),
            words: words.clone(),
            placed: false,
        }
    }

    /// 换成 `choice`：终端会话换、新会话的默认也换（同终端界面）。没有 key 的不换。
    fn choose(&self, choice: &Choice, turn: &Turn, core: &Core) -> Picked {
        let Some(session) = turn.transcript.session.clone() else {
            return Picked::Stay;
        };
        if !choice.state.pickable() {
            return Picked::Stay;
        }
        if turn.transcript.current_ref().as_deref() != Some(choice.reference.as_str()) {
            core.send(Command::Ask {
                tag: CONFIGURE_TAG,
                method: "session.configure",
                params: json!({ "session": session, "model": choice.reference }),
            });
            core.send(Command::SetChat(choice.reference.clone()));
        }
        Picked::Done(Some(result(&fill(
            &self.words.slash_model,
            &[("name", &choice.name)],
        ))))
    }
}

impl Picker for ModelPick {
    fn start(&mut self, core: &Core) {
        core.send(Command::ListChoices);
    }

    fn core(&mut self, update: &Update, turn: &Turn) {
        if let Update::Choices(all) = update {
            let query = self.list.query.clone();
            self.list
                .replace(all.clone(), turn.transcript.current_ref().as_deref());
            if !query.is_empty() {
                self.list.typed(query);
            }
        }
    }

    fn ready(&mut self, turn: &mut Turn, core: &Core, _config: &Config) -> Picked {
        // 终端会话的最新一页读回来、知道正在用哪个了：停到它上面（只停一次，之后照人挪的）。
        let current = turn.transcript.current_ref();
        if !self.placed && self.list.loaded && self.list.query.is_empty() && current.is_some() {
            let all = std::mem::take(&mut self.list.all);
            self.list.replace(all, current.as_deref());
            self.placed = true;
        }
        let Some(given) = &self.given else {
            return Picked::Stay;
        };
        if !self.list.loaded || turn.transcript.session.is_none() {
            return Picked::Stay;
        }
        // 只认写全的名字、引用（不分大小写）：`v4.1 flash` 这样能对上好几个的不猜（2026-10-21 项目主人：「怎么能知道自己要换
        // 的是什么呢？应该输入完整名字才允许这样切」）。
        let wanted = given.trim().to_lowercase();
        let exact = self
            .list
            .all
            .iter()
            .find(|c| c.name.to_lowercase() == wanted || c.reference.to_lowercase() == wanted)
            .cloned();
        if let Some(choice) = exact {
            return self.choose(&choice, turn, core);
        }
        let near: Vec<String> = self
            .list
            .matches()
            .iter()
            .take(NEAR)
            .map(|c| c.name.clone())
            .collect();
        match near.is_empty() {
            false => Picked::Done(Some(result(&fill(
                &self.words.slash_ambiguous,
                &[("query", given), ("names", &near.join("、"))],
            )))),
            true => Picked::Done(Some(result(&fill(
                &self.words.slash_no_match,
                &[("query", given)],
            )))),
        }
    }

    fn typing(&self) -> bool {
        self.list.typing
    }

    fn key(&mut self, key: KeyEvent, turn: &mut Turn, core: &Core, config: &Config) -> Picked {
        // 搜索那一截照终端界面（按键模式按 `/` 才搜，`list_search`）。
        let query = self.list.query.clone();
        match crate::list_search::key(config.layout.vim, &mut self.list.typing, &query, key) {
            crate::list_search::Step::Query(query) => {
                self.list.typed(query);
                return Picked::Stay;
            }
            crate::list_search::Step::Kept => return Picked::Stay,
            crate::list_search::Step::Pass => {}
        }
        match key.code {
            KeyCode::Esc => return Picked::Done(None),
            KeyCode::Up => self.list.step(false),
            KeyCode::Down => self.list.step(true),
            KeyCode::Enter => {
                if let Some(choice) = self.list.picked().cloned() {
                    return self.choose(&choice, turn, core);
                }
            }
            _ => {}
        }
        Picked::Stay
    }

    fn view(
        &self,
        turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        let current = turn.transcript.current_ref();
        let (chrome, lines, _) =
            crate::ui::model_list::lines(&self.list, current.as_deref(), config, width, max);
        (chrome, lines)
    }
}
