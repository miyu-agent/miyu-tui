//! 金额用什么货币显示（蓝图 `tui.md`「第一次打开的引导」第 19a 条，2026-10-26 项目主人定：放在接模型后面）：一种货币一行，
//! 选了写个人设置的 `usage.currency`；模型的金额照近期汇率换成这一种显示（核心 UC-1）。光标停在设过的那一种，没人设过的
//! 照界面语言的那一种（字表 `oobe.currency.default`：中文人民币、日文日元、英文美元）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::pick::Pick;
use crate::core::Refusal;

/// 列出来的几种，照先后（ISO 4217 的三个大写字母）。
pub const CURRENCIES: [&str; 8] = ["CNY", "USD", "EUR", "JPY", "HKD", "GBP", "TWD", "KRW"];

/// 货币那一步。
#[derive(Debug)]
pub struct CurrencyStep {
    /// 一种一行，照 [`CURRENCIES`]。
    pub pick: Pick,
}

impl Default for CurrencyStep {
    fn default() -> Self {
        Self {
            pick: Pick::new(CURRENCIES.iter().map(|c| (*c).to_string()).collect(), 0),
        }
    }
}

impl CurrencyStep {
    /// 进了这一步：读现在设的是哪一种。
    pub fn enter(&mut self, asker: &mut Asker) {
        asker.ask(
            "config.get",
            json!({"keys": ["usage.currency"]}),
            Waiting::CurrencyRead,
        );
    }

    /// 读回来了：人设过的停在那一种，没人设过的（出厂值）停在界面语言的那一种（`default`）。
    pub fn answer(&mut self, result: &Result<Value, Refusal>, default: &str) {
        let item = result.as_ref().map(|got| &got["items"]["usage.currency"]);
        let set = item
            .ok()
            .filter(|item| {
                item["origin"]["layer"]
                    .as_str()
                    .is_some_and(|l| l != "default")
            })
            .and_then(|item| item["value"].as_str());
        let code = set.unwrap_or(default);
        if let Some(at) = CURRENCIES.iter().position(|c| *c == code) {
            self.pick.selected = at;
        }
    }

    /// 按了一个键：上下挪，`Enter` 写下选的、往下走，`Esc` 回上一步。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
        if self.pick.key(key) {
            return Move::Stay;
        }
        match key.code {
            KeyCode::Esc => Move::Back,
            KeyCode::Enter => {
                let code = CURRENCIES[self.pick.selected.min(CURRENCIES.len() - 1)];
                let params = json!({"layer": "personal", "changes": [{"key": "usage.currency", "value": code}]});
                asker.ask("config.set", params, Waiting::CurrencySave);
                Move::Next
            }
            _ => Move::Stay,
        }
    }
}

#[cfg(test)]
mod tests;
