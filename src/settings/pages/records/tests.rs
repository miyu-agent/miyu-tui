use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::{PLAIN, label, record_path, summary};
use crate::config::Config;
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

/// 生图包的账号：名字（当标题）、平台（下拉）、Key（密钥）、每日上限（滑块 0～50，一步 5，默认 10）、开着（开关）。
fn schema() -> Value {
    json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "draw", "name": "画图", "page": "packages"}],
        "items": [{"key": "draw.accounts", "type": "list", "element": "record", "control": "list",
            "layers": ["system"], "name": "账号", "page": "packages", "group": "draw", "package": "draw",
            "title": "name",
            "fields": [
                {"key": "name", "type": "text", "control": "text", "name": "名字"},
                {"key": "platform", "type": "option", "control": "select", "name": "平台", "default": "a",
                 "options": [{"value": "a", "name": "绘世"}, {"value": "b", "name": "别家"}]},
                {"key": "key", "type": "secret", "control": "text", "name": "Key"},
                {"key": "limit", "type": "int", "control": "slider", "min": 0, "max": 50, "step": 5,
                 "default": 10, "name": "每日上限"},
                {"key": "on", "type": "bool", "control": "toggle", "default": true, "name": "开着"},
                {"key": "debug", "type": "bool", "control": "toggle", "name": "调试", "hidden": true}]}]})
}

fn opened(value: Value) -> Settings {
    let layers = json!([{"origin": {"layer": "system", "line": 1}, "value": value, "used": true}]);
    let config = json!({"items": {"draw.accounts": {"value": value, "layers": layers}}});
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "config.schema" => schema(),
            "config.get" => config.clone(),
            _ => json!({}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page.change("packages/draw", 0, &texts());
    page
}

fn press(page: &mut Settings, code: KeyCode) {
    page.key(KeyEvent::new(code, KeyModifiers::NONE), &texts());
}

fn typed(page: &mut Settings, text: &str) {
    for c in text.chars() {
        press(page, KeyCode::Char(c));
    }
    press(page, KeyCode::Enter);
}

fn one() -> Value {
    json!([{"name": "主号", "platform": "a", "key": {"secret": "old-key"}, "limit": 20, "on": true}])
}

#[test]
fn titles_come_from_the_title_field_or_the_position() {
    let page = opened(one());
    let item = page.more.schema.as_ref().unwrap().items[0].clone();
    let words = &texts().more.records;
    assert_eq!(item.fields.len(), 5, "hidden 的字段不画");
    assert_eq!(label(&item, &json!({"name": "主号"}), 1, words), "主号");
    assert_eq!(label(&item, &json!({"name": ""}), 2, words), "第 2 项");
    let two = json!([{"name": "主号"}, {}]);
    assert_eq!(
        summary(&item, &two, words, "、").as_deref(),
        Some("主号、第 2 项")
    );
    assert_eq!(
        summary(&item, &json!([]), words, "、").as_deref(),
        Some("—")
    );
    assert_eq!(
        record_path("draw.accounts[3].key"),
        Some(("draw.accounts".into(), 3, "key".into()))
    );
    assert_eq!(record_path("providers.dev.key"), None);
}

#[test]
fn a_new_record_is_filled_in_and_its_secret_is_stored_before_the_reference() {
    let mut page = opened(one());
    assert!(
        matches!(page.popup, Some(Popup::Records(_))),
        "Enter 开列表窗"
    );
    press(&mut page, KeyCode::Char('a'));
    let Some(Popup::Record(form)) = &page.popup else {
        panic!("a 开表单窗：{:?}", page.popup);
    };
    assert_eq!(form.record.get("limit"), Some(&json!(10)), "照默认填");
    press(&mut page, KeyCode::Enter);
    typed(&mut page, "备用");
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Line(line)) = &page.popup else {
        panic!("密钥开编辑窗");
    };
    assert!(matches!(
        line.target,
        crate::settings::pages::target::Target::RecordField(_, _, true)
    ));
    typed(&mut page, "sk-plain");
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Right);
    let Some(Popup::Record(form)) = &page.popup else {
        panic!("回到表单窗");
    };
    assert_eq!(form.record.get("name"), Some(&json!("备用")));
    assert_eq!(form.record.get("key"), Some(&json!({PLAIN: "sk-plain"})));
    assert_eq!(form.record.get("limit"), Some(&json!(15)), "滑块在行上调");
    press(&mut page, KeyCode::Esc);
    let Some(Popup::Records(list)) = &page.popup else {
        panic!("回到列表窗");
    };
    assert_eq!(list.records.len(), 2);
    page.take_asks();
    press(&mut page, KeyCode::Esc);
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1, "先存密钥：{asks:?}");
    let (tag, method, params) = &asks[0];
    assert_eq!(*method, "secret.set");
    // 明文只在放密钥的那一处，草稿里要存的那份值不带它（存不成时扔掉的也不带）。
    let drafted = page
        .draft
        .value("draw.accounts", page.data.as_ref().unwrap())
        .cloned()
        .unwrap_or_default();
    assert!(!drafted.to_string().contains("sk-plain"), "{drafted}");
    assert_eq!(params["value"], "sk-plain");
    let name = params["name"].as_str().unwrap().to_string();
    page.answer(*tag, Ok(json!({})), &texts());
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("再写配置");
    let value = &set.2["changes"][0]["value"];
    assert_eq!(
        value[0]["key"],
        json!({"secret": "old-key"}),
        "原来的引用照旧"
    );
    assert_eq!(value[1]["key"], json!({"secret": name}), "新的写成引用");
    assert!(!set.2.to_string().contains("sk-plain"), "配置里没有明文");
    assert!(!set.2.to_string().contains(PLAIN), "记号不出终端");
}

#[test]
fn an_untouched_new_record_is_not_added_and_nothing_is_saved() {
    let mut page = opened(one());
    press(&mut page, KeyCode::Char('a'));
    press(&mut page, KeyCode::Esc);
    let Some(Popup::Records(list)) = &page.popup else {
        panic!("回到列表窗");
    };
    assert_eq!(list.records.len(), 1, "一个字段都没填的不加");
    page.take_asks();
    press(&mut page, KeyCode::Esc);
    assert!(page.take_asks().is_empty(), "没动过不存");
    assert!(page.popup.is_none());
}

#[test]
fn deleting_a_record_saves_the_rest_as_is() {
    let mut page = opened(json!([{"name": "主号"}, {"name": "备用"}]));
    press(&mut page, KeyCode::Char('d'));
    page.take_asks();
    press(&mut page, KeyCode::Esc);
    let set = page
        .take_asks()
        .into_iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("动过的存");
    assert_eq!(set.2["changes"][0]["value"], json!([{"name": "备用"}]));
}
