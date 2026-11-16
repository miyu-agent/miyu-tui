//! 账户余额那几行怎么写（「配置与模型」第 5 条，核心 QU-2，2026-11-16 项目主人定的样子）。

use ratatui::text::Line;

use super::lines;
use crate::app::quota::{Balance, BalanceItem};
use crate::config::Config;

fn item(currency: &str, total: &str, granted: &str) -> BalanceItem {
    BalanceItem {
        currency: currency.into(),
        total: total.into(),
        granted: granted.into(),
    }
}

fn balance(available: bool, items: Vec<BalanceItem>) -> Balance {
    Balance { available, items }
}

fn draw(config: &Config, balance: Option<&Balance>) -> Vec<Line<'static>> {
    lines(&config.text.usage, balance, &config.layout.currencies)
}

fn text(lines: &[Line]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn one_cny_item_with_granted_writes_the_dim_granted_part() {
    let config = Config::builtin().unwrap();
    let b = balance(true, vec![item("CNY", "12.34", "2.00")]);
    let got = draw(&config, Some(&b));
    assert_eq!(text(&got), ["余额 ¥12.34  含赠送 ¥2.00"], "主人给的样子");
    assert_eq!(got[0].spans[0].style.fg, None, "能用的照平常的字");
    assert_eq!(got[0].spans[1].style, crate::theme::dim(), "赠送那一截暗色");
}

#[test]
fn granted_zero_has_no_dim_part() {
    let config = Config::builtin().unwrap();
    let b = balance(true, vec![item("CNY", "10.34", "0.00")]);
    let got = draw(&config, Some(&b));
    assert_eq!(text(&got), ["余额 ¥10.34"]);
    assert_eq!(got[0].spans.len(), 1);
}

#[test]
fn usd_uses_its_sign_and_other_currencies_write_the_code_first() {
    let config = Config::builtin().unwrap();
    let b = balance(
        true,
        vec![item("USD", "5.00", "1.50"), item("EUR", "3.00", "0")],
    );
    assert_eq!(
        text(&draw(&config, Some(&b))),
        ["余额 $5.00  含赠送 $1.50", "余额 EUR 3.00"],
        "一种币一行，照核心的先后；人民币、美元以外写代码"
    );
}

#[test]
fn not_available_turns_the_whole_line_red() {
    let config = Config::builtin().unwrap();
    let b = balance(false, vec![item("CNY", "0.00", "2.00")]);
    let got = draw(&config, Some(&b));
    assert_eq!(text(&got), ["余额 ¥0.00  含赠送 ¥2.00"]);
    let red = crate::theme::error().fg;
    assert!(red.is_some());
    assert!(
        got[0].spans.iter().all(|s| s.style.fg == red),
        "整行和额度满了同一个色"
    );
}

#[test]
fn empty_items_write_exhausted_only_when_not_available() {
    let config = Config::builtin().unwrap();
    let out = draw(&config, Some(&balance(false, Vec::new())));
    assert_eq!(text(&out), ["余额不足"]);
    assert_eq!(out[0].spans[0].style.fg, crate::theme::error().fg);
    assert!(
        draw(&config, Some(&balance(true, Vec::new()))).is_empty(),
        "一项都没有、能用的不画"
    );
}

#[test]
fn no_balance_draws_nothing() {
    let config = Config::builtin().unwrap();
    assert!(draw(&config, None).is_empty());
}

#[test]
fn every_language_has_the_balance_words() {
    for lang in ["en", "ja"] {
        let table = crate::language::LanguageTable::builtin().unwrap();
        let config = Config::load(&table.find(lang).unwrap(), false).unwrap();
        let b = balance(true, vec![item("CNY", "12.34", "2.00")]);
        let got = text(&draw(&config, Some(&b)));
        let want = if lang == "en" {
            "Balance ¥12.34  incl. ¥2.00 granted"
        } else {
            "残高 ¥12.34  うち付与 ¥2.00"
        };
        assert_eq!(got, [want], "{lang}");
    }
}
