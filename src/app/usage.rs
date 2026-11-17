//! `/usage` 框（蓝图 `tui.md`「配置与模型」第 5 条，2026-10-07 项目主人定照 Codex 做细，对比页选了 A）：开框时向核心
//! 要五次 `usage.query`（合计、全部时间按天、按模型、按会话、按用途），回来了交给框画；`Tab` `Shift+Tab`（`←` `→`、
//! `h` `l` 也行）换页，`↑` `↓` `PgUp` `PgDn` 滚，`t` 让热度图在 token 和花费之间切，`Esc` 关。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{App, Panel};
use crate::core::{Command, UsageAsk, UsageKind, UsageRow};

/// 读回来的一样：还没回来是 `None`，拒了的是核心的原话。
pub type Loaded = Option<Result<Vec<UsageRow>, String>>;

/// 框有几页：总览、最近 30 天、按模型、按会话。
const PAGES: usize = 4;

/// `/usage` 框要画的：五样读回来的，和打开时本机的今天。
#[derive(Debug)]
pub struct UsageData {
    /// 全部的合计。
    pub total: Loaded,
    /// 全部时间按天。
    pub days: Loaded,
    /// 按模型。
    pub models: Loaded,
    /// 按会话。
    pub sessions: Loaded,
    /// 按用途。
    pub purposes: Loaded,
    /// 打开时本机的今天。
    pub today: jiff::civil::Date,
    /// 用到的要署名的汇率源（核心 UC-1）：框底下注明。
    pub rates: Vec<String>,
    /// 看着的会话的套餐额度（核心 QU-1，`quota.rs`）：没有额度、还没回来的是空的，总览不画。
    pub quota: Vec<super::quota::QuotaWindow>,
    /// 看着的会话的账户余额（核心 QU-2，`quota.rs`）：没查到、还没回来的是 `None`，总览不画。
    pub balance: Option<super::quota::Balance>,
    /// 额度、余额是哪一家的：回应里 `provider` 的名字（核心 QU-3），没带的是 `None`。
    pub quota_provider: Option<String>,
}

impl App {
    /// 打开 `/usage`：要五样，没回来以前那一页写「正在读用量…」。
    pub(super) fn open_usage(&mut self) {
        let now = jiff::Zoned::now();
        let offset = offset_text(now.offset().seconds());
        self.boxes.usage = Some(UsageData {
            total: None,
            days: None,
            models: None,
            sessions: None,
            purposes: None,
            today: now.date(),
            rates: Vec::new(),
            quota: Vec::new(),
            balance: None,
            quota_provider: None,
        });
        self.ask_quota();
        let kinds = [
            UsageKind::Total,
            UsageKind::Days,
            UsageKind::Models,
            UsageKind::Sessions,
            UsageKind::Purposes,
        ];
        for kind in kinds {
            // 分天照本机此刻的时区；全部时间，不写起点。
            let offset = (kind == UsageKind::Days).then(|| offset.clone());
            self.link.core.send(Command::Usage(UsageAsk {
                kind,
                from: None,
                offset,
            }));
        }
        self.boxes.panel = Some(Panel::Usage {
            tab: 0,
            scroll: 0,
            money: false,
        });
    }

    /// 核心交回了一样：记下，框照它画；框关了的不要。
    pub(super) fn usage_rows(
        &mut self,
        kind: UsageKind,
        rows: Result<Vec<UsageRow>, String>,
        rates: Vec<String>,
    ) {
        let Some(data) = self.boxes.usage.as_mut() else {
            return;
        };
        for rate in rates {
            if !data.rates.contains(&rate) {
                data.rates.push(rate);
            }
        }
        let slot = match kind {
            UsageKind::Total => &mut data.total,
            UsageKind::Days => &mut data.days,
            UsageKind::Models => &mut data.models,
            UsageKind::Sessions => &mut data.sessions,
            UsageKind::Purposes => &mut data.purposes,
        };
        *slot = Some(rows);
    }

    /// 框开着时的按键。
    pub(super) fn usage_key(&mut self, (tab, scroll, money): (usize, usize, bool), key: KeyEvent) {
        let page = self.screen.panel_rows.len().max(1);
        let (tab, scroll, money) = match (page_turn(&key), key.code) {
            (Some(true), _) => ((tab + 1) % PAGES, 0, money),
            (Some(false), _) => ((tab + PAGES - 1) % PAGES, 0, money),
            (None, code) => match code {
                KeyCode::Char('t') => (tab, scroll, !money),
                KeyCode::Up => (tab, scroll.saturating_sub(1), money),
                KeyCode::Down => (tab, scroll + 1, money),
                KeyCode::PageUp => (tab, scroll.saturating_sub(page), money),
                KeyCode::PageDown => (tab, scroll + page, money),
                KeyCode::Esc => {
                    self.boxes.panel = None;
                    self.boxes.usage = None;
                    return;
                }
                _ => return,
            },
        };
        // 滚过头停在最后一屏：标签那两行、底下注明汇率源的两行不滚。
        let width = self.screen.areas.menu_text.width;
        let rows = crate::ui::usage::body_rows(&self.seen(), (tab, money), width);
        let rates = self
            .boxes
            .usage
            .as_ref()
            .is_some_and(|d| !d.rates.is_empty());
        let shown = page.saturating_sub(if rates { 4 } else { 2 }).max(1);
        let scroll = scroll.min(rows.saturating_sub(shown));
        self.boxes.panel = Some(Panel::Usage { tab, scroll, money });
    }
}

/// 换页的键：`Tab`、`→`、`l` 下一页（`true`），`Shift+Tab`、`←`、`h` 上一页（`false`）；别的是 `None`。终端界面和 shellhook
/// 的 `/usage` 都照它（2026-11-17 验收第 7 条：`h` `l` 同 `←` `→`）。
pub(crate) fn page_turn(key: &KeyEvent) -> Option<bool> {
    let plain = key.modifiers == KeyModifiers::NONE;
    match key.code {
        KeyCode::Tab | KeyCode::Right => Some(true),
        KeyCode::Char('l') if plain => Some(true),
        KeyCode::BackTab | KeyCode::Left => Some(false),
        KeyCode::Char('h') if plain => Some(false),
        _ => None,
    }
}

/// 时区写成核心认的样子：`+09:00`、`-05:30`。
pub(crate) fn offset_text(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

#[cfg(test)]
mod tests {
    use super::offset_text;

    #[test]
    fn offsets_are_written_the_way_the_core_reads_them() {
        assert_eq!(offset_text(9 * 3600), "+09:00");
        assert_eq!(offset_text(-(5 * 3600 + 30 * 60)), "-05:30");
        assert_eq!(offset_text(0), "+00:00");
    }
}
