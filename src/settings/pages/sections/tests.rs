//! 软件包的设置分组（「配置页」第 43d 条）：包照 `sections` 分块，条件不满足的组不画、值留着，一项不剩的组不画，旧核心照平铺。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::super::Section;
use super::super::schema::{Row, Schema};
use super::visible;
use crate::config::Config;
use crate::settings::Settings;
use crate::settings::data::Data;
use crate::settings::popup::{Delete, Popup};

/// 语音播报的包：引擎选 mimo 时出 mimo 那一组，选 minimax 时出 minimax 那一组；还有一组一项都没有，一项写了不认得的组。
fn say(sections: bool) -> Value {
    let item = |key: &str, name: &str, section: &str| {
        json!({"key": key, "control": "text", "layers": ["system"], "name": name, "page": "packages",
               "group": "say", "section": section})
    };
    let mut group = json!({"id": "say", "name": "语音播报", "page": "packages"});
    if sections {
        group["sections"] = json!([
            {"id": "mimo", "name": "MiMo", "shown_when": {"key": "say.engine", "is": ["mimo"]}},
            {"id": "minimax", "name": "MiniMax", "shown_when": {"key": "say.engine", "is": ["minimax"]}},
            {"id": "empty", "name": "空的"},
            {"id": "extra", "name": "附加", "shown_when": {"key": "say.nothing", "is": ["x"]}}
        ]);
    }
    let mut engine = item("say.engine", "引擎", "");
    engine["default"] = json!("mimo");
    json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [group],
        "items": [engine,
                  item("say.mimo.key", "MiMo 的 key", "mimo"),
                  item("say.minimax.key", "MiniMax 的 key", "minimax"),
                  item("say.speed", "语速", "nowhere"),
                  item("say.extra", "附加项", "extra")]})
}

/// 一页排成的样子：组名写成 `# 名字`，项写名字。
fn names(schema: &Schema, rows: &[Row]) -> Vec<String> {
    rows.iter()
        .map(|r| match r {
            Row::Group(g) => format!("# {}", schema.groups[*g].name),
            Row::Item(i) => schema.items[*i].name.clone(),
        })
        .collect()
}

/// 只写引擎这一项的系统配置。
fn data(engine: Option<&str>) -> Data {
    let mut data = Data::default();
    if let Some(engine) = engine {
        data.values
            .insert("say.engine".into(), (json!(engine), "system".into()));
        data.system.insert("say.engine".into(), json!(engine));
    }
    data
}

#[test]
fn sections_follow_the_package_group_and_hide_when_their_condition_fails() {
    let schema = Schema::read(&say(true));
    let ids: Vec<&str> = schema.groups.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(
        ids,
        ["say", "say#mimo", "say#minimax", "say#empty", "say#extra"],
        "组照 sections 的先后排在包那一组后面"
    );
    assert_eq!(schema.items[1].group, "say#mimo");
    assert_eq!(schema.items[1].package, "say", "挪进组的项记着是哪个包的");
    assert_eq!(schema.items[3].group, "say", "写了不认得的组的留在包那一组");
    let rows = schema.rows("packages");
    // 读来的值还没到：先都画；一项都没有的组不画。
    assert_eq!(
        names(&schema, &visible(rows.clone(), &schema, None)),
        [
            "# 语音播报",
            "引擎",
            "语速",
            "# MiMo",
            "MiMo 的 key",
            "# MiniMax",
            "MiniMax 的 key",
            "# 附加",
            "附加项"
        ]
    );
    // 系统配置没写引擎：照清单的默认 mimo，minimax 那一组不画；条件的键清单里找不到的照画。
    assert_eq!(
        names(&schema, &visible(rows.clone(), &schema, Some(&data(None)))),
        [
            "# 语音播报",
            "引擎",
            "语速",
            "# MiMo",
            "MiMo 的 key",
            "# 附加",
            "附加项"
        ]
    );
    // 换成 minimax：mimo 那一组藏起来，它的值照旧留着（这里不碰值）。
    assert_eq!(
        names(
            &schema,
            &visible(rows, &schema, Some(&data(Some("minimax"))))
        ),
        [
            "# 语音播报",
            "引擎",
            "语速",
            "# MiniMax",
            "MiniMax 的 key",
            "# 附加",
            "附加项"
        ]
    );
}

#[test]
fn a_section_is_changed_when_one_of_its_items_is_written_in_the_system_config() {
    let schema = Schema::read(&say(true));
    let mimo = schema
        .groups
        .iter()
        .position(|g| g.id == "say#mimo")
        .unwrap();
    let mut data = data(Some("mimo"));
    assert!(!schema.changed(mimo, &data), "引擎写了不算 mimo 那一组改过");
    data.system.insert("say.mimo.key".into(), json!("k"));
    assert!(schema.changed(mimo, &data));
    assert_eq!(schema.keys_of(mimo), ["say.mimo.key"]);
    assert_eq!(schema.section_of_item(1), Some(mimo));
    assert_eq!(schema.section_of_item(0), None, "包那一组不是分出来的组");
}

#[test]
fn an_old_core_without_sections_keeps_the_flat_list() {
    let schema = Schema::read(&say(false));
    assert!(schema.sections.is_empty());
    assert_eq!(
        names(
            &schema,
            &visible(
                schema.rows("packages"),
                &schema,
                Some(&data(Some("minimax")))
            )
        ),
        [
            "# 语音播报",
            "引擎",
            "MiMo 的 key",
            "MiniMax 的 key",
            "语速",
            "附加项"
        ]
    );
}

#[test]
fn shift_d_in_a_changed_section_asks_to_reset_every_written_item_of_it() {
    let config = Config::builtin().unwrap();
    let mut page = Settings::open(false, true);
    let written = |v: Value| {
        let layer = json!({"origin": {"layer": "system", "line": 1}, "value": v, "used": true});
        json!({"origin": {"layer": "system"}, "value": v, "layers": [layer]})
    };
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => crate::settings::test_support::model_list(),
            "config.get" => json!({"items": {
                "say.engine": written(json!("mimo")),
                "say.mimo.key": written(json!("k"))
            }}),
            "config.schema" => say(true),
            "package.list" => json!({"packages": [{"package": "say", "name": "语音播报"}]}),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    let id = "packages/say";
    // 引擎、语速，然后 MiMo 那一组、附加那一组；MiniMax 那一组藏着。
    let shown: Vec<String> = page
        .page_items(id)
        .iter()
        .map(|&i| page.more.schema.as_ref().unwrap().items[i].key.clone())
        .collect();
    assert_eq!(
        shown,
        ["say.engine", "say.speed", "say.mimo.key", "say.extra"],
    );
    page.on_menu = false;
    page.more.section = Some(Section::Page(id.into(), 2));
    page.key(
        KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT),
        &config.text.settings,
    );
    let Some(Popup::Confirm(confirm)) = &page.popup else {
        panic!("要先问：{:?}", page.popup.is_some());
    };
    assert!(confirm.title.contains("MiMo"), "{}", confirm.title);
    assert!(
        matches!(&confirm.ask, Delete::Settings(keys) if keys == &["say.mimo.key".to_string()])
    );
    // 停在没分组的项上按 `D`：不问。
    page.popup = None;
    page.more.section = Some(Section::Page(id.into(), 0));
    page.key(
        KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT),
        &config.text.settings,
    );
    assert!(page.popup.is_none());
}
