//! `@` 文件列表的按键和鼠标（蓝图 `tui.md`「`@` 文件列表」第 5 条）：`↑`、`↓` 选，`Tab` 进目录（文件同 `Enter`），
//! `Enter` 收成块，`Esc` 关；悬停选中，点一下收成块。按键模式是 Vim 时 `Esc` 先进列表里选（发版前 check 16）。

use std::time::Instant;

use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::App;
use crate::core::Command;
use crate::input::Action;
use crate::mention::{Found, Mode, escape};

impl App {
    /// 这一帧开着的 `@` 列表：历史列表、后台面板、抽屉开着时不开。
    pub fn mention_found(&mut self) -> Option<Found> {
        // 翻输入历史翻出来的那句还没改过：不弹，`↑` `↓` 接着翻（蓝图「按键」`↑`、`↓`）。
        if self.typing.history.open
            || self.boxes.panel.is_some()
            || self.talk.drawers.open()
            || self.typing.input.recalled()
        {
            return None;
        }
        let editor = &self.typing.input.editor;
        let (text, cursor) = (editor.text().to_string(), editor.cursor());
        let found = self.typing.mention.find(&text, cursor)?;
        // 词变了、清单在建到点了：问核心。`cwd` 是现在用的工作区：开着的会话照会话的，还没开的照 `/workspace` 换过的，
        // 都没有的照界面所在的目录（2026-10-26 项目主人报：切了工作区 `@` 还列旧的；原来一直照界面所在的目录）。
        if let Some((word, ask)) = self.typing.mention.next_ask(Instant::now()) {
            let shown = self.seen().current_workspace().to_string();
            let cwd = crate::local::expand_home(shown.into(), std::env::home_dir().as_deref());
            let (method, params) = ask.request(&cwd.to_string_lossy());
            self.link.core.send(Command::Files {
                word,
                method,
                params,
            });
        }
        Some(found)
    }

    /// 列表开着时的按键：管得了的交回 `Some`，别的（打字、挪光标）交给输入框。
    pub(super) fn mention_key(&mut self, key: KeyEvent, found: &Found) -> Option<Action> {
        if self.config.layout.vim
            && let Some(action) = self.mention_vim(key, found)
        {
            return action;
        }
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.typing
                    .mention
                    .step(key.code == KeyCode::Down, found.items.len());
            }
            KeyCode::Tab => self.mention_tab(found),
            KeyCode::Enter => self.mention_take(found),
            KeyCode::Esc => self.typing.mention.dismiss(found),
            _ => return None,
        }
        Some(Action::None)
    }

    /// 按键模式是 Vim（发版前 check 16，2026-11-09 项目主人：「列表里能用 j/k 选」，经 core1 转）：打字时 `Esc` 进列表里选；
    /// 选的时候 `j` `k` 上下、`h` `l` 同 `←` `→`、`/` 筛，别的字不进输入框，`Esc` 关；筛的时候打的字接在词后面，`Enter`
    /// 筛好了回去选，`Esc` 不筛了、词换回按 `/` 以前的。交回 `None` 的照不是 Vim 的办（`↑` `↓`、`Tab`、`Enter` 都一样）。
    fn mention_vim(&mut self, key: KeyEvent, found: &Found) -> Option<Option<Action>> {
        let plain = key.modifiers.difference(KeyModifiers::SHIFT).is_empty();
        let arrow = |code| KeyEvent::new(code, KeyModifiers::NONE);
        match (self.typing.mention.mode.clone(), key.code) {
            (Mode::Typing, KeyCode::Esc) => self.typing.mention.mode = Mode::Picking,
            (Mode::Picking, KeyCode::Char(c)) if plain => match c {
                'j' | 'k' => self.typing.mention.step(c == 'j', found.items.len()),
                'h' => return Some(Some(self.typing.input.key(arrow(KeyCode::Left)))),
                'l' => return Some(Some(self.typing.input.key(arrow(KeyCode::Right)))),
                '/' => {
                    let editor = &self.typing.input.editor;
                    let before = editor.text()[found.start..editor.cursor()].to_string();
                    self.typing.mention.mode = Mode::Filtering(before);
                }
                _ => {}
            },
            (Mode::Picking, KeyCode::Backspace | KeyCode::Delete) => {}
            (Mode::Filtering(_), KeyCode::Enter) => self.typing.mention.mode = Mode::Picking,
            (Mode::Filtering(before), KeyCode::Esc) => {
                self.typing.input.retype_mention(found.start, &before);
                self.typing.mention.mode = Mode::Picking;
            }
            _ => return None,
        }
        Some(Some(Action::None))
    }

    /// 列表里的鼠标（「斜杠命令列表」第 3 条）：悬停高亮指针下那一条、列表不动，滚轮滚，点一下收成块。
    pub(super) fn mention_mouse(&mut self, mouse: MouseEvent, found: &Found) -> Action {
        let rows = crate::ui::menu_rows(
            self.config.layout.menu_rows,
            self.screen.areas.menu_text.height,
        );
        let count = found.items.len();
        let Some(by) = super::mouse::list_step(mouse.kind, false) else {
            return Action::None;
        };
        let mention = &mut self.typing.mention;
        crate::menu::pin(mention.selected, &mut mention.pinned, count, rows, by);
        let top = crate::menu::top(mention.selected, mention.pinned, count, rows);
        let text = self.screen.areas.menu_text;
        let Some(index) = crate::ui::menu_index_at(text, count, top, rows, mouse.row) else {
            return Action::None;
        };
        self.typing.mention.selected = index;
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            self.mention_take(found);
        }
        Action::None
    }

    /// `Tab`：选中的是目录，词换成这个目录接着列；是文件，收成块。
    fn mention_tab(&mut self, found: &Found) {
        let Some(item) = found.items.get(self.typing.mention.selected) else {
            return;
        };
        if item.dir {
            let text = format!("@{}", escape(&item.shown));
            self.typing.input.retype_mention(found.start, &text);
        } else {
            self.mention_take(found);
        }
    }

    /// `Enter`、点一下：选中的那一条收成块。
    fn mention_take(&mut self, found: &Found) {
        if let Some(item) = found.items.get(self.typing.mention.selected) {
            self.typing
                .input
                .take_mention(found.start, item.path.clone());
        }
    }
}
