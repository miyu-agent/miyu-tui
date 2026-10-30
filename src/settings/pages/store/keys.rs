//! 「软件包」三页的按键（蓝图「配置页」第 43c 条）：问的时候只认 `y`、`n`；做的时候不收；打筛的字时字归筛；平常换页、
//! 挪、各页自己的那几样。`Esc` 先收打字、再清筛的字、再清勾选，都没有了才回主菜单。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::Tab;
use super::flow::{Flow, say};
use crate::settings::pages::Section;
use crate::settings::pages::packages::page_of;
use crate::settings::{Outcome, Settings, Texts, Tone, Waiting};

impl Settings {
    /// 从主菜单进「软件包」：停在「下载软件」，要能升级的、软件源（第 43c 条）。
    pub(in crate::settings) fn store_enter(&mut self) -> Option<Section> {
        self.more.store.tab = Tab::Source;
        let ask = Waiting::Store(super::Ask::Upgrades);
        self.ask("package.upgrades", serde_json::json!({}), ask);
        self.store_load();
        Some(Section::Store)
    }

    /// 进了「软件包」时的一个键。
    pub(in crate::settings) fn store_key(&mut self, key: KeyEvent, texts: &Texts) -> Outcome {
        self.status = self.status.take().filter(|(_, t)| *t == Tone::Busy);
        match &self.more.store.flow {
            Some(Flow::Confirm { .. }) => {
                match key.code {
                    KeyCode::Char('y' | 'Y') | KeyCode::Enter => self.store_go(),
                    KeyCode::Char('n' | 'N') | KeyCode::Esc => self.store_cancel(texts),
                    _ => {}
                }
                return Outcome::Stay;
            }
            Some(_) => return Outcome::Stay,
            None => {}
        }
        if self.more.store.typing {
            self.store_typing(key);
        } else if self.store_common(key) {
            return Outcome::Stay;
        } else {
            match self.more.store.tab {
                Tab::Source => self.store_source_key(key, texts),
                Tab::Installed => self.store_installed_key(key, texts),
                Tab::Backstage => self.store_backstage_key(key, texts),
            }
        }
        self.store_load();
        Outcome::Stay
    }

    /// 三页都认的：换页、挪、`Esc`。认了的交回 `true`。
    fn store_common(&mut self, key: KeyEvent) -> bool {
        let store = &mut self.more.store;
        let at = store.cursor();
        match key.code {
            KeyCode::Char(',' | '，' | 'h') | KeyCode::Left => store.turn(false),
            KeyCode::Char('.' | '。' | 'l') | KeyCode::Right => store.turn(true),
            KeyCode::Char('j') | KeyCode::Down => store.go(at + 1),
            KeyCode::Char('k') | KeyCode::Up => store.go(at.saturating_sub(1)),
            KeyCode::Esc | KeyCode::Char('q') => {
                if !store.query.is_empty() {
                    store.query.clear();
                } else if !store.picked.is_empty() {
                    store.picked.clear();
                } else {
                    self.more.section = None;
                    self.on_menu = true;
                    return true;
                }
            }
            _ => return false,
        }
        self.store_load();
        true
    }

    /// 打筛的字：`Enter` 收起打字、筛出来的留着；`Esc` 不筛了；字删光了再按退格也一样。
    fn store_typing(&mut self, key: KeyEvent) {
        let store = &mut self.more.store;
        let at = store.cursor();
        match key.code {
            KeyCode::Enter => store.typing = false,
            KeyCode::Esc => {
                store.typing = false;
                store.query.clear();
            }
            KeyCode::Backspace if store.query.is_empty() => store.typing = false,
            KeyCode::Backspace => {
                store.query.pop();
                store.go(0);
            }
            KeyCode::Down => store.go(at + 1),
            KeyCode::Up => store.go(at.saturating_sub(1)),
            KeyCode::Char(c) => {
                store.query.push(c);
                store.go(0);
            }
            _ => {}
        }
    }

    /// 「下载软件」：`/` 打字筛，`Tab` 勾选、挪到下一行，`s` 换排序，`Enter` 装。
    fn store_source_key(&mut self, key: KeyEvent, texts: &Texts) {
        let store = &mut self.more.store;
        match key.code {
            KeyCode::Char('/') => {
                store.typing = true;
                store.query.clear();
            }
            KeyCode::Tab => {
                let at = store.cursor();
                if let Some(id) = store.current().map(|r| r.id().to_string())
                    && !store.picked.remove(&id)
                {
                    store.picked.insert(id);
                }
                store.go(at + 1);
            }
            KeyCode::Char('s') => {
                if let super::Listing::Got(list) = &store.source {
                    store.sort = super::sort::next(store.sort, list);
                    store.go(0);
                }
            }
            KeyCode::Enter => self.store_install(texts),
            _ => {}
        }
    }

    /// 「已安装」：`Enter` 进设置、空格开关、`d` 卸、`u` 升这一个、`U` 全部升。
    fn store_installed_key(&mut self, key: KeyEvent, texts: &Texts) {
        match key.code {
            KeyCode::Enter => self.store_settings(texts),
            KeyCode::Char(' ') => self.store_toggle(),
            KeyCode::Char('d') => self.store_remove(),
            KeyCode::Char('u') => self.store_upgrade(false, texts),
            KeyCode::Char('U') => self.store_upgrade(true, texts),
            _ => {}
        }
    }

    /// 「软件后台」`Enter`：交给 App 在后台跑 `miyu web --package`（`backstage.rs`）。
    fn store_backstage_key(&mut self, key: KeyEvent, texts: &Texts) {
        if key.code != KeyCode::Enter {
            return;
        }
        let Some((id, name)) = self
            .more
            .store
            .current()
            .map(|r| (r.id().to_string(), r.name().to_string()))
        else {
            return;
        };
        if crate::backstage::program().is_none() {
            let line = say(&texts.more.store.status, "run_web").replace("{id}", &id);
            return self.say(line, Tone::Note);
        }
        self.more.store.web = Some((id, name));
    }

    /// 「已安装」`Enter`：有设置项的进它自己那一页，没有的状态行说一句。
    fn store_settings(&mut self, texts: &Texts) {
        let Some(id) = self.more.store.current().map(|r| r.id().to_string()) else {
            return;
        };
        if self.more.packages().iter().any(|p| p.id == id) {
            self.more.section = Some(Section::Page(page_of(&id), 0));
        } else {
            let line = say(&texts.more.store.status, "no_settings");
            self.say(line, Tone::Note);
        }
    }
}
