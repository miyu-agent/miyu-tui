//! 状态行里模型的数目只属于「供应商和模型」那一页（蓝图 `tui.md`「配置页」第 4 条）：别的页（软件包自己的设置、软件包三页）
//! 底下不写「暂无模型」（2026-11-20 主人本机验收第 10 条：语音的设置页底下一直写着它，那是供应商页选中的那一家没有模型）。

use serde_json::{Value, json};

use super::screen;
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::pages::Section;

/// 第一家供应商没有模型的配置页，带着语音输入包的一页设置。
fn page(config: &Config) -> Settings {
    let mut page = Settings::open(false, true);
    let item = json!({"key": "voice-input.engine", "name": "识别引擎", "page": "packages",
        "group": "voice-input", "package": "voice-input", "control": "text", "kind": "string",
        "layers": ["system"]});
    for (tag, method, _) in page.take_asks() {
        let got: Value = match method {
            "model.list" => json!({"uses": {"chat": null, "vision": null}, "pools": [],
                "providers": [{"id": "acme", "name": {"value": "Acme", "from": "config"},
                    "driver": "openai-chat", "base_url": "https://acme.example.invalid/v1", "models": []}]}),
            "config.get" => json!({"items": {}}),
            "config.schema" => json!({"pages": [{"id": "packages", "name": "软件包"}],
                "groups": [{"id": "voice-input", "name": "语音输入", "page": "packages"}],
                "items": [item]}),
            "package.list" => json!({"packages": [{"package": "voice-input", "name": "语音输入"}]}),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page.on_menu = false;
    page
}

#[test]
fn the_providers_page_still_says_a_provider_has_no_models() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(text.contains("暂无模型"), "先证明那一家没有模型：\n{text}");
}

#[test]
fn a_package_settings_page_does_not_talk_about_models() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    page.more.section = Some(Section::Page("packages/voice-input".into(), 0));
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(text.contains("识别引擎"), "画的是语音输入那一页：\n{text}");
    assert!(!text.contains("暂无模型"), "{text}");
}

#[test]
fn the_package_store_does_not_talk_about_models() {
    let config = Config::builtin().unwrap();
    let mut page = page(&config);
    page.more.section = Some(Section::Store);
    let (text, _) = screen(&mut page, &config, 100, 30);
    assert!(!text.contains("暂无模型"), "{text}");
}
