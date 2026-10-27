//! 鼠标：按下时落在哪一块，拖动、松开都归它；点开收起时间线；命令列表的悬停和点击（`tui.md`「鼠标」）。

use ratatui::crossterm::event::{MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use super::{App, Grab};
use crate::body_view::BodyAction;
use crate::commands::Spec;
use crate::input::Action;
use crate::ui::rows::Target;

impl App {
    /// 鼠标：按下时看落在哪一块，之后的拖动、松开都归那一块；悬停、滚轮照位置分。
    pub(super) fn mouse(&mut self, mouse: MouseEvent) -> Action {
        // 拖滚动条时终端按像素报（`pixel_mouse.rs`）：先换回格子，带小数的第几行交给正文。
        let (mouse, pixel_y) = self.screen.pixel.as_cells(mouse);
        self.screen.view.pixel_y = pixel_y;
        let action = self.mouse_cells(mouse);
        self.screen.view.pixel_y = None;
        // 抓着滚动条就开像素坐标，松开就关（`main.rs` 每帧照它开关）。
        self.screen.pixel.want = self.screen.view.bar_held();
        action
    }

    fn mouse_cells(&mut self, mouse: MouseEvent) -> Action {
        let at = Position::new(mouse.column, mouse.row);
        // 首页的吉祥物没字可看时跟着鼠标转头（`tui.md`「空会话的首页」第 6 条）。
        self.screen.pointer = Some(at);
        // 已抓住的拖选和松开始终归起点区域，经过侧栏、按钮或抽屉也不被截走。
        let captured = self.screen.grab.is_some()
            && matches!(mouse.kind, MouseEventKind::Drag(_) | MouseEventKind::Up(_));
        // 界面块：点了做那个动作、展开收起；悬停底栏那一格说是谁放的（`widgets.rs`）。
        if !captured && self.widgets_mouse(mouse, at) {
            return Action::None;
        }
        if !captured && self.jobs_mouse(mouse, at) {
            return Action::None;
        }
        // 排着队的话：点哪条退回哪条，字放回输入框，她这一轮照跑（2026-10-22 项目主人：「不如直接点击那条消息撤回」）。
        if !captured && self.screen.areas.queued.contains(at) {
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                let row = usize::from(at.y - self.screen.areas.queued.y);
                self.withdraw(row);
            }
            return Action::None;
        }
        // 点目标那一行：开长期目标的浮窗（`goal_box.rs`，2026-10-26 项目主人）。
        if !captured && matches!(mouse.kind, MouseEventKind::Down(_)) && self.goal_clicked(at) {
            return Action::None;
        }
        // 回到底部的按钮（`ui/bottom_button.rs`）。
        if !captured && self.screen.areas.bottom.contains(at) {
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.screen.view.follow();
            }
            return Action::None;
        }
        // 抽屉开着：悬停选中那一项，点一下等于 `Enter`（`tui.md`「确认和提问的抽屉」第 4 条）。
        if !captured && self.talk.drawers.open() && self.screen.areas.text.contains(at) {
            // 滚轮滚要跑的命令那一块：抽屉里只有它会滚（`ui/drawer/command.rs`）。
            if let MouseEventKind::ScrollUp | MouseEventKind::ScrollDown = mouse.kind {
                if let Some(d) = self.talk.drawers.current.as_mut() {
                    d.scroll_command(mouse.kind == MouseEventKind::ScrollDown);
                }
                return Action::None;
            }
            let row = usize::from(at.y - self.screen.areas.text.y);
            if let Some(index) = self.screen.drawer_rows.get(row).copied().flatten() {
                match mouse.kind {
                    MouseEventKind::Moved => {
                        if let Some(d) = self.talk.drawers.current.as_mut() {
                            d.cursor[d.tab] = index;
                        }
                    }
                    MouseEventKind::Down(_) => self.drawer_click(index),
                    _ => {}
                }
            }
            return Action::None;
        }
        let under = if self.screen.areas.menu.contains(at) {
            Some(Grab::Menu)
        } else if self.screen.areas.frame.contains(at) {
            Some(Grab::Input)
        } else if self.screen.areas.body.contains(at) || self.screen.view.on_bar_column(at.x, at.y)
        {
            Some(Grab::Body)
        } else {
            None
        };
        let owner = match mouse.kind {
            MouseEventKind::Down(_) => {
                self.screen.grab = under;
                under
            }
            MouseEventKind::Drag(_) => self.screen.grab,
            MouseEventKind::Up(_) => self.screen.grab.take(),
            _ => under,
        };
        if owner != Some(Grab::Body) {
            self.screen.view.hover = None;
            self.screen.view.bar_hover = false;
            // 链接的悬停也放掉：挪到正文左边的空白、输入框这些地方时，下划线和手形指针不留着（2026-10-13 验收 T-225）。
            self.screen.view.hover_link = None;
        }
        if owner != Some(Grab::Input) {
            self.typing.input.unhover();
        }
        match owner {
            Some(Grab::Menu) if self.typing.history.open => self.history_mouse(mouse),
            Some(Grab::Menu) => {
                if let Some(found) = self.mention_found() {
                    self.mention_mouse(mouse, &found)
                } else if let Some(matches) = self.menu_matches() {
                    self.menu_mouse(mouse, &matches)
                } else {
                    Action::None
                }
            }
            Some(Grab::Input) => self.typing.input.mouse(mouse, under == Some(Grab::Input)),
            Some(Grab::Body) => {
                match self.screen.view.mouse(mouse) {
                    BodyAction::None => {}
                    BodyAction::Copy(text) => return Action::Copy(text),
                    BodyAction::Toggle(target) => self.toggle(target),
                    BodyAction::Open(url) => self.open_link(&url),
                }
                Action::None
            }
            None => Action::None,
        }
    }

    /// 点了链接、附件块、文件块：本机的文件、目录和展示的一样，照「展示」第 7 条的退路开（系统给这种文件定的默认程序，
    /// 没有的交浏览器），网址用系统的打开方式；开不了的提示一句（「她的回答：Markdown」第 10 条）。
    pub(super) fn open_link(&mut self, url: &str) {
        use crate::launch::{Env, Launched, launch};
        use crate::open::OpenError;
        // 她给人看的文件：照展示的退路开（「展示」第 6、7 条）。
        if self.open_shown_link(url) {
            return;
        }
        // 本机的文件、目录：同展示（2026-10-24 项目主人：点 `.html` 进了编辑器，「和 show 用类似点击打开策略」；原来
        // 文本文件进 `$EDITOR`）。种类照文件本身问，问不到的当不认得的文件。
        if let Some(file) = crate::open::local(url) {
            if launch(&file, "application/octet-stream", &Env::current()) == Launched::Nothing {
                let words = &self.config.text.shown;
                let note = words
                    .no_program
                    .replace("{path}", &file.display().to_string());
                self.hint(note, false);
            }
            return;
        }
        let words = &self.config.text.open;
        let note = match crate::open::open(url) {
            Ok(()) => return,
            Err(OpenError::Scheme) => words.refused.replace("{url}", url),
            Err(OpenError::Missing) => words.missing.replace("{path}", url),
            Err(OpenError::Spawn(e)) => words.failed.replace("{reason}", &e.to_string()),
        };
        self.hint(note, false);
    }

    /// 编辑器用完了、回到界面：起不来的提示一句。
    pub fn edited(&mut self, result: std::io::Result<()>) {
        self.media.figures.borrow_mut().forget();
        // `Ctrl+G` 开的：读回输入框（`compose.rs`）；配置页改提示词开的：交还配置页（`settings.rs`）；引导开的交还引导（`oobe.rs`）。
        let ours = self.oobe_edited() | self.settings_prompt_edited() | self.composed();
        if ours && result.is_ok() {
            return;
        }
        if let Err(e) = result {
            let note = self
                .config
                .text
                .open
                .failed
                .replace("{reason}", &e.to_string());
            self.hint(note, false);
        }
    }

    /// 点开、收起时间线的一段或一步。
    pub(super) fn toggle(&mut self, target: Target) {
        let entries = &mut self.talk.transcript.entries;
        match target {
            Target::Segment(i) => {
                if let Some(segment) = entries.get_mut(i).and_then(|e| e.segment.as_mut()) {
                    segment.open = Some(!segment.expanded(self.config.timeline.fold));
                }
            }
            Target::Entry(i) => {
                if let Some(entry) = entries.get_mut(i) {
                    entry.open = !entry.open;
                }
            }
            Target::Step(i, j) => {
                let segment = entries.get_mut(i).and_then(|e| e.segment.as_mut());
                if let Some(step) = segment.and_then(|s| s.steps.get_mut(j)) {
                    step.open = Some(!step.opened(&self.config.timeline));
                }
            }
            // 回答里的 `<details>`：点过的记在这一条上，再点一下去掉（和写的 `open` 反过来）。
            Target::Details(i, k) => {
                if let Some(entry) = entries.get_mut(i) {
                    match entry.details.iter().position(|&d| d == k) {
                        Some(at) => {
                            entry.details.remove(at);
                        }
                        None => entry.details.push(k),
                    }
                }
            }
        }
    }

    /// 输入历史列表上的鼠标：悬停选中那一条，点一下等于 `Enter`（`tui.md`「输入历史列表」第 5 条）。
    fn history_mouse(&mut self, mouse: MouseEvent) -> Action {
        use ratatui::crossterm::event::MouseButton;
        // 下面要改列表的状态：先拷一份列的东西。
        let list = self.list_items().0.into_owned();
        let found = self.typing.history.matches(&list);
        let width = self.screen.areas.menu_text.width;
        let now = std::time::SystemTime::now();
        // 和画出来的同一份：放进列表那块的高度（「窗口小的时候」第 1 条）。
        let max = usize::from(self.screen.areas.menu_text.height);
        // 悬停钉住、滚轮滚（最新的贴底：滚轮往上是更早的），照钉住以后画出来的行认指针下是哪一条。
        let Some(by) = list_step(mouse.kind, true) else {
            return Action::None;
        };
        let shown = self.config.layout.history_rows.max(1);
        let history = &mut self.typing.history;
        crate::menu::pin(
            history.selected,
            &mut history.pinned,
            found.len(),
            shown,
            by,
        );
        let (_, rows) =
            crate::ui::history_lines(&self.typing.history, &found, width, &self.config, now, max);
        let at = crate::ui::history_index_at(self.screen.areas.menu_text, &rows, mouse.row);
        if let Some(index) = at {
            self.typing.history.selected = index;
            if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                self.pick_history();
            }
        }
        Action::None
    }

    /// 列表上的鼠标（「斜杠命令列表」第 3 条）：悬停高亮指针下那一条、列表不动，滚轮滚，点一下就执行。
    pub(super) fn menu_mouse(&mut self, mouse: MouseEvent, matches: &[Spec]) -> Action {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        // 露了几条照画出来的：框里放字的那一块有几行（「窗口小的时候」第 1 条）。
        let rows = crate::ui::menu_rows(
            self.config.layout.menu_rows,
            self.screen.areas.menu_text.height,
        );
        let count = matches.len();
        let Some(by) = list_step(mouse.kind, false) else {
            return Action::None;
        };
        crate::menu::pin(
            self.typing.menu.selected,
            &mut self.typing.menu.pinned,
            count,
            rows,
            by,
        );
        let top = crate::menu::top(
            self.typing.menu.selected,
            self.typing.menu.pinned,
            count,
            rows,
        );
        let at = crate::ui::menu_index_at(self.screen.areas.menu_text, count, top, rows, mouse.row);
        let Some((index, spec)) = at.and_then(|i| Some((i, matches.get(i)?))) else {
            return Action::None;
        };
        self.typing.menu.selected = index;
        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
            let spec = spec.clone();
            self.typing.input.editor.take();
            self.run(&spec, None);
        }
        Action::None
    }
}

/// 列表上的鼠标这一下要把露的那一段挪几条：悬停、按下左键钉住不挪（0），滚轮一格一条；别的不管。`upward`：列表
/// 从下往上排（输入历史列表，最新的贴底），滚轮往上是往后翻。
pub(super) fn list_step(kind: MouseEventKind, upward: bool) -> Option<isize> {
    use ratatui::crossterm::event::MouseButton;
    let sign = if upward { -1 } else { 1 };
    match kind {
        MouseEventKind::Moved | MouseEventKind::Down(MouseButton::Left) => Some(0),
        MouseEventKind::ScrollDown => Some(sign),
        MouseEventKind::ScrollUp => Some(-sign),
        _ => None,
    }
}
