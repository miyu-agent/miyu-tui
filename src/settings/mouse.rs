//! 配置页的鼠标（蓝图「配置页」第 6、15 条）：点分页换页，点一行选中、点选中的那行或双击开窗，点悬浮窗里的行、按钮。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

use ratatui::layout::Position;

use super::forms::Focus;
use super::nav::PAGES;
use super::popup::Popup;
use super::{Hit, Outcome, Settings, Texts};

impl Settings {
    /// 指针停在包页面那一行的上游上（[`Hit::Link`]）、MCP 页的「登录」上（[`Hit::Login`]）：指针变手。
    pub fn over_link(&self) -> bool {
        self.hover.is_some_and(|at| {
            self.hits
                .iter()
                .any(|(r, h)| matches!(h, Hit::Link | Hit::Login(_)) && r.contains(at))
        })
    }

    /// 左键点了 `at`；`window` 以内又点同一处的算双击。
    pub fn click(&mut self, at: Position, window: Duration, texts: &Texts) -> Outcome {
        let now = Instant::now();
        let double = self
            .last_click
            .replace((at, now))
            .is_some_and(|(was, then)| was == at && now.duration_since(then) <= window);
        let Some(hit) = self
            .hits
            .iter()
            .find(|(r, _)| r.contains(at))
            .map(|(_, h)| *h)
        else {
            return Outcome::Stay;
        };
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        match hit {
            // 包页面那一行的上游：用浏览器打开（第 43a 条）。
            Hit::Link => {
                if let Some(url) = self.link.clone() {
                    // 打不开的不用说：和正文里点链接一样，系统没有能开它的程序时什么都不做。
                    if crate::open::open(&url).is_err() {}
                }
            }
            // MCP 页的「登录」：选中那一行、去要授权页（第 43b 条）。
            Hit::Login(i) if self.popup.is_none() => {
                self.more.section = Some(super::pages::Section::Mcp(i));
                self.login(i, texts);
            }
            Hit::Menu(i) => {
                self.more.menu_at = i;
                return self.key(enter, texts);
            }
            // 「软件包」的分页条（第 43c 条）：问着、做着的时候不换。
            Hit::Tab(i) if self.more.section == Some(super::pages::Section::Store) => {
                if self.more.store.flow.is_none() {
                    let tab = super::pages::store::Tab::ALL[i.min(2)];
                    self.more.store.tab = tab;
                    self.more.store.typing = false;
                    self.store_load();
                }
            }
            Hit::Tab(i) if self.popup.is_none() => {
                self.nav.turn(PAGES[i.min(PAGES.len() - 1)]);
                self.nav.col = 0;
            }
            Hit::Row(col, row) if self.popup.is_none() => {
                let cols = self.nav.cols(&self.view);
                let again = self.nav.focus(&self.view) == col && self.nav.selected(col) == row;
                if let Some(at) = cols.iter().position(|c| *c == col) {
                    self.nav.col = at;
                }
                self.nav.pick(col, row);
                if again || double {
                    return self.key(enter, texts);
                }
            }
            Hit::FormRow(i) => {
                if let Some(Popup::Form(form)) = &mut self.popup {
                    let again = form.focus == Focus::Row(i);
                    form.focus = Focus::Row(i);
                    if again || double {
                        return self.key(enter, texts);
                    }
                }
            }
            Hit::PickRow(i) => {
                let sel = match &mut self.popup {
                    Some(Popup::Pick(pick)) => &mut pick.sel,
                    Some(Popup::Catalog(pick)) => &mut pick.sel,
                    _ => return Outcome::Stay,
                };
                let again = *sel == i;
                *sel = i;
                if again || double {
                    return self.key(enter, texts);
                }
            }
            Hit::Button(b) => {
                match &mut self.popup {
                    Some(Popup::Form(form)) => form.focus = Focus::Button(b),
                    Some(Popup::Confirm(confirm)) => confirm.sel = b,
                    _ => return Outcome::Stay,
                }
                return self.key(enter, texts);
            }
            _ => {}
        }
        Outcome::Stay
    }
}
