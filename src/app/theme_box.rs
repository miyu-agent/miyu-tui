//! `/theme` 的配色框（蓝图 `tui.md`「主题」第 6 条，2026-10-15 项目主人：出列表选，选好写进设置）：和界面语言的框一个位置、
//! 一个样子。挪到哪一种当场换成哪一种的样子（和引导、配置页选配色一个规矩），`Enter` 写进个人设置的 `tui.theme`，`Esc`
//! 换回打开以前的。框被别的方式关掉（开了别的框）也换回去，不留一个没写进设置的样子。

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use super::{App, Panel};
use crate::core::Command;
use crate::theme::choice::COLORS;

/// 开着的配色框：打开以前用的那种、列出来的几种。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeBox {
    pub before: String,
    pub options: Vec<&'static str>,
}

impl App {
    /// 开框：列几种配色（找不到桌面壁纸的不列跟随壁纸，正用着的照列），选着现在用的那种。
    pub(super) fn open_themes(&mut self) {
        let before = self.screen.theme_choice.colors.clone();
        let wallpaper = self.screen.wallpaper.available();
        let options: Vec<&'static str> = COLORS
            .iter()
            .copied()
            .filter(|c| wallpaper || *c != "wallpaper" || *c == before)
            .collect();
        let selected = options.iter().position(|c| *c == before).unwrap_or(0);
        self.boxes.theme_box = Some(ThemeBox { before, options });
        self.boxes.panel = Some(Panel::Theme { selected });
    }

    /// 框开着时的按键。
    pub(super) fn theme_key(&mut self, selected: usize, key: KeyEvent) {
        let last = self
            .boxes
            .theme_box
            .as_ref()
            .map_or(0, |b| b.options.len().saturating_sub(1));
        match key.code {
            KeyCode::Up => self.pick_theme(selected.saturating_sub(1)),
            KeyCode::Down => self.pick_theme((selected + 1).min(last)),
            KeyCode::Enter => self.choose_theme(selected),
            KeyCode::Esc => {
                self.boxes.panel = None;
                self.theme_box_closed();
            }
            _ => {}
        }
    }

    /// 鼠标在框上：悬停选中（当场换样子），点一下定下；交回归不归它。点在框的边上不算点中哪一种。
    pub(super) fn theme_mouse(&mut self, mouse: MouseEvent, at: Position) -> bool {
        let Some(Panel::Theme { selected }) = self.boxes.panel else {
            return false;
        };
        let areas = self.screen.areas;
        if !areas.menu.contains(at) {
            return false;
        }
        let row = at.y.checked_sub(areas.menu_text.y).map(usize::from);
        let index = row.and_then(|row| self.screen.panel_rows.get(row).copied().flatten());
        if let Some(index) = index.filter(|_| areas.menu_text.contains(at)) {
            if index != selected {
                self.pick_theme(index);
            }
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.choose_theme(index);
            }
        }
        true
    }

    /// 选着第 `index` 种：当场换成它的样子（还不写设置）。
    fn pick_theme(&mut self, index: usize) {
        let Some(value) = self
            .boxes
            .theme_box
            .as_ref()
            .and_then(|b| b.options.get(index).copied())
        else {
            return;
        };
        self.boxes.panel = Some(Panel::Theme { selected: index });
        self.show_colors(value);
    }

    /// 定下第 `index` 种：写进个人设置（和配置页外观页一样），关框；选的就是原来的只关框。
    fn choose_theme(&mut self, index: usize) {
        let Some(open) = self.boxes.theme_box.take() else {
            return;
        };
        self.boxes.panel = None;
        let Some(value) = open.options.get(index).copied() else {
            self.show_colors(&open.before);
            return;
        };
        self.show_colors(value);
        if value != open.before {
            self.link.core.send(Command::SetTheme(value.to_string()));
        }
    }

    /// 框关了却没定下（`Esc`、开了别的框）：换回打开以前的那种。每一拍看一次（`tick`）。
    pub(super) fn theme_box_closed(&mut self) {
        if matches!(self.boxes.panel, Some(Panel::Theme { .. })) {
            return;
        }
        if let Some(open) = self.boxes.theme_box.take() {
            self.show_colors(&open.before);
        }
    }

    /// 照 `value` 这种配色画：换掉选着的配色，主色照它重挑，当场换上。
    fn show_colors(&mut self, value: &str) {
        if self.screen.theme_choice.colors == value {
            return;
        }
        self.screen.theme_choice.colors = value.to_string();
        self.screen.theme_seed = self.wanted_seed();
        self.apply_theme();
    }
}
