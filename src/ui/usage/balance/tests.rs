//! 账户余额那一行怎么写（「配置与模型」第 5 条，核心 QU-2；2026-11-17 验收第 4 条项目主人定照 DeepSeek 官网写成一行）。

use ratatui::text::Line;

use super::lines;
use crate::app::quota::{Balance, BalanceItem};
use crate::config::Config;

fn item(currency: &str, topped_up: &str, granted: &str) -> BalanceItem {
    BalanceItem {
        currency: currency.into(),
        topped_up: topped_up.into(),
        granted: granted.into(),
    }
}

fn balance(available: bool, items: Vec<BalanceItem>) -> Balance {
    Balance { available, items }
}

/// 照供应商「DeepSeek」画（名字照配置、目录，2026-11-17 验收第 36 条）。
fn draw(config: &Config, balance: Option<&Balance>) -> Vec<Line<'static>> {
    lines(
        &config.text.usage,
        ("DeepSeek", balance),
        &config.layout.currencies,
    )
}

fn text(lines: &[Line]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn every_currency_tops_up_first_then_the_granted_part_in_one_line() {
    // 官网：充值余额「-$0.01 USD + ¥3.12 CNY」，有赠送的再接赠送。
    let config = Config::builtin().unwrap();
    let b = balance(
        true,
        vec![item("USD", "-0.01", "0.00"), item("CNY", "3.12", "2.00")],
    );
    let got = draw(&config, Some(&b));
    assert_eq!(
        text(&got),
        ["DeepSeek 余额：-$0.01 + ¥3.12 + 赠送 ¥2.00"],
        "主人给的样子"
    );
    assert_eq!(got[0].spans[0].style, crate::theme::dim(), "开头的字暗色");
    assert_eq!(got[0].spans[1].style.fg, None, "金额照平常的字");
}

#[test]
fn no_granted_part_when_nothing_is_granted() {
    let config = Config::builtin().unwrap();
    let b = balance(true, vec![item("CNY", "10.34", "0.00")]);
    assert_eq!(text(&draw(&config, Some(&b))), ["DeepSeek 余额：¥10.34"]);
}

#[test]
fn other_currencies_write_the_code_first() {
    let config = Config::builtin().unwrap();
    let b = balance(
        true,
        vec![item("USD", "5.00", "1.50"), item("EUR", "3.00", "0")],
    );
    assert_eq!(
        text(&draw(&config, Some(&b))),
        ["DeepSeek 余额：$5.00 + EUR 3.00 + 赠送 $1.50"],
        "照核心的先后；人民币、美元以外写代码"
    );
}

#[test]
fn not_available_turns_the_whole_line_red() {
    let config = Config::builtin().unwrap();
    let b = balance(false, vec![item("CNY", "0.00", "2.00")]);
    let got = draw(&config, Some(&b));
    assert_eq!(text(&got), ["DeepSeek 余额：¥0.00 + 赠送 ¥2.00"]);
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
    assert_eq!(text(&out), ["DeepSeek 余额不足"]);
    assert_eq!(out[0].spans[0].style.fg, crate::theme::error().fg);
    assert!(
        draw(&config, Some(&balance(true, Vec::new()))).is_empty(),
        "一项都没有、能用的不画"
    );
}

#[test]
fn the_head_names_the_provider_it_came_from() {
    let config = Config::builtin().unwrap();
    let b = balance(true, vec![item("CNY", "1.00", "0")]);
    let named = |provider| {
        text(&lines(
            &config.text.usage,
            (provider, Some(&b)),
            &config.layout.currencies,
        ))
    };
    assert_eq!(named("官方 DS"), ["官方 DS 余额：¥1.00"], "配置里起的名字");
    assert_eq!(named(""), ["余额：¥1.00"], "不知道是哪家的不留空格");
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
        let b = balance(
            true,
            vec![item("USD", "1.00", "0"), item("CNY", "10.34", "2.00")],
        );
        let got = text(&draw(&config, Some(&b)));
        let want = if lang == "en" {
            "DeepSeek balance: $1.00 + ¥10.34 + granted ¥2.00"
        } else {
            "DeepSeek 残高：$1.00 + ¥10.34 + 付与 ¥2.00"
        };
        assert_eq!(got, [want], "{lang}");
    }
}
