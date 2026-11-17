//! 选项由包在运行时给的文字项（核心 SG-2 的 `choices_from`）：那一行照选项的名字写，不写存着的原值（语音会话 VO-32 报：「引擎
//! deepseek-tools」，下拉里叫「DeepSeek 功能全家桶」）；值是空的那个选项是「不设」时用的，没设时写它的名字；问不到的照原值写。

use serde_json::{Value, json};

use crate::config::Config;
use crate::settings::Settings;

fn schema() -> Value {
    json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "voice-input", "name": "语音输入", "page": "packages"}],
        "items": [{"key": "voice-input.engine", "control": "text", "type": "text", "layers": ["system"],
                   "name": "引擎", "page": "packages", "group": "voice-input", "package": "voice-input",
                   "choices_from": {"package": "voice-input", "method": "engines"}}]})
}

/// 开着配置页，系统配置里引擎写的是 `stored`；包答 `engines`（`None` 是问不到）。交回「引擎」那一行。
fn engine_row(stored: Option<&str>, engines: Option<Value>) -> String {
    let config = Config::builtin().unwrap();
    let texts = &config.text.settings;
    let mut page = Settings::open(false, true);
    let mut calls = Vec::new();
    for _ in 0..3 {
        for (tag, method, _) in page.take_asks() {
            let got = match method {
                "config.schema" => schema(),
                "config.get" => {
                    let mut items = json!({});
                    if let Some(stored) = stored {
                        let origin = json!({"layer": "system", "line": 1});
                        items["voice-input.engine"] = json!({"value": stored, "origin": origin,
                            "layers": [{"origin": origin, "value": stored, "used": true}]});
                    }
                    json!({ "items": items })
                }
                "package.list" => {
                    json!({"packages": [{"package": "voice-input", "name": "语音输入"}]})
                }
                "package.call" => {
                    calls.push(tag);
                    match &engines {
                        Some(got) => {
                            page.answer(tag, Ok(got.clone()), texts);
                            continue;
                        }
                        None => {
                            let refusal = crate::core::Refusal {
                                reason: Some("program_not_running".into()),
                                message: "方法执行失败。".into(),
                                data: json!({"reason": "program_not_running"}),
                            };
                            page.answer(tag, Err(refusal), texts);
                            continue;
                        }
                    }
                }
                _ => json!({}),
            };
            page.answer(tag, Ok(got), texts);
        }
    }
    assert_eq!(calls.len(), 1, "读到清单就问一次包");
    let lines = super::super::pages::page_lines(&page, "packages/voice-input", 0, texts, 60);
    lines
        .iter()
        .map(|(l, _)| l.to_string())
        .find(|l| l.contains("引擎"))
        .unwrap_or_else(|| panic!("没有「引擎」那一行：{lines:#?}"))
}

fn engines() -> Value {
    json!({"choices": [
        {"value": "", "name": "本机 SenseVoice"},
        {"value": "deepseek-tools", "name": "DeepSeek 功能全家桶"}]})
}

#[test]
fn a_package_choice_row_shows_the_choice_name() {
    let row = engine_row(Some("deepseek-tools"), Some(engines()));
    assert!(row.contains("DeepSeek 功能全家桶"), "{row}");
    assert!(!row.contains("deepseek-tools"), "{row}");
}

#[test]
fn an_unset_package_choice_shows_the_empty_valued_choice() {
    let row = engine_row(None, Some(engines()));
    assert!(row.contains("本机 SenseVoice"), "{row}");
}

#[test]
fn without_choices_the_row_shows_the_stored_value() {
    let row = engine_row(Some("deepseek-tools"), None);
    assert!(row.contains("deepseek-tools"), "{row}");
}
