use serde_json::json;

use super::{Field, Focus, Target, Val, apply, model, pool, provider, window_text, window_value};
use crate::settings::draft::Draft;
use crate::settings::test_support::sample;

fn set_text(form: &mut super::Form, field: super::Field, text: &str) {
    let row = form.rows.iter_mut().find(|r| r.field == field).unwrap();
    if let Val::Text { text: t, .. } = &mut row.val {
        *t = text.to_string();
    }
}

#[test]
fn windows_read_and_write_in_k_and_m() {
    assert_eq!(window_text(128_000), "128k");
    assert_eq!(window_text(1_000_000), "1M");
    assert_eq!(window_text(1_048_576), "1M");
    assert_eq!(window_value("128k"), Some(128_000));
    assert_eq!(window_value("1.5M"), Some(1_500_000));
    assert_eq!(window_value("abc"), None);
}

#[test]
fn a_new_provider_needs_a_valid_unused_id_and_its_key_goes_to_the_secret_store() {
    let data = sample();
    let mut draft = Draft::default();
    let mut form = provider(&data, &data, &draft, None);
    assert!(matches!(form.focus, Focus::Row(0)), "新加的先填 ID");
    set_text(&mut form, Field::Id, "Zen");
    assert_eq!(
        apply(&form, &data, &data, &mut draft),
        Err(("bad_id", "Zen".into()))
    );
    set_text(&mut form, Field::Id, "relay");
    assert_eq!(
        apply(&form, &data, &data, &mut draft),
        Err(("taken", "relay".into()))
    );
    assert!(draft.is_empty(), "不收的什么都不记");
    set_text(&mut form, Field::Id, "zen");
    set_text(&mut form, Field::Key, "sk-zen");
    assert_eq!(apply(&form, &data, &data, &mut draft), Ok("zen".into()));
    assert_eq!(draft.new_providers, ["zen"]);
    assert!(draft.has_secret("zen"));
    assert!(!draft.changed("providers.zen.driver"), "接口「自动」不写");
}

#[test]
fn an_existing_provider_writes_only_what_moved() {
    let data = sample();
    let mut draft = Draft::default();
    let mut form = provider(&data, &data, &draft, Some("relay"));
    assert!(matches!(form.rows[0].val, Val::Fixed(_)), "已有的 ID 只读");
    set_text(&mut form, Field::Name, "Relay");
    apply(&form, &data, &data, &mut draft).unwrap();
    assert_eq!(
        draft.value("providers.relay.name", &data),
        Some(&json!("Relay"))
    );
    assert!(!draft.changed("providers.relay.base_url"));
    assert!(!draft.has_secret("relay"), "key 留空不改");
}

#[test]
fn the_model_form_shows_inherited_values_with_sources_and_greys_out_temperature() {
    let data = sample();
    let draft = Draft::default();
    let form = model(&data, &data, &draft, "relay", Some("zhipu/glm-5v"));
    let window = form.rows.iter().find(|r| r.field == Field::Window).unwrap();
    assert_eq!(window.inherit.as_deref(), Some("64k"));
    assert_eq!(window.source.as_deref(), Some("provider"));
    assert!(!window.own());
    let temperature = form
        .rows
        .iter()
        .find(|r| r.field == Field::Temperature)
        .unwrap();
    assert_eq!(
        temperature.val,
        Val::Fixed("no_temperature".into()),
        "目录说不收温度"
    );
}

#[test]
fn touching_one_price_writes_the_whole_price() {
    let data = sample();
    let mut draft = Draft::default();
    let mut form = model(&data, &data, &draft, "relay", Some("cline/deepseek-v4"));
    set_text(&mut form, Field::Output, "2");
    apply(&form, &data, &data, &mut draft).unwrap();
    let table = "providers.relay.models.\"cline/deepseek-v4\".price";
    let got = |item: &str| draft.value(&format!("{table}.{item}"), &data).cloned();
    assert_eq!(got("input"), Some(json!(0.27)), "没动的照显示的继承值补上");
    assert_eq!(got("output"), Some(json!(2.0)));
    assert_eq!(got("cache_read"), Some(json!(0.07)));
    assert_eq!(got("currency"), Some(json!("USD")));
    set_text(&mut form, Field::Output, "x");
    let mut other = Draft::default();
    assert_eq!(
        apply(&form, &data, &data, &mut other),
        Err(("bad_number", "output".into()))
    );
}

#[test]
fn a_new_model_is_written_with_its_inputs_so_the_config_lists_it() {
    let data = sample();
    let mut draft = Draft::default();
    let mut form = model(&data, &data, &draft, "dev", None);
    set_text(&mut form, Field::Model, "pro");
    set_text(&mut form, Field::Window, "1M");
    assert_eq!(apply(&form, &data, &data, &mut draft), Ok("dev/pro".into()));
    assert_eq!(
        draft.value("providers.dev.models.pro.inputs", &data),
        Some(&json!(["text"]))
    );
    assert_eq!(
        draft.value("providers.dev.models.pro.window", &data),
        Some(&json!(1_000_000))
    );
}

#[test]
fn pool_members_keep_the_order_they_were_ticked_and_gone_members_come_first() {
    let mut data = sample();
    data.pools[0].members.insert(0, "old/gone".into());
    let mut draft = Draft::default();
    let mut form = pool(&data, Some("daily"));
    assert_eq!(form.target, Target::Pool(Some("daily".into())));
    let first_member = form
        .rows
        .iter()
        .find(|r| matches!(r.val, Val::Member { .. }))
        .unwrap();
    assert!(matches!(&first_member.val, Val::Member { gone: true, .. }));
    form.members.retain(|m| m != "dev/flash");
    form.members.push("relay/gpt-oss".into());
    apply(&form, &data, &data, &mut draft).unwrap();
    assert_eq!(
        draft.value("pools.daily.models", &data),
        Some(&json!([
            "old/gone",
            "relay/cline/deepseek-v4",
            "relay/gpt-oss"
        ]))
    );
}

#[test]
fn inputs_offer_audio_and_video_too() {
    // 2026-10-07 项目主人要；核心 8-27 起配置收这五种，先后照核心的。
    let data = sample();
    let form = model(&data, &data, &Draft::default(), "dev", Some("flash"));
    let row = form.rows.iter().find(|r| r.field == Field::Inputs).unwrap();
    let Val::Multi { options, .. } = &row.val else {
        panic!("多选");
    };
    assert_eq!(options, &["text", "image", "pdf", "audio", "video"]);
}

#[test]
fn constrained_decoding_is_auto_on_or_off_and_auto_writes_nothing() {
    // 核心 L-2（2026-10-15）：供应商多一项 `compat.strict_tools`，自动照档案（不写），开、关写布尔。
    let data = sample();
    let pick = |at: usize| {
        let mut draft = Draft::default();
        let mut form = provider(&data, &data, &draft, Some("relay"));
        let row = form
            .rows
            .iter_mut()
            .find(|r| r.field == Field::StrictTools)
            .expect("有约束解码这一行");
        assert!(
            matches!(&row.val, Val::Choice { at: Some(0), .. }),
            "没写的停在自动"
        );
        if let Val::Choice { at: a, .. } = &mut row.val {
            *a = Some(at);
        }
        apply(&form, &data, &data, &mut draft).unwrap();
        draft
    };
    let key = "providers.relay.compat.strict_tools";
    assert_eq!(pick(1).value(key, &data), Some(&json!(true)));
    assert_eq!(pick(2).value(key, &data), Some(&json!(false)));
    assert!(!pick(0).changed(key), "自动不写");
}

#[test]
fn editing_a_text_row_keeps_the_old_text_and_types_after_it() {
    // 2026-10-20 项目主人：回车进编辑整段选中，一敲就替换了全部的字，不对。光标停在末尾、不选中。
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let data = sample();
    let mut form = provider(&data, &data, &Draft::default(), Some("relay"));
    let row = form
        .rows
        .iter()
        .position(|r| matches!(r.val, Val::Text { .. }))
        .unwrap();
    form.focus = Focus::Row(row);
    for code in [KeyCode::Enter, KeyCode::Char('!')] {
        crate::settings::form_keys::form_key(&mut form, KeyEvent::new(code, KeyModifiers::NONE));
    }
    let text = form.editing.as_ref().map_or("", |e| e.text());
    assert!(
        text.len() > 1 && text.ends_with('!'),
        "原来的字还在：{text}"
    );
}
