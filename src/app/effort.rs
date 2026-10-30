//! `/effort`：换这个会话的思考强度（蓝图 `tui.md`「配置与模型」思考强度，核心 SM-1；2026-10-30 项目主人定：`/effort` 只管
//! 当前会话，设置页管新会话的默认）。开框时向核心要一次 `model.list`，照正在请求的模型（没有的照会话的引用、默认的聊天
//! 模型）列它的几级，第一行「跟随设置」；`Enter` 交给核心（`session.configure {effort}`），关框，不弹提示，成了底栏当场
//! 变。还没开会话的记着，开会话时带上。用的是模型池的提示一句，不开框（2026-10-02、10-30 项目主人定：池不能设）；开框
//! 以前就看（2026-10-30 项目主人报：先闪一下大框才关）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use super::{App, Panel};
use crate::core::{Command, EffortList, Efforts};

impl App {
    /// 开框，向核心要几级。
    pub(super) fn open_effort(&mut self) {
        if !self.reachable() {
            return;
        }
        // 会话的引用、没开会话时默认的聊天模型是池的：当场提示，不开框。还不知道的（刚连上）照旧开框等核心交回来再看。
        let reference = self
            .talk
            .transcript
            .model_ref()
            .or(self.link.default_chat.as_deref());
        if reference.is_some_and(|r| r.starts_with('@')) {
            let note = self.config.text.effort.need_model.clone();
            self.hint(note, false);
            return;
        }
        self.boxes.efforts = None;
        self.boxes.panel = Some(Panel::Effort { selected: 0 });
        self.refresh_effort();
    }

    /// 向核心要一次 `model.list`，底栏照它写模型和这一档：连上核心（读到默认的聊天模型以后）、`/new` 以后、换了强度以后
    /// （2026-10-02 项目主人报：空会话的底栏没有模型、强度，换了强度要等下一轮才变）。只要正在用的、会话的、默认的那几个
    /// （核心 V-6 上的 `models`，2026-10-15 核心转来：整份 120 KB，只为底栏一个模型名）；有池的、还不知道的要整份。
    pub(super) fn refresh_effort(&mut self) {
        let in_use = self
            .talk
            .transcript
            .model
            .as_ref()
            .map(|(m, e)| format!("{e}/{m}"));
        let refs = [
            self.talk.transcript.model_ref(),
            in_use.as_deref(),
            self.link.default_chat.as_deref(),
        ];
        let wanted = crate::core::wanted_models(&refs);
        self.link.core.send(Command::ListEfforts(wanted));
    }

    /// 读到默认的聊天模型（`models.chat`，连上时和别的配置合成一条读）：记下，照它要底栏的模型资料；读不到的（核心不认、
    /// 没配）要整份。
    pub(super) fn chat_configured(&mut self, got: Option<&serde_json::Value>) {
        if let Some(chat) = got.and_then(|g| g["items"]["models.chat"]["value"].as_str()) {
            self.link.default_chat = Some(chat.to_string());
        }
        self.refresh_effort();
    }

    /// 核心交回了：底栏照它写；框开着的，找出改哪个模型、选着配置的默认那一级，用的是模型池的关框、提示一句。
    pub(super) fn efforts_listed(&mut self, list: EffortList) {
        // 收哪几种附件照同一份 `model.list`（`attachments.rs`）。
        self.link.abilities = list.abilities.clone();
        self.link.default_chat = list.chat.clone();
        self.footer_effort(&list);
        if !matches!(self.boxes.panel, Some(Panel::Effort { .. })) {
            return;
        }
        let Some(target) = self.effort_target(&list).cloned() else {
            self.boxes.panel = None;
            let note = self.config.text.effort.need_model.clone();
            self.hint(note, false);
            return;
        };
        // 框里标的「现在用的」是这个会话自己定的那一档，跟着配置的标第一行（不是配置的默认，核心 SM-1）。
        let mut target = target;
        target.current = self.talk.transcript.own_effort().map(str::to_string);
        let selected = crate::ui::effort_list::initial(&target);
        self.boxes.panel = Some(Panel::Effort { selected });
        self.boxes.efforts = Some(target);
    }

    /// 底栏：还没开会话的照默认的聊天模型写模型和这一档；开了的、改的就是正在用的那个，照新的最终值写这一档。会话自己定了
    /// 一档的照它，不拿配置的盖掉（核心 SM-1）。
    fn footer_effort(&mut self, list: &EffortList) {
        let Some(target) = self.effort_target(list).cloned() else {
            return;
        };
        if self.talk.transcript.model_ref().is_none() {
            self.talk
                .transcript
                .show_default(&target.reference, target.current.clone());
            return;
        }
        let in_use = self
            .talk
            .transcript
            .model
            .as_ref()
            .map(|(m, e)| format!("{e}/{m}"));
        if in_use.as_deref() == Some(target.reference.as_str())
            && self.talk.transcript.own_effort().is_none()
        {
            self.talk.transcript.set_effort(target.current);
        }
    }

    /// 改哪个模型：正在请求的那个，没有的照会话的引用，还没开会话的照默认的聊天模型。
    fn effort_target<'a>(&self, list: &'a EffortList) -> Option<&'a Efforts> {
        let in_use = self
            .talk
            .transcript
            .model
            .as_ref()
            .map(|(m, e)| format!("{e}/{m}"));
        list.target(in_use.as_deref(), self.talk.transcript.model_ref())
    }

    /// 框开着时的按键。
    pub(super) fn effort_key(&mut self, selected: usize, key: KeyEvent) {
        let last = self.boxes.efforts.as_ref().map_or(0, |t| t.levels.len());
        let at = |selected| Some(Panel::Effort { selected });
        match key.code {
            KeyCode::Up => self.boxes.panel = at(selected.saturating_sub(1)),
            KeyCode::Down => self.boxes.panel = at((selected + 1).min(last)),
            KeyCode::Enter => self.choose_effort(selected),
            KeyCode::Esc => self.boxes.panel = None,
            _ => {}
        }
    }

    /// 鼠标在框上：悬停选中，点一下换；交回归不归它。
    pub(super) fn effort_mouse(&mut self, mouse: MouseEvent, at: Position) -> bool {
        let Some(Panel::Effort { .. }) = self.boxes.panel else {
            return false;
        };
        let areas = self.screen.areas;
        if !areas.menu.contains(at) {
            return false;
        }
        let row = at.y.checked_sub(areas.menu_text.y).map(usize::from);
        let index = row.and_then(|row| self.screen.panel_rows.get(row).copied().flatten());
        if let Some(index) = index.filter(|_| areas.menu_text.contains(at)) {
            self.boxes.panel = Some(Panel::Effort { selected: index });
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.choose_effort(index);
            }
        }
        true
    }

    /// 换成第 `index` 行（第 0 行跟随设置）：交给核心，关框，不弹提示（2026-10-01 项目主人：改模型、思考强度不要通知）。
    /// 和现在一样的只关框。成了底栏照 [`App::effort_set`] 变。
    fn choose_effort(&mut self, index: usize) {
        self.boxes.panel = None;
        let Some(target) = self.boxes.efforts.take() else {
            return;
        };
        let level = index
            .checked_sub(1)
            .and_then(|i| target.levels.get(i).cloned());
        if index > 0 && level.is_none() || level == target.current {
            return;
        }
        self.link.core.send(Command::SessionEffort(level));
    }

    /// 这个会话的思考强度换成了（核心答应了，或者还没开会话、当场算换成了）：定了一档的底栏当场写它；回到跟随设置的再要
    /// 一次 `model.list`，底栏照配置的那一档写（核心只改思考强度时不当场推 `model.changed`，SM-1）。
    pub(super) fn effort_set(&mut self, level: Option<String>) {
        let back = level.is_none();
        self.talk.transcript.own_effort_set(level);
        if back {
            self.refresh_effort();
        }
    }
}
