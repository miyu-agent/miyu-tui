//! 供应商页按 `a` 先弹目录（「配置页」第 11a 条，2026-10-26 项目主人：「a 新建时先弹目录」）。

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{opened, press, texts};
use crate::settings::Settings;
use crate::settings::pages::catalog::Row;
use crate::settings::popup::Popup;

/// 常用的几家：一家接不上的夹在中间，一家编号不合写法，一家和配置里的撞了。
fn featured() -> Value {
    json!({"providers": [
        {"id": "deepseek", "name": "DeepSeek", "supported": true},
        {"id": "dark", "name": "Dark", "supported": false},
        {"id": "wafer.ai", "name": "Wafer", "supported": true},
        {"id": "relay", "name": "Relay", "supported": true}
    ]})
}

/// 按 `a`，交回常用的几家。
fn catalog() -> Settings {
    let mut page = opened(false);
    press(&mut page, KeyCode::Char('a'));
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].1, "provider.catalog");
    assert_eq!(asks[0].2, json!({"featured": true}));
    page.answer(asks[0].0, Ok(featured()), &texts());
    page
}

/// `a` 开目录、目录回来是空的，选「自定义…」：开空的编辑窗（别的测试新加供应商用）。
pub(super) fn custom(page: &mut Settings) {
    press(page, KeyCode::Char('a'));
    let tag = page.take_asks()[0].0;
    page.answer(tag, Ok(json!({})), &texts());
    press(page, KeyCode::Char('j'));
    press(page, KeyCode::Enter);
    assert!(matches!(page.popup, Some(Popup::Form(_))));
}

/// 选中的那一行。
fn chosen(page: &Settings) -> String {
    let Some(Popup::Catalog(pick)) = &page.popup else {
        panic!("目录窗没开：{:?}", page.popup);
    };
    match pick.rows()[pick.sel] {
        Row::Provider(p) => p.id.clone(),
        Row::More => "more".into(),
        Row::Custom => "custom".into(),
    }
}

/// 选了一家开的编辑窗：编号、`catalog`。
fn form(page: &Settings) -> (String, Option<String>) {
    let Some(Popup::Form(form)) = &page.popup else {
        panic!("编辑窗没开：{:?}", page.popup);
    };
    let id = form.text(crate::settings::forms::Field::Id).to_string();
    (id, form.catalog.clone())
}

#[test]
fn a_lists_the_featured_providers_then_more_and_custom_skipping_ones_that_cannot_connect() {
    let mut page = catalog();
    assert_eq!(chosen(&page), "deepseek");
    let mut seen = vec![];
    for _ in 0..5 {
        press(&mut page, KeyCode::Char('j'));
        seen.push(chosen(&page));
    }
    assert_eq!(
        seen,
        ["wafer.ai", "relay", "more", "custom", "custom"],
        "接不上的跳过，到底不绕回"
    );
    press(&mut page, KeyCode::Enter);
    assert_eq!(form(&page), (String::new(), None), "自定义…开空的编辑窗");
}

#[test]
fn picking_one_fills_in_a_path_safe_id_apart_from_existing_ones_and_saves_the_catalog() {
    let mut page = catalog();
    press(&mut page, KeyCode::Enter);
    assert_eq!(form(&page), ("deepseek".into(), Some("deepseek".into())));
    press(&mut page, KeyCode::Esc);
    let mut page = catalog();
    press(&mut page, KeyCode::Char('j'));
    press(&mut page, KeyCode::Enter);
    assert_eq!(form(&page), ("wafer-ai".into(), Some("wafer.ai".into())));
    let mut page = catalog();
    for _ in 0..2 {
        press(&mut page, KeyCode::Char('j'));
    }
    press(&mut page, KeyCode::Enter);
    assert_eq!(
        form(&page),
        ("relay-2".into(), Some("relay".into())),
        "配置里有 relay 了"
    );
    // 只填 key 的窗：回车就存（这里没填 key）。
    press(&mut page, KeyCode::Enter);
    let asks = page.take_asks();
    let (_, _, sent) = asks
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存");
    let catalog = sent["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "providers.relay-2.catalog");
    assert_eq!(
        catalog.map(|c| &c["value"]),
        Some(&json!("relay")),
        "{sent}"
    );
}

#[test]
fn an_id_matching_the_catalog_is_not_written_twice() {
    let mut page = catalog();
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Enter);
    let asks = page.take_asks();
    let (_, _, sent) = asks
        .iter()
        .find(|(_, m, _)| *m == "config.set")
        .expect("存");
    assert!(
        !sent.to_string().contains("providers.deepseek.catalog"),
        "{sent}"
    );
}

#[test]
fn more_reads_the_whole_catalog_and_a_late_featured_answer_is_ignored() {
    let mut page = opened(false);
    press(&mut page, KeyCode::Char('a'));
    let featured_tag = page.take_asks()[0].0;
    // 常用的还没回来就选「更多供应商…」（还没回来时它是第一行）。
    assert_eq!(chosen(&page), "more");
    press(&mut page, KeyCode::Enter);
    let asks = page.take_asks();
    assert_eq!(asks[0].1, "provider.catalog");
    assert!(asks[0].2.get("featured").is_none(), "全目录：{}", asks[0].2);
    page.answer(featured_tag, Ok(featured()), &texts());
    let Some(Popup::Catalog(pick)) = &page.popup else {
        panic!("目录窗没开");
    };
    assert!(pick.providers.is_none(), "晚到的常用的那份不换上");
    let all = json!({"providers": [
        {"id": "zhipu", "name": "Zhipu", "supported": true},
        {"id": "anthropic", "name": "Anthropic", "supported": true}
    ]});
    page.answer(asks[0].0, Ok(all), &texts());
    assert_eq!(chosen(&page), "anthropic", "全目录照名字排");
    press(&mut page, KeyCode::Char('/'));
    for c in "zhi".chars() {
        press(&mut page, KeyCode::Char(c));
    }
    assert!(page.typing());
    press(&mut page, KeyCode::Enter);
    assert_eq!(chosen(&page), "zhipu");
    let Some(Popup::Catalog(pick)) = &page.popup else {
        panic!("目录窗没开");
    };
    assert!(!pick.rows().contains(&Row::More), "全目录里没有「更多」");
    press(&mut page, KeyCode::Esc);
    assert_eq!(chosen(&page), "anthropic", "Esc 先清筛的");
    press(&mut page, KeyCode::Esc);
    assert!(page.popup.is_none(), "再 Esc 关窗");
}

#[test]
fn picking_one_asks_only_for_its_key_enter_saves_and_esc_drops_it() {
    // 2026-10-29 项目主人：选了目录里有的一家，只弹一个填 key 的框（照 opencode），用户不知道要去哪填 key。
    use crate::settings::forms::Field;
    let mut page = catalog();
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Form(f)) = &page.popup else {
        panic!("开了窗：{:?}", page.popup);
    };
    assert_eq!(f.key_only.as_deref(), Some("DeepSeek"), "标题后面写名字");
    assert!(f.editing.is_some(), "直接在打");
    let shown: Vec<Field> = (0..f.rows.len())
        .filter(|&i| f.shows(i))
        .map(|i| f.rows[i].field)
        .collect();
    assert_eq!(shown, [Field::Key], "只露 key 那一行");
    for c in "sk-test".chars() {
        press(&mut page, KeyCode::Char(c));
    }
    press(&mut page, KeyCode::Enter);
    let asks = page.take_asks();
    let secret = asks.iter().find(|(_, m, _)| *m == "secret.set");
    assert_eq!(
        secret.map(|(_, _, p)| &p["value"]),
        Some(&json!("sk-test")),
        "回车就存，key 进密钥库：{asks:?}"
    );
    let mut page = catalog();
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Esc);
    assert!(!matches!(page.popup, Some(Popup::Form(_))), "Esc 关掉");
    assert!(page.take_asks().is_empty(), "不存");
}
