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

/// 默认模型页、光标在第 `at` 项上的样子。
fn use_at(config: &Config, schema: &Value, at: isize) -> String {
    let mut page = page_with(config, schema);
    page.on_menu = false;
    page.nav.page_step(true);
    page.nav.vertical(at, &page.view);
    screen(&mut page, config, 120, 30).0
}

/// 默认模型页、光标在第二项（多模态）上的样子。
fn second_use(config: &Config, schema: &Value) -> String {
    use_at(config, schema, 1)
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
        let config = *Config::load(&table.find(lang).unwrap(), false).unwrap();
        config.text.usage.purposes["vision"].clone()
    };
    assert_eq!(words("zh"), "多模态");
    assert_eq!(words("en"), "multimodal");
    assert_eq!(words("ja"), "マルチモーダル");
}

/// ACC-1121-#3（核心那边传话：对话那一行也照核心的字表，网页同）：默认文本模型那一行的名字、用途一句照 `models.chat` 的
/// `name`、`description`；老核心没有这一项的照终端的字。
#[test]
fn the_chat_row_takes_its_name_and_note_from_the_cores_schema() {
    let config = Config::builtin().unwrap();
    let schema = json!({"pages": [{"id": "models", "name": "模型"}], "groups": [{"id": "uses", "name": "用途", "page": "models"}],
        "items": [{"key": "models.chat", "type": "reference", "control": "text", "layers": ["system"],
                   "name": "主对话的模型", "description": "新会话默认用的模型。", "page": "models", "group": "uses"}]});
    let text = use_at(&config, &schema, 0);
    assert!(
        text.contains("主对话的模型") && text.contains("新会话默认用的模型。"),
        "{text}"
    );
    assert!(
        !text.contains("默认文本模型") && !text.contains("回合未指定模型时使用"),
        "{text}"
    );
    let old = use_at(&config, &json!({"secrets": []}), 0);
    assert!(
        old.contains("默认文本模型") && old.contains("回合未指定模型时使用"),
        "{old}"
    );
}

/// 多模态、语义模型两行的用途一句也照核心清单里那一项的 `description`（ACC-1121-#3，2026-11-21：网页同，主人要两个头都照
/// 核心的字表）；老核心没有的照终端的字（上面两条）。
#[test]
fn every_rows_note_comes_from_the_cores_schema() {
    let config = Config::builtin().unwrap();
    let item = |key: &str, kind: &str, name: &str, said: &str| {
        json!({"key": key, "type": kind, "control": "text", "layers": ["system"], "name": name,
               "description": said, "page": "models", "group": "uses"})
    };
    let schema = json!({"pages": [{"id": "models", "name": "模型"}], "groups": [{"id": "uses", "name": "用途", "page": "models"}],
        "items": [item("models.vision", "model", "多模态模型", "核心写的多模态说明。"),
                  item("models.embedding", "model_or", "语义模型", "核心写的语义模型说明。")]});
    let vision = use_at(&config, &schema, 1);
    assert!(vision.contains("核心写的多模态说明。"), "{vision}");
    assert!(
        !vision.contains("在对话模型不支持多模态时提供多模态能力"),
        "{vision}"
    );
    let embedding = use_at(&config, &schema, 2);
    assert!(embedding.contains("核心写的语义模型说明。"), "{embedding}");
}

/// 用途一句长的折行写全，不截掉（ACC-1121-#3：核心的说明比终端原来的长）。
#[test]
fn a_long_note_wraps_in_the_detail_column() {
    let config = Config::builtin().unwrap();
    let said = "新会话默认用的模型，写成供应商斜杠模型，也能写成一个模型池的名字，前面加一个记号。这一句后面还有尾巴。";
    let schema = json!({"pages": [{"id": "models", "name": "模型"}], "groups": [{"id": "uses", "name": "用途", "page": "models"}],
        "items": [{"key": "models.chat", "type": "reference", "control": "text", "layers": ["system"],
                   "name": "主对话的模型", "description": said, "page": "models", "group": "uses"}]});
    let text = use_at(&config, &schema, 0);
    assert!(text.contains("新会话默认用的模型"), "{text}");
    assert!(
        text.contains("这一句后面还有尾巴。"),
        "折行写全，最后几个字也在：{text}"
    );
}
