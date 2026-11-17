//! `/config` 里选默认人格、默认预设的选择窗按 `e` 编辑（2026-11-17 项目主人验收第 2 条）：开那一行的详情窗，关了回选择窗、光标
//! 还在那一行；「无人格」那一行不开，说一句不能编辑。别的选择窗的 `e` 照旧不管。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::core::Persona;
use crate::settings::pages::choose::Choose;
use crate::settings::pages::schema::Schema;
use crate::settings::popup::Popup;
use crate::settings::{Data, Settings, Texts};

fn texts() -> Texts {
    crate::config::Config::builtin().unwrap().text.settings
}

fn persona(id: &str, name: &str) -> Persona {
    Persona {
        id: id.into(),
        name: Some(name.into()),
        ..Persona::default()
    }
}

/// 配置页读好了清单、人格、预设，开着 `key` 那一项的选择窗。
fn choosing(key: &str, texts: &Texts) -> Settings {
    let mut page = Settings::default();
    let item = |key: &str, name: &str, kind: &str| {
        json!({"key": key, "type": kind, "control": "select", "layers": ["system"], "name": name,
               "page": "general", "group": "new_session"})
    };
    page.more.schema = Some(Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}],
        "groups": [{"id": "new_session", "name": "新会话", "page": "general"}],
        "items": [item("persona.default", "默认人格", "text"), item("preset.default", "默认预设", "preset"),
                  item("ui.language", "界面语言", "text")]})));
    page.more.personas = Some(vec![persona("pirate", "海盗"), persona("cat", "猫")]);
    page.more.presets = Some(vec![persona("full", "全功能")]);
    page.data = Some(Data::default());
    page.online = true;
    let schema = page.more.schema.clone().unwrap();
    let item = schema.items.iter().find(|i| i.key == key).unwrap();
    let list = super::choices(item, page.more.named_item(item), &texts.more);
    page.popup = Some(Popup::Choose(Choose::open(
        key,
        &item.name,
        list,
        &Value::Null,
    )));
    page
}

fn press(page: &mut Settings, code: KeyCode, texts: &Texts) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), texts);
}

#[test]
fn e_in_the_default_persona_chooser_opens_the_persona_and_comes_back() {
    let texts = texts();
    let mut page = choosing("persona.default", &texts);
    // 第一行「无人格」：不开，说一句。
    press(&mut page, KeyCode::Char('e'), &texts);
    assert!(
        matches!(page.popup, Some(Popup::Choose(_))),
        "{:?}",
        page.popup
    );
    assert_eq!(
        page.status.as_ref().map(|s| s.0.as_str()),
        Some(texts.more.cant_edit.as_str())
    );
    // 挪到猫按 `e`：开猫的详情窗，去读它。
    press(&mut page, KeyCode::Down, &texts);
    press(&mut page, KeyCode::Down, &texts);
    page.take_asks();
    press(&mut page, KeyCode::Char('e'), &texts);
    let Some(Popup::Persona(view)) = &page.popup else {
        panic!("开详情窗：{:?}", page.popup);
    };
    assert_eq!(view.id, "cat");
    assert!(page.take_asks().iter().any(|a| a.1 == "persona.get"));
    // 关了回到选择窗，光标还在猫上，没选、没存。
    press(&mut page, KeyCode::Esc, &texts);
    let Some(Popup::Choose(choose)) = &page.popup else {
        panic!("回选择窗：{:?}", page.popup);
    };
    assert_eq!(choose.key, "persona.default");
    assert_eq!(choose.chosen(), Some(&json!("cat")));
    assert!(
        !page.take_asks().iter().any(|a| a.1 == "config.set"),
        "不算选"
    );
}

#[test]
fn e_in_the_default_preset_chooser_opens_the_preset() {
    let texts = texts();
    let mut page = choosing("preset.default", &texts);
    press(&mut page, KeyCode::Char('e'), &texts);
    let Some(Popup::Preset(view)) = &page.popup else {
        panic!("开预设的详情窗：{:?}", page.popup);
    };
    assert_eq!(view.id, "full");
    press(&mut page, KeyCode::Esc, &texts);
    assert!(
        matches!(page.popup, Some(Popup::Choose(_))),
        "{:?}",
        page.popup
    );
}
