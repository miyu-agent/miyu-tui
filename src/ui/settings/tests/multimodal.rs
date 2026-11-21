//! 默认模型里替对话模型看图、听音频的那一项叫「多模态」（2026-11-20 主人本机验收第 6 条，同网页、核心的字表）。名字照核心
//! `config.schema` 里 `models.vision` 的 `name`（ACC-1121-#3：两个头都照核心的字表，核心 FX-9 改名「多模态模型」）；旧核心
//! 没有这一项的照终端自己的字。

use serde_json::{Value, json};

use super::screen;
use crate::config::Config;
use crate::settings::Settings;

/// 配置页读好了模型、配置，`config.schema` 照 `schema` 答。
fn page_with(config: &Config, schema: &Value) -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => crate::settings::test_support::model_list(),
            "config.get" => crate::settings::test_support::config_all(),
            "config.schema" => schema.clone(),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    page
}

/// 默认模型页、光标在第二项（多模态）上的样子。
fn second_use(config: &Config, schema: &Value) -> String {
    let mut page = page_with(config, schema);
    page.on_menu = false;
    page.nav.page_step(true);
    page.nav.vertical(1, &page.view);
    screen(&mut page, config, 120, 30).0
}

/// 核心的清单：`models.vision` 叫 `name`。
fn schema_naming(name: &str) -> Value {
    json!({"pages": [{"id": "models", "name": "模型"}], "groups": [{"id": "uses", "name": "用途", "page": "models"}],
        "items": [{"key": "models.vision", "type": "model", "control": "text", "layers": ["system"],
                   "name": name, "page": "models", "group": "uses"}]})
}

#[test]
fn the_second_default_model_is_called_multimodal() {
    // 旧核心的清单里没有这一项：照终端自己的字。
    let config = Config::builtin().unwrap();
    let text = second_use(&config, &json!({"secrets": []}));
    assert!(text.contains("默认多模态模型"), "{text}");
    assert!(
        text.contains("在对话模型不支持多模态时提供多模态能力"),
        "{text}"
    );
    assert!(!text.contains("视觉"), "{text}");
}

#[test]
fn the_second_default_model_takes_its_name_from_the_cores_schema() {
    let config = Config::builtin().unwrap();
    let text = second_use(&config, &schema_naming("多模态模型"));
    assert!(text.contains("多模态模型"), "{text}");
    assert!(!text.contains("默认多模态模型"), "照核心的名字：{text}");
}

#[test]
fn usage_names_the_multimodal_purpose_the_same_way() {
    let table = crate::language::LanguageTable::builtin().unwrap();
    let words = |lang: &str| {
        let config = Config::load(&table.find(lang).unwrap(), false).unwrap();
        config.text.usage.purposes["vision"].clone()
    };
    assert_eq!(words("zh"), "多模态");
    assert_eq!(words("en"), "multimodal");
    assert_eq!(words("ja"), "マルチモーダル");
}
