//! 扩展的整页接到程序上（蓝图 `tui.md`「界面块」第 2、7 条）：开着时整屏归它，按键、鼠标都归它。`↑` `↓` 在能点的之间走
//! （同时滚，一个能点的都没有的一行行滚），`PgUp` `PgDn` 一页一页滚，滚轮也滚，`Enter`、点一下做那个动作，`Esc` 回到打开
//! 以前的地方（输入框里的字、正文滚到哪都照原样：整页不碰它们）。

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::layout::Position;

use super::App;
use crate::widgets::{PageRef, PageView};

/// 滚轮一格滚几行。
const WHEEL: usize = 3;

impl App {
    /// 打开一整页：从头看起。
    pub(super) fn open_widget_page(&mut self, target: PageRef) {
        self.boxes.widget_page = Some(PageView {
            target,
            scroll: 0,
            selected: 0,
        });
        self.screen.draw_now = true;
    }

    /// 整页开着时的一个事件。
    pub(super) fn widget_page_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Release => {}
            Event::Key(key) => self.widget_page_key(key.code),
            Event::Mouse(mouse) => {
                let at = Position::new(mouse.column, mouse.row);
                self.screen.pointer = Some(at);
                match mouse.kind {
                    MouseEventKind::ScrollDown => self.page_scroll(WHEEL, true),
                    MouseEventKind::ScrollUp => self.page_scroll(WHEEL, false),
                    _ => {
                        self.widgets_mouse(mouse, at);
                    }
                }
            }
            _ => {}
        }
    }

    fn widget_page_key(&mut self, code: KeyCode) {
        let frame = &self.screen.widget_page;
        let (actions, view) = (frame.actions.len(), frame.view.max(1));
        match code {
            KeyCode::Esc => {
                self.boxes.widget_page = None;
                self.screen.draw_now = true;
            }
            KeyCode::Down | KeyCode::Char('j') if actions > 0 => self.page_select(true),
            KeyCode::Up | KeyCode::Char('k') if actions > 0 => self.page_select(false),
            KeyCode::Down | KeyCode::Char('j') => self.page_scroll(1, true),
            KeyCode::Up | KeyCode::Char('k') => self.page_scroll(1, false),
            KeyCode::PageDown => self.page_scroll(view, true),
            KeyCode::PageUp => self.page_scroll(view, false),
            KeyCode::Enter => {
                let chosen = self.boxes.widget_page.as_ref().and_then(|page| {
                    let action = self.screen.widget_page.actions.get(page.selected)?;
                    Some((page.target.package.clone(), action.1.clone()))
                });
                if let Some((package, action)) = chosen {
                    self.widget_act(&package, action);
                }
            }
            _ => {}
        }
    }

    /// 选下一个、上一个能点的，让它露在屏上。
    fn page_select(&mut self, down: bool) {
        let frame = &self.screen.widget_page;
        let Some(page) = self.boxes.widget_page.as_mut() else {
            return;
        };
        let last = frame.actions.len().saturating_sub(1);
        page.selected = if down {
            (page.selected + 1).min(last)
        } else {
            page.selected.saturating_sub(1)
        };
        if let Some((row, _)) = frame.actions.get(page.selected) {
            let view = frame.view.max(1);
            if *row < page.scroll {
                page.scroll = *row;
            } else if *row >= page.scroll + view {
                page.scroll = row + 1 - view;
            }
        }
    }

    /// 滚 `by` 行，到头就停。
    fn page_scroll(&mut self, by: usize, down: bool) {
        let frame = &self.screen.widget_page;
        let most = frame.rows.saturating_sub(frame.view);
        if let Some(page) = self.boxes.widget_page.as_mut() {
            page.scroll = if down {
                (page.scroll + by).min(most)
            } else {
                page.scroll.saturating_sub(by)
            };
        }
    }
}
