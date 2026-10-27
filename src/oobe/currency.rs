//! 金额用什么货币显示（蓝图 `tui.md`「第一次打开的引导」第 19a 条，2026-10-26 项目主人定：放在接模型后面）：一种货币一行，
//! 选了写个人设置的 `usage.currency`；模型的金额照近期汇率换成这一种显示（核心 UC-1）。光标停在设过的那一种，没人设过的
//! 照界面语言的那一种（字表 `oobe.currency.default`：中文人民币、日文日元、英文美元）。列常见的一二十种，最后一行「其他…」
//! 自己填三个字母的代码（2026-10-27 项目主人：「列更多常见的，然后加其他」；核心 `usage.currency` 是三个字的文字）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::Move;
use super::asker::{Asker, Waiting};
use super::field::{Edit, Field};
use super::pick::Pick;
use crate::core::Refusal;

/// 列出来的几种，照先后（ISO 4217 的三个大写字母）。
pub const CURRENCIES: [&str; 20] = [
    "CNY", "USD", "EUR", "JPY", "HKD", "GBP", "TWD", "KRW", "SGD", "AUD", "CAD", "CHF", "NZD",
    "INR", "THB", "MYR", "IDR", "VND", "PHP", "RUB",
];

/// 货币那一步。
#[derive(Debug)]
pub struct CurrencyStep {
    /// 一种一行，照 [`CURRENCIES`]，最后多一行「其他…」。
    pub pick: Pick,
    /// 「其他…」填的代码。
    pub other: Field,
    /// 填的不是三个字母：红字说一句。
    pub invalid: bool,
}

impl Default for CurrencyStep {
    fn default() -> Self {
        let mut rows: Vec<String> = CURRENCIES.iter().map(|c| (*c).to_string()).collect();
        rows.push(String::new());
        Self {
            pick: Pick::new(rows, 0),
            other: Field::line(),
            invalid: false,
        }
    }
}

/// 填的代码：去掉空白、转大写，是三个英文字母的才收。
pub fn code(typed: &str) -> Option<String> {
    let code = typed.trim().to_ascii_uppercase();
    (code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())).then_some(code)
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
        match CURRENCIES.iter().position(|c| *c == code) {
            Some(at) => self.pick.selected = at,
            // 设过列表里没有的：停在「其他…」，格子里是它。
            None => {
                self.pick.selected = self.other_row();
                self.other.editor.set(code);
            }
        }
    }

    /// 「其他…」那一行是第几行。
    pub fn other_row(&self) -> usize {
        CURRENCIES.len()
    }

    /// 写个人设置，往下走。
    fn save(code: &str, asker: &mut Asker) -> Move {
        let params =
            json!({"layer": "personal", "changes": [{"key": "usage.currency", "value": code}]});
        asker.ask("config.set", params, Waiting::CurrencySave);
        Move::Next
    }

    /// 按了一个键：上下挪，`Enter` 写下选的、往下走（「其他…」上是开始填），`Esc` 回上一步。填代码时按键进格子：
    /// `Enter` 是三个字母的就写下往下走，不是的红字留着改；`Esc` 不填了。
    pub fn key(&mut self, key: KeyEvent, asker: &mut Asker) -> Move {
        if self.other.editing() {
            match self.other.edit(key) {
                Edit::Done => match code(self.other.text()) {
                    Some(code) => return Self::save(&code, asker),
                    None => {
                        self.invalid = true;
                        self.other.begin();
                    }
                },
                Edit::Cancelled => self.invalid = false,
                Edit::Stay => {}
            }
            return Move::Stay;
        }
        if self.pick.key(key) {
            return Move::Stay;
        }
        match key.code {
            KeyCode::Esc => Move::Back,
            KeyCode::Enter if self.pick.selected >= self.other_row() => {
                self.other.begin();
                Move::Stay
            }
            KeyCode::Enter => Self::save(CURRENCIES[self.pick.selected], asker),
            _ => Move::Stay,
        }
    }
}

#[cfg(test)]
mod tests;
