//! 程序的状态，和把终端事件、核心的消息分给各块。按键在 `keys.rs`，鼠标在 `mouse.rs`。

mod asking;
mod attachments;
mod avatars;
mod boxes;
mod build;
mod cards;
mod catalog;
mod compose;
mod config_asks;
mod deadline;
mod diagrams;
mod drawer;
mod effort;
mod frame;
mod goal;
mod jobs;
mod jump;
mod widget_page;
mod widgets;

pub use avatars::{Avatar, Place};
pub use frame::Lists;
pub use jobs::Panel;
pub use mascot::MascotState;
pub use media::Media;
pub use screen::Screen;
pub use seen::Seen;
pub use usage::UsageData;
mod head_settings;
mod help;
mod keys;
mod language;
mod link;
mod look;
mod lost;
mod mascot;
mod mascot_pick;
mod media;
mod mention;
mod models;
mod mouse;
pub mod new_session;
mod notify;
mod oobe;
mod output;
mod packages;
mod pages;
mod paste;
mod redo;
mod screen;
mod seen;
mod session;
mod sessions;
mod settings;
mod switch;
mod takeback;
mod talk;
mod theme;
mod theme_box;
mod typing;
mod updates;
pub mod usage;
mod vim;
mod workspace;

use std::time::{Duration, Instant};

use crate::human::Human;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::commands::{self, Spec};
use crate::config::Config;
use crate::core::Core;
use crate::drawer::Drawers;
use crate::figures::Figures;
use crate::focus::Focus;
use crate::history::History;
use crate::input::{Action, Draft, InputBox, PasteRule};
use crate::jobs::Board;
use crate::menu::Menu;
use crate::notify::Notifier;
use crate::tips::Tips;
use crate::transcript::Transcript;

/// 按下鼠标时落在哪一块：拖动、松开都归它，拖出了那一块也一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Grab {
    Menu,
    Input,
    Body,
}

/// 一条会自己消失的提示。
#[derive(Debug)]
pub struct Notice {
    /// 显示的字。
    pub text: String,
    /// 接在后面、放不下时也整个留着的一截（`/goal` 看进度的「 · 进行中 · 第 1 轮」）：截前面的字；没有的是空的。
    pub tail: String,
    /// 什么时候消失。
    pub until: Instant,
    /// 是好消息（复制成了）：绿；别的提示暗。
    pub good: bool,
}

/// 最近发出去的一句，没发出去时撤回来用（「输入框」第 12、13 条）。
struct Unsent {
    /// 放回输入框的字：说的话、编辑上一句改过的；`/redo` 原样重来的没有。
    draft: Option<Draft>,
    /// 编辑上一句的，改之前的那句：放回去以后接着编辑。
    original: Option<Draft>,
    /// 重做先藏掉的那一轮：显示回来。
    turn: Option<u64>,
}

/// 整个程序的状态。
pub struct App {
    /// 界面上的字和布局的数值。
    pub config: Config,
    /// 系统通知、报给 herdr（蓝图「系统通知」）。
    pub notifier: Notifier,
    /// 开着的框和整屏页（`boxes.rs`，蓝图 `app.md`）。
    pub boxes: boxes::Boxes,
    /// 输入框和它上面的列表、焦点、编辑器、输入法、提示（`typing.rs`，蓝图 `app.md`）。
    pub typing: typing::Typing,
    /// 连着核心的一头和核心给的资料（`link.rs`，蓝图 `app.md`）。
    pub link: link::Link,
    /// 会话：正在看的和停放着的、切换、确认和提问（`talk.rs`，蓝图 `app.md`）。
    pub talk: talk::Talk,
    /// 要画的东西的账：排好的行、做好的图、链接卡片、mermaid、头像、吉祥物包（`media.rs`，蓝图 `app.md`）。
    pub media: media::Media,
    /// 上一帧画出来的：各块位置、视口、光标、行表、鼠标（`screen.rs`，蓝图 `app.md`）。
    pub screen: screen::Screen,
    /// 吉祥物和运行状态行会动的那几样（`mascot.rs`，蓝图 `app.md`）。
    pub mascot: mascot::MascotState,
    /// 工作目录，家目录写成 `~`：侧边栏照它写。
    pub cwd: String,
    /// 该退出了。
    pub quit: bool,
    /// 按了 Ctrl+Z，主循环该把程序挂起到后台了。
    pub suspend: bool,
}

impl App {
    /// 这一下 Esc 归输入框（打断、清空）：列表（命令列表、输入历史列表）、后台面板、抽屉开着的，焦点在别处的，
    /// 有选区的，都先归它们（`13-终端界面.md` 第十节「由近及远」，`tui.md`「按键」Esc）。
    fn esc_for_input(&self, menu_open: bool) -> bool {
        !menu_open
            && !self.typing.history.open
            && self.boxes.panel.is_none()
            && self.typing.focus == Focus::Input
            && !self.talk.drawers.open()
            && self.typing.input.editor.selection().is_none()
            && self.screen.view.select.is_none()
    }

    /// 退出程序前收尾：输入法还原成进来时的样子（`ime.rs`）。
    pub fn finish(&mut self) {
        std::mem::take(&mut self.typing.ime).finish();
        // 这一回的样子记下，下次起来连上以前先照它画（`look.rs`）。
        self.save_look();
    }

    /// 处理一个终端事件。
    pub fn handle(&mut self, event: Event) {
        self.mascot.heard(&event, Instant::now());
        // 引导开着：按键、鼠标、粘贴都归它（`oobe.rs`）。
        if self.boxes.oobe.is_some() {
            self.oobe_event(event);
            return;
        }
        // 配置页开着：按键、鼠标、粘贴都归它（`settings.rs`）。
        if self.boxes.settings.is_some() {
            self.settings_event(event);
            return;
        }
        // 扩展的整页开着：按键、鼠标都归它（`widget_page.rs`）。
        if self.boxes.widget_page.is_some() {
            self.widget_page_event(event);
            return;
        }
        let menu_open = self.menu_matches().is_some() || self.mention_found().is_some();
        let event = self.vim_keys(event, menu_open);
        let action = match event {
            // Windows 上松开键也报一次，只认按下和按住。
            Event::Key(key) if key.kind == KeyEventKind::Release => Action::None,
            // 在子会话里看：输入框空着时 Esc 回主会话（「切进子会话」第 4 条）；打断它按 Ctrl+C。
            Event::Key(key)
                if key.code == KeyCode::Esc
                    && self.talk.home.is_some()
                    && self.esc_for_input(menu_open)
                    && self.typing.input.editor.is_empty() =>
            {
                self.leave_child();
                Action::None
            }
            // 编辑上一句时一下就取消（「输入框」第 13 条），不照下面有字时两下清空的规矩。
            Event::Key(key)
                if key.code == KeyCode::Esc
                    && self.typing.input.editing()
                    && self.esc_for_input(menu_open)
                    && self.talk.transcript.running.is_none() =>
            {
                self.typing.input.cancel_edit();
                Action::None
            }
            // Esc 由近及远（`13-终端界面.md` 第十节）：列表、选区归 `key`；在回答时（发出去了、还没开轮的也算）第一下
            // 只提示，一小会儿以内再按一下才打断；没在回答、有字时，两下清空。
            Event::Key(key)
                if key.code == KeyCode::Esc
                    && self.esc_for_input(menu_open)
                    && (self.talk.transcript.running.is_some()
                        || self.talk.transcript.starting != crate::transcript::Starting::No
                        || !self.typing.input.editor.is_empty()) =>
            {
                self.esc();
                Action::None
            }
            Event::Key(key) => self.key(key),
            Event::Paste(text) => {
                self.paste(&text);
                Action::None
            }
            Event::Mouse(mouse) => self.mouse(mouse),
            // 终端报的在不在前台：系统通知照它（「系统通知」第 2 条）。
            Event::FocusGained | Event::FocusLost => {
                let gained = matches!(event, Event::FocusGained);
                self.notifier.focus(gained);
                // 拿回焦点：跟随壁纸的再找一次（只能靠命令换壁纸的也跟得上，「主题」的「跟随壁纸」第 3 条），跟着终端颜色的
                // 再问一次终端（「主题」第 11 条）。
                if gained {
                    self.theme_focus();
                }
                Action::None
            }
            _ => Action::None,
        };
        match action {
            Action::None => {}
            Action::Submit(text) => self.submit(text),
            Action::Copy(text) => self.copy(&text),
            Action::Quit => self.quit = true,
            // Ctrl+C 分级：输入框空着时，在回答就打断（排着队的退回输入框），不在回答才提示用 Ctrl+D 退出。
            Action::ExitHint if self.talk.transcript.running.is_some() => self.interrupt(),
            Action::ExitHint => self.hint(self.config.text.exit_hint.clone(), false),
            Action::Cleared => self.hint(self.config.text.input_cleared.clone(), false),
            Action::Open(path) => self.open_link(&path),
        }
    }

    /// 在输入框左上方写一条提示，新的顶掉旧的。
    fn hint(&mut self, text: String, good: bool) {
        self.hint_keeping(text, String::new(), good);
    }

    /// 同 [`App::hint`]，后面接着 `tail`：放不下时截前面的字，它整个留着。
    fn hint_keeping(&mut self, text: String, tail: String, good: bool) {
        // 好几行的（扩展办的斜杠命令的回执）多停一会儿：一行一份，最多五份。
        let lines = u32::try_from(text.lines().count().clamp(1, 5)).unwrap_or(1);
        let stay = Duration::from_millis(self.config.layout.notice_ms) * lines;
        self.typing.notice = Some(Notice {
            text,
            tail,
            until: Instant::now() + stay,
            good,
        });
    }

    /// 吉祥物画着：在首页或宽屏的侧边栏里，那里的开关开着，人格也没有头像替掉它（`tui.md`「后台命令、子代理和侧边栏」
    /// 第 7 条，「空会话的首页」第 10 条）。
    fn mascot_shown(&self) -> bool {
        let layout = &self.config.layout;
        (self.talk.blank() && layout.mascot_home && self.seen().avatar(Place::Home) == Avatar::None)
            || (self.screen.areas.sidebar.width > 0
                && layout.mascot_sidebar
                && self.seen().avatar(Place::Sidebar) == Avatar::None)
    }

    /// 首页吉祥物那一块在：吉祥物或者头像（被顶上去、走下来照它）。
    fn home_slot_shown(&self) -> bool {
        self.talk.blank()
            && (self.config.layout.mascot_home || self.seen().avatar(Place::Home) != Avatar::None)
    }

    /// 到点了：收掉过期的提示。
    pub fn tick(&mut self) {
        // 读配置攒着的还没发的一起发（`config_asks.rs`；平常办完一条消息就发了）。
        self.flush_config_asks();
        // 引导走完了：关掉，回到首页（`oobe.rs`）。
        self.oobe_tick();
        // 附件照模型收：收不了的那几种交给输入框，刚改成文件块的提示一句（`attachments.rs`）。
        self.sync_refused();
        // 做完的待办全打勾露够了：收掉（「后台命令、子代理和侧边栏」第 4 条）。
        self.talk.board.expire_todos(Instant::now());
        // 配置页、引导里不在打字的时候输入法关成英文，打字时开回来；对话里的输入框一直算在打字（`ime.rs`）。
        let typing = match (&self.boxes.oobe, &self.boxes.settings) {
            (Some(oobe), _) => oobe.editing(),
            (None, Some(page)) => page.typing(),
            (None, None) => true,
        };
        self.typing.ime.typing(typing);
        self.advance_jobs();
        // 老会话按页读：最上面那一行露出来了，要更早的一页（`pages.rs`）。
        self.ask_older();
        self.send_card_asks();
        self.ask_avatars();
        // 引导「主题」那一步选着的、跟随头像的主色变了，换一套（`theme.rs`）。
        self.follow_theme();
        self.send_diagram_asks();
        self.renumber_attachments();
        // 压缩那一行的进度条追一下（`tui.md`「正文」第 9 条）。
        let layout = &self.config.layout;
        self.talk.transcript.climb(
            Instant::now(),
            layout.bar.width,
            &layout.compaction,
            &mut self.screen.rng,
        );
        self.drawer_tick();
        // 配色框没定下就关了：换回打开以前的（`theme_box.rs`）。
        self.theme_box_closed();
        // 界面块照屏上这一刻有的位置订阅，变了才再订（`widgets.rs`）。
        self.sync_widgets();
        if self
            .typing
            .notice
            .as_ref()
            .is_some_and(|n| n.until <= Instant::now())
        {
            self.typing.notice = None;
        }
    }

    /// 列表开着时筛出来的命令；没开是 `None`。每次按输入框里现在的字重新筛，顺手定开不开。
    pub fn menu_matches(&mut self) -> Option<Vec<Spec>> {
        // 翻输入历史翻出来的命令还没改过：不弹，`↑` `↓` 接着翻（蓝图「按键」`↑`、`↓`）。
        if self.typing.input.recalled() {
            return None;
        }
        let text = self.typing.input.editor.text();
        let typed = commands::menu_typed(text);
        // 列表刚弹出来：问一次核心认哪些（会话换了、写了新模板的照新的列，`catalog.rs`）。
        let opening = typed.is_some() && !self.typing.menu.typing();
        let matches: Vec<Spec> = typed
            .map(|t| {
                self.config
                    .commands
                    .filter(t)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let open = self
            .typing
            .menu
            .sync(text, typed, matches.len())
            .then_some(matches);
        if opening {
            self.ask_catalog();
        }
        open
    }
}
