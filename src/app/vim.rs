//! 列表开着时 `Ctrl+J`、`Ctrl+K` 当 `↓`、`↑`（蓝图 `tui.md`「按键」，2026-10-01 项目主人要 vim 的上下）；开着时不能打字的框
//! 直接 `j`、`k` 也行（2026-10-11 项目主人）；按键模式是 Vim 时会话列表、`/model` 没在打搜索的字也算（2026-10-21）。
//! 只换按键本身，后面照常分给开着的那个列表；抽屉里写字时 `Ctrl+J` 照旧换行，不换。

use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::{App, Panel};

impl App {
    /// 有列表、面板开着，或者抽屉开着、没在写字时，换掉这个按键；`menu_open`：命令列表、`@` 文件列表开着。
    pub(super) fn vim_keys(&self, event: Event, menu_open: bool) -> Event {
        let listing = menu_open
            || self.typing.history.open
            || self.boxes.panel.is_some()
            || (self.talk.drawers.open()
                && !self.talk.drawers.editing()
                && !self.talk.drawers.command_scrolls());
        let no_typing = self.no_typing();
        match event {
            Event::Key(key) => Event::Key(plain(step(key, listing), no_typing)),
            other => other,
        }
    }
}

impl App {
    /// 开着时不能打字的框；抽屉开着时按键先归抽屉（它自己认 vim 键）。
    fn no_typing(&self) -> bool {
        !self.talk.drawers.open()
            && (self.vim_list_idle()
                || matches!(
                    self.boxes.panel,
                    Some(
                        Panel::Pick { .. }
                            | Panel::Changed { .. }
                            | Panel::Prefix { .. }
                            | Panel::Language { .. }
                            | Panel::Export { .. }
                            | Panel::Theme { .. }
                            | Panel::Effort { .. }
                            | Panel::Background { .. }
                            | Panel::Help { .. }
                            | Panel::Usage { .. }
                            | Panel::Status { .. }
                            | Panel::Goal { .. }
                            | Panel::Tools { .. }
                            | Panel::Context { .. }
                    )
                ))
    }

    /// 按键是 `j`、`k` 这些、不是在打字：不能打字的框开着，或者按键模式是 Vim、`@` 文件列表里按了 `Esc` 在选。输入法照它关成
    /// 英文（发版前 check 16，2026-11-09 验收时项目主人：「这个没改啊？」——中文输入法开着，`j`、`k` 进了输入法，到不了列表）。
    pub(super) fn keys_not_typing(&self) -> bool {
        self.no_typing()
            || (self.config.layout.vim && self.typing.mention.mode == crate::mention::Mode::Picking)
    }

    /// 按键模式是 Vim，会话列表、`/model`、输入历史列表（发版前 check 第 31 条）开着、没在打搜索的字（`list_search.rs`）。
    fn vim_list_idle(&self) -> bool {
        if !self.config.layout.vim {
            return false;
        }
        if self.typing.history.open {
            return !self.typing.history.typing;
        }
        match self.boxes.panel {
            Some(Panel::Sessions) => self.boxes.session_list.as_ref().is_some_and(|l| !l.typing),
            Some(Panel::Models) => self.boxes.model_list.as_ref().is_some_and(|l| !l.typing),
            _ => false,
        }
    }
}

/// 开着时不能打字的框（`no_typing`：人格框、预设框、`/language`、`/effort`、后台面板、`/help`、`/usage`）：直接按
/// `j`、`k` 也换成 `↓`、`↑`（2026-10-11 项目主人）。会话列表、换模型、命令列表、输入历史打字是搜，不换。
pub fn plain(key: KeyEvent, no_typing: bool) -> KeyEvent {
    if !no_typing || key.modifiers != KeyModifiers::NONE {
        return key;
    }
    let code = match key.code {
        KeyCode::Char('j') => KeyCode::Down,
        KeyCode::Char('k') => KeyCode::Up,
        _ => return key,
    };
    KeyEvent { code, ..key }
}

/// `list_open`：有列表、面板开着，或者抽屉开着、没在写字。是的话把 `Ctrl+J`、`Ctrl+K` 换成 `↓`、`↑`。
pub fn step(key: KeyEvent, list_open: bool) -> KeyEvent {
    if !list_open || key.modifiers != KeyModifiers::CONTROL {
        return key;
    }
    let code = match key.code {
        KeyCode::Char('j' | 'J') => KeyCode::Down,
        KeyCode::Char('k' | 'K') => KeyCode::Up,
        _ => return key,
    };
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        ..key
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::step;

    #[test]
    fn ctrl_j_and_k_move_only_while_a_list_is_open() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        assert_eq!(step(ctrl('j'), true).code, KeyCode::Down);
        assert_eq!(step(ctrl('k'), true).code, KeyCode::Up);
        assert_eq!(step(ctrl('k'), true).modifiers, KeyModifiers::NONE);
        assert_eq!(step(ctrl('j'), false), ctrl('j'), "没开列表：照旧换行");
        let plain = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(step(plain, true), plain, "不按 Ctrl 的照常打字");
        let shifted = KeyEvent::new(
            KeyCode::Char('J'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(step(shifted, true), shifted);
    }

    #[test]
    fn plain_j_and_k_move_in_boxes_that_take_no_typing() {
        // 2026-10-11 项目主人：人格框这类开着不能打字的框，直接 j、k 上下。
        let plain = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        assert_eq!(super::plain(plain('j'), true).code, KeyCode::Down);
        assert_eq!(super::plain(plain('k'), true).code, KeyCode::Up);
        assert_eq!(
            super::plain(plain('j'), false),
            plain('j'),
            "能打字的列表照常打字"
        );
        assert_eq!(super::plain(plain('x'), true), plain('x'));
    }

    /// 发版前 check 16（2026-11-09 验收时项目主人：「这个没改啊？」）：Vim 模式下 `@` 列表里按了 `Esc` 在选，`j`、`k` 不是打字，
    /// 输入法关成英文；不是 Vim 的照旧算在打字。
    #[test]
    fn picking_in_the_mention_list_turns_the_input_method_off() {
        let (mut app, _sent) = crate::app::test_support::app();
        app.config.layout.vim = true;
        app.tick();
        assert_eq!(app.typing.ime.wants_typing(), Some(true), "平常在打字");
        app.typing.mention.mode = crate::mention::Mode::Picking;
        app.tick();
        assert_eq!(app.typing.ime.wants_typing(), Some(false), "在列表里选");
        app.config.layout.vim = false;
        app.tick();
        assert_eq!(app.typing.ime.wants_typing(), Some(true), "不是 Vim 的照旧");
    }
}
