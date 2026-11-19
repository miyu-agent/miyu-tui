//! 选了带 `notice` 的选项以后弹的提示窗（蓝图「配置页」第 33 条「选项的说明和提示」，核心设计 72）：用「问一句」的窗，标题写
//! 选项的名字，下面写核心给的那一句，只有一个按钮「知道了」。选的值照常存，不等它关；`Enter`、`Esc`、点按钮都是关掉。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::settings::Settings;
use crate::settings::popup::Popup;

/// 提示窗：标题（选项的名字）、那一句。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// 选项的名字。
    pub title: String,
    /// 核心给的那一句（照连接的语言）。
    pub text: String,
}

impl Settings {
    /// 提示窗的按键：`Enter`、`Esc`、`q`、空格关掉，别的键不管、窗留着。
    pub(in crate::settings) fn notice_key(&mut self, notice: Notice, key: KeyEvent) {
        if !matches!(
            key.code,
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q' | ' ')
        ) {
            self.popup = Some(Popup::Notice(notice));
        }
    }

    /// 选择窗里选好了以后：选的那一项带 `notice`、又不是开窗时的值，存完接着弹提示窗。存没开别的窗（存失败的原因写在状态行）
    /// 时才弹，不盖掉别的窗。
    pub(in crate::settings) fn show_notice(&mut self, notice: Option<Notice>) {
        if let Some(notice) = notice
            && self.popup.is_none()
        {
            self.popup = Some(Popup::Notice(notice));
        }
    }
}
