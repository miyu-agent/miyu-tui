//! 界面块接到程序上（蓝图 `tui.md`「界面块」第 8 条）：每一拍照屏上这一刻有的位置算订阅的说法，和上一次说的不一样才
//! 再订阅；核心推来的整份、一块一块换进 `link.widgets`。配置页、引导开着时什么都看不到，不订阅（核心不叫程序干活）。

use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use super::App;
use crate::core::{Command, Update};
use crate::ui::widget::Kind;
use crate::widgets::{Action, Place, View};

impl App {
    /// 屏上这一刻有的位置：首页 `home`、`status`；对话里宽屏 `sidebar`、`status`，窄屏只有 `status`。整屏被配置页、引导
    /// 占着的是 `None`。
    fn widget_view(&self) -> Option<View> {
        if self.boxes.settings.is_some() || self.boxes.oobe.is_some() {
            return None;
        }
        if self.talk.blank() {
            return Some(View {
                session: None,
                showing: vec![Place::Home, Place::Status],
                page: None,
            });
        }
        let mut showing = Vec::new();
        if self.screen.areas.sidebar.width > 0 {
            showing.push(Place::Sidebar);
        }
        showing.push(Place::Status);
        Some(View {
            // 看着子会话时是那个子会话（`transcript.session` 就是正在看的）。
            session: self.talk.transcript.session.clone(),
            showing,
            page: None,
        })
    }

    /// 说法变了才再订阅（换会话、首页和对话之间换、窗口跨过侧边栏的宽度、开关配置页）。
    pub(super) fn sync_widgets(&mut self) {
        let view = self.widget_view();
        if view == self.link.widgets.sent {
            return;
        }
        self.link.widgets.sent.clone_from(&view);
        self.link.core.send(Command::Widgets(view));
    }

    /// 界面块的几条更新；办了的交回 `true`。
    pub(super) fn widget_update(&mut self, update: &Update) -> bool {
        match update {
            Update::Widgets(blocks) => self.link.widgets.listed(blocks.clone()),
            Update::WidgetChanged { package, id, block } => {
                self.link.widgets.changed(package, id, block.clone());
            }
            Update::PackageCallRefused { message } => {
                let said = message.trim_end().trim_end_matches(['。', '.']);
                if !said.is_empty() {
                    self.hint(said.to_string(), false);
                }
            }
            _ => return false,
        }
        true
    }

    /// 鼠标落在界面块上（蓝图「界面块」第 3、5、7 条）：点能做事的做、点收起来的块展开收起；指针挪到底栏的另一格上，提示是
    /// 哪个软件放的。办了的交回 `true`；悬停不算办了，别处的悬停照旧。
    pub(super) fn widgets_mouse(&mut self, mouse: MouseEvent, at: Position) -> bool {
        let spot = self
            .screen
            .widget_spots
            .iter()
            .find(|s| s.rect.contains(at))
            .cloned();
        match mouse.kind {
            MouseEventKind::Moved => {
                let status = spot
                    .filter(|s| s.what == Kind::Status)
                    .map(|s| (s.package, s.id));
                if status != self.screen.status_hover {
                    self.screen.status_hover.clone_from(&status);
                    if let Some(said) = status.and_then(|(p, i)| self.widget_owner(&p, &i)) {
                        self.hint(said, false);
                    }
                }
                false
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(spot) = spot else {
                    return false;
                };
                match spot.what {
                    Kind::Act(action) => self.widget_act(&spot.package, action),
                    Kind::Toggle => self.link.widgets.toggle(&spot.package, &spot.id),
                    Kind::Status => return false,
                }
                true
            }
            _ => false,
        }
    }

    /// 这一块是谁放的：`<软件名> · <标题>`，没写标题的只写软件名。
    fn widget_owner(&self, package: &str, id: &str) -> Option<String> {
        let block = self.link.widgets.find(package, id)?;
        let (title, source) = block.heading();
        Some(title.map_or_else(|| source.to_string(), |t| format!("{source} · {t}")))
    }

    /// 做一个动作：调方法交给核心（回应不看，拒了的弹原话）；复制照拖选的复制。
    fn widget_act(&mut self, package: &str, action: Action) {
        match action {
            Action::Call { method, params } => self.link.core.send(Command::PackageCall {
                package: package.to_string(),
                method,
                params,
            }),
            Action::Copy(text) => self.copy(&text),
            // 打开整页随整页一起做（「界面块」第 2 条）。
            Action::Open(_) => {}
        }
    }
}
