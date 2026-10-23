//! `/usage`（同终端界面「配置与模型」第 5 条，排法用它的 `ui::usage::page_lines`）：提示符下面原地画用量框，`Tab` `←` `→`
//! 换页、`↑` `↓` 滚、`t` 让热度图在 token 和花费之间切；`Esc` 收掉，正看着的那一页留在滚动区（只看的东西印下来，
//! 2026-10-20 项目主人：「实在不行就在终端打印内容呗」）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

use super::{Picked, Picker};
use crate::app::UsageData;
use crate::config::Config;
use crate::core::{Command, Core, SessionInfo, Update, UsageAsk, UsageKind};
use crate::shellhook::say::turn::Turn;
use crate::ui::Chrome;
use crate::ui::usage::{Sources, page_lines};

/// 用量框有几页（同终端界面）。
const PAGES: usize = 4;

/// `/usage`。
pub struct UsageShow {
    data: UsageData,
    /// 第几页、滚到哪、热度图写花费。
    tab: usize,
    scroll: usize,
    money: bool,
    /// 照哪种币写（`HeadConfig`）。
    currency: String,
    /// 知道的会话：按会话那一页写标题。
    sessions: Vec<SessionInfo>,
    /// 按了 `Esc`：下一拍照这一页印下来。
    leaving: bool,
}

impl UsageShow {
    pub fn new() -> Self {
        Self {
            data: UsageData {
                total: None,
                days: None,
                models: None,
                sessions: None,
                purposes: None,
                today: jiff::Zoned::now().date(),
            },
            tab: 0,
            scroll: 0,
            money: false,
            currency: "USD".to_string(),
            sessions: Vec::new(),
            leaving: false,
        }
    }

    fn page(&self, config: &Config, width: u16, max: usize) -> (Chrome, Vec<Line<'static>>) {
        let src = Sources {
            config,
            currency: &self.currency,
            sessions: &self.sessions,
        };
        page_lines(
            &src,
            &self.data,
            (self.tab, self.scroll, self.money),
            width,
            max,
        )
    }
}

impl Picker for UsageShow {
    fn start(&mut self, core: &Core) {
        let now = jiff::Zoned::now();
        let offset = crate::app::usage::offset_text(now.offset().seconds());
        for kind in [
            UsageKind::Total,
            UsageKind::Days,
            UsageKind::Models,
            UsageKind::Sessions,
            UsageKind::Purposes,
        ] {
            // 分天照本机此刻的时区；全部时间，不写起点（同终端界面）。
            let offset = (kind == UsageKind::Days).then(|| offset.clone());
            core.send(Command::Usage(UsageAsk {
                kind,
                from: None,
                offset,
            }));
        }
        core.send(Command::ListSessions);
    }

    fn core(&mut self, update: &Update, _turn: &Turn) {
        match update {
            Update::UsageRows { kind, rows } => {
                let slot = match kind {
                    UsageKind::Total => &mut self.data.total,
                    UsageKind::Days => &mut self.data.days,
                    UsageKind::Models => &mut self.data.models,
                    UsageKind::Sessions => &mut self.data.sessions,
                    UsageKind::Purposes => &mut self.data.purposes,
                };
                *slot = Some(rows.clone());
            }
            Update::HeadConfig(head) => self.currency = head.currency.clone(),
            Update::Sessions(all) => self.sessions = all.clone(),
            _ => {}
        }
    }

    fn ready(&mut self, _turn: &mut Turn, _core: &Core, config: &Config) -> Picked {
        if !self.leaving {
            return Picked::Stay;
        }
        let width = super::super::session::size().0.saturating_sub(4);
        let (chrome, mut lines) = self.page(config, width, usize::MAX);
        // 框里换页不变高，矮的页补了空行（同终端界面）；印下来用不着。
        while lines.last().is_some_and(|l| l.width() == 0) {
            lines.pop();
        }
        Picked::Show(
            Chrome {
                hint: None,
                ..chrome
            },
            lines,
        )
    }

    fn key(&mut self, key: KeyEvent, _turn: &mut Turn, _core: &Core, _config: &Config) -> Picked {
        match key.code {
            KeyCode::Tab | KeyCode::Right => {
                self.tab = (self.tab + 1) % PAGES;
                self.scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = (self.tab + PAGES - 1) % PAGES;
                self.scroll = 0;
            }
            KeyCode::Char('t') => self.money = !self.money,
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down => self.scroll += 1,
            KeyCode::Esc => self.leaving = true,
            _ => {}
        }
        Picked::Stay
    }

    fn view(
        &self,
        _turn: &Turn,
        config: &Config,
        width: u16,
        max: usize,
    ) -> (Chrome, Vec<Line<'static>>) {
        self.page(config, width, max)
    }
}
