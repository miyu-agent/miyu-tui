//! 总览顶上的账户余额（核心 QU-2，蓝图「配置与模型」第 5 条）：用 DeepSeek 官方供应商时，套餐额度那几行的地方写一行，
//! 照 DeepSeek 官网用量页「充值余额」的写法（2026-11-17 验收第 4 条项目主人定）：`DeepSeek 余额：$x + ¥y + 赠送 ¥z`。
//! 前面是每种币充值的（`topped_up`，照核心的先后），后面接每种币赠送的（`granted` 大于 0 才写）；开头那几个字暗色。
//! 官方说不能用的整行照 `error`（和额度条满了同一个色）；不画条。一项都没有又不能用的写「DeepSeek 余额不足」，一项都没有
//! 能用的、没查到的不画。金额照官方的原样字符串；人民币、美元照 `layout.currencies` 的记号写在前面（负的负号在记号前，
//! `-$0.01`），别的币写代码加空格（`EUR 3.00`）。

use std::collections::HashMap;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::Texts;
use crate::app::quota::Balance;
use crate::theme;

/// 写记号的币种（项目主人定：人民币、美元），别的写代码。
const SIGNED: [&str; 2] = ["CNY", "USD"];

/// 余额那一行；没查到、一项都没有又能用的是空的。
pub(super) fn lines(
    texts: &Texts,
    (provider, balance): (&str, Option<&Balance>),
    symbols: &HashMap<String, String>,
) -> Vec<Line<'static>> {
    let Some(balance) = balance else {
        return Vec::new();
    };
    let (head, main) = if balance.available {
        (theme::dim(), Style::new())
    } else {
        (theme::error(), theme::error())
    };
    if balance.items.is_empty() {
        return if balance.available {
            Vec::new()
        } else {
            vec![Line::from(Span::styled(
                super::named(&texts.balance_exhausted, provider),
                main,
            ))]
        };
    }
    let topped = balance
        .items
        .iter()
        .map(|item| money(&item.currency, &item.topped_up, symbols));
    let granted = balance
        .items
        .iter()
        .filter(|item| item.granted.trim().parse::<f64>().is_ok_and(|g| g > 0.0))
        .map(|item| {
            let amount = money(&item.currency, &item.granted, symbols);
            texts.balance_granted.replace("{amount}", &amount)
        });
    let amounts = topped.chain(granted).collect::<Vec<_>>().join(" + ");
    let template = super::named(&texts.balance, provider);
    let (before, after) = template
        .split_once("{amounts}")
        .unwrap_or((template.as_str(), ""));
    vec![Line::from(vec![
        Span::styled(before.to_string(), head),
        Span::styled(format!("{amounts}{after}"), main),
    ])]
}

/// 一笔：人民币、美元照主题的记号写在前面（`¥12.34`，负的 `-$0.01`），别的写代码加空格（`EUR 3.00`）。
fn money(currency: &str, value: &str, symbols: &HashMap<String, String>) -> String {
    let value = value.trim();
    match symbols.get(currency) {
        Some(symbol) if SIGNED.contains(&currency) => match value.strip_prefix('-') {
            Some(rest) => format!("-{symbol}{rest}"),
            None => format!("{symbol}{value}"),
        },
        _ => format!("{currency} {value}"),
    }
}

#[cfg(test)]
mod tests;
