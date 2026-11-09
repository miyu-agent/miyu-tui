//! `$` 点名技能的列表的按键和鼠标（蓝图 `tui.md`「`$` 点名技能」，发版前 check 26）：照 `@` 文件列表（`mention.rs`）：
//! `↑`、`↓` 选，`Tab`、`Enter` 把词换成「`$名字 `」，`Esc` 关；按键模式是 Vim 时 `Esc` 先进列表里选（`j` `k`、`h` `l`、`/` 筛，
//! 同发版前 check 16）；悬停选中，点一下选定。

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::App;
use crate::input::Action;
use crate::mention::Mode;
use crate::skill_word::Found;

impl App {
    /// 这一帧开着的 `$` 列表：历史列表、后台面板、抽屉开着时不开；核心没给能点名的技能的不开。
    pub fn skill_found(&mut self) -> Option<Found> {
        if self.typing.history.open
            || self.boxes.panel.is_some()
            || self.talk.drawers.open()
            || self.typing.input.recalled()
        {
            return None;
        }
        let editor = &self.typing.input.editor;
        let (text, cursor) = (editor.text().to_string(), editor.cursor());
        let skills = &self.config.commands.skills;
        self.typing.skill_word.find(&text, cursor, skills)
    }

    /// 列表开着时的按键：管得了的交回 `Some`，别的（打字、挪光标）交给输入框。
    pub(super) fn skill_key(&mut self, key: KeyEvent, found: &Found) -> Option<Action> {
        if self.config.layout.vim
            && let Some(action) = self.skill_vim(key, found)
        {
            return action;
        }
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                let count = found.items.len();
                self.typing
                    .skill_word
                    .step(key.code == KeyCode::Down, count);
            }
            KeyCode::Tab | KeyCode::Enter => self.skill_take(found),
            KeyCode::Esc => self.typing.skill_word.dismiss(found),
            _ => return None,
        }
        Some(Action::None)
    }

    /// 按键模式是 Vim：同 `@` 列表（`mention.rs` 的 `mention_vim`）。交回 `None` 的照不是 Vim 的办。
    fn skill_vim(&mut self, key: KeyEvent, found: &Found) -> Option<Option<Action>> {
        let plain = key.modifiers.difference(KeyModifiers::SHIFT).is_empty();
        let arrow = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let picker = &mut self.typing.skill_word;
        match (picker.mode.clone(), key.code) {
            (Mode::Typing, KeyCode::Esc) => picker.mode = Mode::Picking,
            (Mode::Picking, KeyCode::Char(c)) if plain => match c {
                'j' | 'k' => picker.step(c == 'j', found.items.len()),
                'h' => return Some(Some(self.typing.input.key(arrow(KeyCode::Left)))),
                'l' => return Some(Some(self.typing.input.key(arrow(KeyCode::Right)))),
                '/' => {
                    let editor = &self.typing.input.editor;
                    let before = editor.text()[found.start..editor.cursor()].to_string();
                    self.typing.skill_word.mode = Mode::Filtering(before);
                }
                _ => {}
            },
            (Mode::Picking, KeyCode::Backspace | KeyCode::Delete) => {}
            (Mode::Filtering(_), KeyCode::Enter) => picker.mode = Mode::Picking,
            (Mode::Filtering(before), KeyCode::Esc) => {
                self.typing.input.retype_mention(found.start, &before);
                self.typing.skill_word.mode = Mode::Picking;
            }
            _ => return None,
        }
        Some(Some(Action::None))
    }

    /// 列表里的鼠标：悬停高亮指针下那一条、列表不动，滚轮滚，点一下选定。
    pub(super) fn skill_mouse(&mut self, mouse: MouseEvent, found: &Found) -> Action {
        let rows = crate::ui::menu_rows(
            self.config.layout.menu_rows,
            self.screen.areas.menu_text.height,
        );
        let count = found.items.len();
        let Some(by) = super::mouse::list_step(mouse.kind, false) else {
            return Action::None;
        };
        let picker = &mut self.typing.skill_word;
        crate::menu::pin(picker.selected, &mut picker.pinned, count, rows, by);
        let top = crate::menu::top(picker.selected, picker.pinned, count, rows);
        let text = self.screen.areas.menu_text;
        let Some(index) = crate::ui::menu_index_at(text, count, top, rows, mouse.row) else {
            return Action::None;
        };
        self.typing.skill_word.selected = index;
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            self.skill_take(found);
        }
        Action::None
    }

    /// 选定：词换成「`$名字 `」，接着打字。
    fn skill_take(&mut self, found: &Found) {
        if let Some((skill, _)) = found.items.get(self.typing.skill_word.selected) {
            let text = format!("${} ", skill.name);
            self.typing.input.retype_mention(found.start, &text);
            self.typing.skill_word.mode = Mode::Typing;
        }
    }
}
