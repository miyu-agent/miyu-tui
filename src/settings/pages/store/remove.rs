//! 卸载的弹窗给按钮（PK-5，蓝图「配置页」第 43c 条之六）：「卸载」（配置留着）、「卸载并删除配置」、「取消」；核心没说这个包
//! 有配置目录、设置项的（预览里没有 `config`）只给「卸载」「取消」。`←`、`→`、`Tab` 挪，`Enter` 按，`Esc` 取消，鼠标点按钮也行；
//! 预览块照旧能滚，没看到最后一行之前两个卸的按钮不做，「取消」随时能按。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use super::flow::Flow;
use crate::settings::{Settings, Texts};

/// 卸载弹窗上的一个按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// 卸载，配置目录留着。
    Keep,
    /// 卸载，配置目录一起删。
    Purge,
    /// 不卸。
    Cancel,
}

/// 这一份预览给哪几个按钮：说了配置目录在哪的三个，没说的两个。
pub fn buttons(preview: Option<&Value>) -> &'static [Button] {
    // 目录、设置项有一样留着的就有这一格（核心 PK-5补：只有设置项的是 `{kept, settings}`）。
    let has_config = preview.is_some_and(|p| p["config"].is_object());
    if has_config {
        &[Button::Keep, Button::Purge, Button::Cancel]
    } else {
        &[Button::Keep, Button::Cancel]
    }
}

/// 正在问卸载时停着的按钮；不在问卸载的是 `None`。
pub fn selected(store: &super::Store) -> Option<Button> {
    let Some(Flow::Confirm { previews, .. }) = &store.flow else {
        return None;
    };
    let all = buttons(previews.first());
    Some(all[store.button.min(all.len() - 1)])
}

impl Settings {
    /// 问卸载时的一个键。
    pub(super) fn store_remove_key(&mut self, key: KeyEvent, texts: &Texts) {
        let count = match &self.more.store.flow {
            Some(Flow::Confirm { previews, .. }) => buttons(previews.first()).len(),
            _ => return,
        };
        let store = &mut self.more.store;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => store.ask.scroll(1),
            KeyCode::Char('k') | KeyCode::Up => store.ask.scroll(-1),
            KeyCode::PageDown => store.ask.page(true),
            KeyCode::PageUp => store.ask.page(false),
            KeyCode::Left | KeyCode::BackTab => store.button = store.button.saturating_sub(1),
            KeyCode::Right | KeyCode::Tab => store.button = (store.button + 1).min(count - 1),
            KeyCode::Enter => self.store_press(texts),
            KeyCode::Esc => self.store_cancel(texts),
            _ => {}
        }
    }

    /// 按下停着的那个按钮。
    fn store_press(&mut self, texts: &Texts) {
        match selected(&self.more.store) {
            Some(Button::Cancel) => self.store_cancel(texts),
            Some(_) if !self.more.store.ask.seen() => self.store_unread(texts),
            Some(button) => self.store_go(button == Button::Purge),
            None => {}
        }
    }
}
