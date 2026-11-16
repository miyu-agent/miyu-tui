//! 总览顶上的账户余额（核心 QU-2，蓝图「配置与模型」第 5 条，2026-11-16 项目主人定样子）：用 DeepSeek 官方供应商时，
//! 套餐额度那几行的地方一种币一行「余额 ¥12.34」，有赠送的后面暗字「含赠送 ¥2.00」；官方说不能用的整行照 `error`
//! （和额度条满了同一个色）；不画条。一项都没有又不能用的写一行「余额不足」，一项都没有能用的、没查到的不画。
//! 金额照官方的原样字符串；人民币、美元照 `layout.currencies` 的记号写在前面，别的币写代码加空格（`EUR 3.00`）。

use std::collections::HashMap;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::Texts;
use crate::app::quota::Balance;
use crate::theme;

/// 写记号的币种（项目主人定：人民币、美元），别的写代码。
const SIGNED: [&str; 2] = ["CNY", "USD"];

/// 余额那几行；没查到、一项都没有又能用的是空的。
pub(super) fn lines(
    texts: &Texts,
    balance: Option<&Balance>,
    symbols: &HashMap<String, String>,
) -> Vec<Line<'static>> {
    let Some(balance) = balance else {
        return Vec::new();
    };
    let (main, faint) = if balance.available {
        (Style::new(), theme::dim())
    } else {
        (theme::error(), theme::error())
    };
    if balance.items.is_empty() {
        return if balance.available {
            Vec::new()
        } else {
            vec![Line::from(Span::styled(
                texts.balance_exhausted.clone(),
                main,
            ))]
        };
    }
    balance
        .items
        .iter()
        .map(|item| {
            let amount = |value: &str| money(&item.currency, value, symbols);
            let mut spans = vec![Span::styled(
                texts.balance.replace("{amount}", &amount(&item.total)),
                main,
            )];
            if item.granted.trim().parse::<f64>().is_ok_and(|g| g > 0.0) {
                let granted = texts
                    .balance_granted
                    .replace("{amount}", &amount(&item.granted));
                spans.push(Span::styled(format!("  {granted}"), faint));
            }
            Line::from(spans)
        })
        .collect()
}

/// 一笔：人民币、美元照主题的记号写在前面（`¥12.34`），别的写代码加空格（`EUR 3.00`）。
fn money(currency: &str, value: &str, symbols: &HashMap<String, String>) -> String {
    match symbols.get(currency) {
        Some(symbol) if SIGNED.contains(&currency) => format!("{symbol}{value}"),
        _ => format!("{currency} {value}"),
    }
}

#[cfg(test)]
mod tests;
