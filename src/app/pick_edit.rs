//! 人格框、预设框里按 `e` 编辑（蓝图 `tui.md`「新会话：人格、工作区」第 2b 条，2026-10-20 项目主人定）：直接开配置页里那
//! 一项的详情窗，关了回到原来的框、光标不动；改了开着的会话正在用的那一个、改成了的，回框以前先问要不要压缩（「是」关框
//! 做一次前台压缩，「否」、`Esc` 回框）。从 `/config` 改的不问（`settings/in_use.rs`）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::new_session::Kind;
use super::{App, Panel};
use crate::settings::Settings;
use crate::settings::in_use::Item;
use crate::transcript::Link;

impl App {
    /// 框里按了 `e`：开那一项的详情窗，记着关了回哪个框、停哪一行。
    pub(super) fn edit_pick(&mut self, kind: Kind, selected: usize, id: String) {
        let online = matches!(self.talk.transcript.link, Link::Ready);
        let item = match kind {
            Kind::Persona => Item::Persona,
            Kind::Preset => Item::Preset,
        };
        self.boxes.settings = Some(Settings::one(online, item, id));
        self.boxes.pick_back = Some((kind, selected));
        self.boxes.panel = None;
        self.send_settings_asks();
    }

    /// 从框进去的详情窗关了：存成了正在用的那一个的先问要不要压缩，别的回框。
    pub(super) fn pick_returned(&mut self, saved: &[(Item, String)]) {
        let Some((kind, selected)) = self.boxes.pick_back.take() else {
            return;
        };
        let item = match kind {
            Kind::Persona => Item::Persona,
            Kind::Preset => Item::Preset,
        };
        // 没东西可压缩的不问（2026-10-20 项目主人，核心 W-12 的 `compactable`）。
        let compactable = self.talk.transcript.cache.compactable != Some(false);
        let changed = compactable
            && self
                .current_pick(kind)
                .is_some_and(|id| saved.iter().any(|(i, s)| *i == item && *s == id));
        self.boxes.panel = Some(if changed {
            Panel::Changed {
                kind,
                selected,
                yes: true,
            }
        } else {
            Panel::Pick { kind, selected }
        });
    }

    /// 问要不要压缩的框：上下挪（框开着不能打字，`j`、`k` 也挪，`vim.rs`），`Enter` 照选的办，`Esc` 同「否」。
    pub(super) fn changed_key(
        &mut self,
        (kind, selected, yes): (Kind, usize, bool),
        key: KeyEvent,
    ) {
        let back = Panel::Pick { kind, selected };
        self.boxes.panel = match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::Tab => Some(Panel::Changed {
                kind,
                selected,
                yes: !yes,
            }),
            KeyCode::Enter if yes => {
                self.compact(None);
                None
            }
            KeyCode::Enter | KeyCode::Esc => Some(back),
            _ => self.boxes.panel,
        };
    }
}
