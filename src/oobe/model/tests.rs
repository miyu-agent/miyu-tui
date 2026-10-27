//! 接模型那一步（「第一次打开的引导」第 15–20 条）：一步步走，查发给核心的请求。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::{Value, json};

use super::{Phase, Slot};
use crate::oobe::tests::{asks, find, fresh, key, texts, to_model, typed};
use crate::oobe::{Oobe, Step, Texts};

fn catalog() -> Value {
    json!({"providers": [
        {"id": "deepseek", "name": "DeepSeek", "supported": true},
        {"id": "openai", "name": "OpenAI", "supported": true}]})
}

/// 走到接模型、三份回应和配置都回来；`chat` 是现在的聊天模型。
fn at_model(t0: Instant, texts: &Texts, chat: Option<&str>) -> Oobe {
    let mut oobe = fresh(t0);
    to_model(&mut oobe, t0, texts);
    let sent = asks(&mut oobe);
    let list = match chat {
        Some(chat) => json!({"uses": {"chat": chat}, "providers": [{"id": "zen", "models": [
            {"model": "big", "ref": "zen/big", "facts": {"name": {"value": "Big Model"}}}]}]}),
        None => json!({"uses": {"chat": null}, "providers": []}),
    };
    for (tag, method, _) in sent {
        let reply = match method {
            "model.list" => list.clone(),
            "provider.detect" => {
                json!({"keys": [{"env": "DEEPSEEK_API_KEY", "provider": "deepseek", "supported": true}]})
            }
            "provider.catalog" => catalog(),
            "config.get" => json!({"items": {}}),
            _ => json!({}),
        };
        oobe.answer(tag, Ok(reply), t0, texts);
    }
    oobe
}

#[test]
fn a_found_key_is_tested_right_away_and_the_chosen_model_is_saved_by_reference() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    assert!(matches!(oobe.model.phase, Phase::List));
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(matches!(oobe.model.phase, Phase::Testing(_)));
    assert!(oobe.react.waiting, "试的时候抬头");
    let (tag, params) = find(&asks(&mut oobe), "provider.test");
    assert_eq!(
        params,
        json!({"candidate": {"catalog": "deepseek", "key": {"env": "DEEPSEEK_API_KEY"}}})
    );
    let ok = json!({"ok": true, "models": ["deepseek-v4-pro", "deepseek-flash"], "model": "deepseek-flash"});
    oobe.answer(tag, Ok(ok), t0, &texts);
    let Phase::Models(models) = &oobe.model.phase else {
        panic!("{:?}", oobe.model.phase)
    };
    assert_eq!(
        models.names,
        ["deepseek-flash", "deepseek-v4-pro"],
        "试的排第一"
    );
    assert!(!oobe.react.waiting && !oobe.react.droop);
    // `/` 搜、打字筛：只剩一个，回车搜完，再回车存。
    oobe.key(key(KeyCode::Char('/')), t0, &texts);
    typed(&mut oobe, "pro", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "搜的时候回车只是搜完");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let sent = asks(&mut oobe);
    assert!(
        sent.iter().all(|(_, m, _)| *m != "secret.set"),
        "找到的 key 不存"
    );
    let (tag, params) = find(&sent, "config.set");
    let changes = params["changes"].as_array().unwrap();
    assert!(
        changes.contains(
            &json!({"key": "providers.deepseek.key", "value": {"env": "DEEPSEEK_API_KEY"}})
        )
    );
    assert!(changes.contains(&json!({"key": "models.chat", "value": "deepseek/deepseek-v4-pro"})));
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Currency, "接模型后面是货币那一步");
    assert_eq!(oobe.summary().model, "deepseek-v4-pro");
}

#[test]
fn a_pasted_key_is_needed_tested_and_stored_before_the_config() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(matches!(oobe.model.phase, Phase::Form(_)), "请填写密钥");
    // 光标停在密钥上打字不进去（第 7 条）：回车才编辑。
    typed(&mut oobe, "x", t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(
        oobe.model.error.as_deref(),
        Some(texts.model.key_needed.as_str()),
        "停在「测试连接」上回车：没填不试"
    );
    oobe.paste("sk-test\n");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "回车只是写好");
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "provider.test");
    assert_eq!(params["candidate"]["key"], json!({"value": "sk-test"}));
    // 没试通：红字照哪一步加原话，耷拉耳朵，光标回到密钥。
    let failed =
        json!({"ok": false, "stage": "request", "error": {"class": "auth", "message": "bad key"}});
    oobe.answer(tag, Ok(failed), t0, &texts);
    assert!(matches!(oobe.model.phase, Phase::Form(_)));
    assert!(oobe.react.droop);
    let error = oobe.model.error.clone().unwrap();
    assert!(
        error.contains("发送请求") && error.contains("bad key"),
        "{error}"
    );
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.test");
    oobe.answer(
        tag,
        Ok(json!({"ok": true, "models": ["gpt-4o"], "model": "gpt-4o"})),
        t0,
        &texts,
    );
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "secret.set");
    assert_eq!(params, json!({"name": "openai", "value": "sk-test"}));
    assert!(
        sent.iter().all(|(_, m, _)| *m != "config.set"),
        "key 存好了再写配置"
    );
    oobe.answer(tag, Ok(json!({"replaced": false})), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    let changes = params["changes"].as_array().unwrap();
    assert!(
        changes.contains(&json!({"key": "providers.openai.key", "value": {"secret": "openai"}}))
    );
    assert!(
        !params.to_string().contains("pools."),
        "池不写：三个固定池核心自己有（Q-1）"
    );
}

#[test]
fn a_custom_endpoint_needs_a_good_address_and_asks_for_a_model_name_when_it_cannot_list() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    for _ in 0..5 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let Phase::Form(form) = &oobe.model.phase else {
        panic!()
    };
    assert!(form.provider.is_none(), "自定义");
    assert_eq!(
        form.slot(),
        Some(Slot::Id),
        "编号在最上面，光标先停在它上面"
    );
    // 编号下面是显示名称（2026-10-26 项目主人），再下面是地址。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    let Phase::Form(form) = &oobe.model.phase else {
        panic!()
    };
    assert_eq!(form.slot(), Some(Slot::Name));
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "api.example.invalid", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(
        oobe.model.error.as_deref(),
        Some(texts.model.url_bad.as_str())
    );
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(
        ratatui::crossterm::event::KeyEvent::new(
            KeyCode::Char('u'),
            ratatui::crossterm::event::KeyModifiers::CONTROL,
        ),
        t0,
        &texts,
    );
    typed(&mut oobe, "https://api.example.invalid/v1/", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.model.error, None, "地址对了红字去掉");
    // 接口那一格 `l` 换下一个；再往下两格（密钥）到「测试连接」。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('l')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "provider.test");
    assert_eq!(
        params,
        json!({"candidate": {"driver": "anthropic", "base_url": "https://api.example.invalid/v1"}}),
        "空着的 key 不带"
    );
    // 核心 8-11 五补：列不出、给模型名能再试的带 `need_model`。
    oobe.answer(
        tag,
        Ok(json!({"ok": false, "stage": "list", "need_model": true, "error": {"message": "404"}})),
        t0,
        &texts,
    );
    assert_eq!(
        oobe.model.warn.as_deref(),
        Some(texts.model.no_list.as_str())
    );
    assert!(!oobe.react.droop, "列不出模型不算没连上");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "glm-5", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "provider.test");
    assert_eq!(params["model"], "glm-5");
    // 拿填的模型名发了一句、通了：列出的是 0 个，写「glm-5 可以用」。
    oobe.answer(
        tag,
        Ok(json!({"ok": true, "checked": "request", "models": [], "listed": "catalog", "model": "glm-5", "first_token_ms": 812})),
        t0,
        &texts,
    );
    let Phase::Models(models) = &oobe.model.phase else {
        panic!("到了选模型");
    };
    assert_eq!(
        (models.listed, models.names.as_slice()),
        (0, &["glm-5".to_string()][..])
    );
}

#[test]
fn a_list_stage_failure_without_need_model_is_a_plain_failure() {
    // 核心 8-11 五补：列模型那一步认证错的（没带 `need_model`）照没通办，不问模型名。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    // 找到 key 的那一家排第一：回车就试。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.test");
    oobe.answer(
        tag,
        Ok(json!({"ok": false, "stage": "list", "error": {"class": "auth", "status": 401, "message": "bad key"}})),
        t0,
        &texts,
    );
    assert_eq!(oobe.model.warn, None, "不问模型名");
    assert!(
        oobe.model
            .error
            .as_deref()
            .is_some_and(|e| e.contains("bad key"))
    );
}

#[test]
fn an_existing_chat_model_can_be_kept_with_one_enter() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, Some("zen/big"));
    assert!(matches!(oobe.model.phase, Phase::Ready(true)));
    assert_eq!(oobe.model.chat.as_deref(), Some("Big Model"), "写显示名");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.step, Step::Currency, "接模型后面是货币那一步");
    assert_eq!(oobe.summary().model, "Big Model");
}

#[test]
fn more_providers_opens_a_searchable_window_and_enter_fills_in_that_one() {
    // 第 16a 条（2026-10-10 项目主人：多一个「更多供应商」，浮窗里上面一格搜索，回车进填写那一屏）。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    // 按键模式是方向键（出厂；Vim 的见下一条）。
    // 两家常用的、「其他」段名（停不上）、更多供应商。
    for _ in 0..2 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.model.more.is_some(), "开了浮窗");
    let (tag, params) = find(&asks(&mut oobe), "provider.catalog");
    assert!(params.get("featured").is_none(), "拿全目录：{params}");
    assert!(params["limit"].as_u64().unwrap() >= 1000, "{params}");
    let all = json!({"providers": [
        {"id": "zai", "name": "Z.AI", "supported": true},
        {"id": "moonshotai-cn", "name": "Moonshot AI (China)", "supported": true},
        {"id": "weird", "name": "Moon Weird", "supported": false},
        {"id": "moonshotai", "name": "Moonshot AI", "supported": true}]});
    oobe.answer(tag, Ok(all), t0, &texts);
    // 开窗就在搜索那一格里：打字就筛，j 也是字。
    typed(&mut oobe, "moon", t0, &texts);
    let more = oobe.model.more.as_ref().unwrap();
    let names: Vec<&str> = more.matches().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Moon Weird", "Moonshot AI", "Moonshot AI (China)"]);
    assert_eq!(more.cursor, 1, "接不上的停不上");
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.model.more.is_none(), "关窗");
    let Phase::Form(form) = &oobe.model.phase else {
        panic!("{:?}", oobe.model.phase)
    };
    assert_eq!(form.provider.as_ref().unwrap().catalog, "moonshotai-cn");
    // Esc 只关窗。
    oobe.key(key(KeyCode::Esc), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    asks(&mut oobe);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert!(oobe.model.more.is_none() && matches!(oobe.model.phase, Phase::List));
    assert_eq!(oobe.step, Step::Model);
}

#[test]
fn with_vim_keys_the_more_window_searches_after_a_slash() {
    // 2026-10-21 项目主人：按键模式是 Vim 时 `/` 才搜，`Enter` 收起、筛的留着，`j` `k` 挪，`Esc` 先去掉筛的再关窗。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    oobe.model.vim = true;
    for _ in 0..2 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.catalog");
    let all = json!({"providers": [
        {"id": "zai", "name": "Z.AI", "supported": true},
        {"id": "moonshotai-cn", "name": "Moonshot AI (China)", "supported": true},
        {"id": "moonshotai", "name": "Moonshot AI", "supported": true}]});
    oobe.answer(tag, Ok(all), t0, &texts);
    let more = oobe.model.more.as_ref().unwrap();
    assert!(!more.filter.editing(), "开窗不在搜索那一格");
    typed(&mut oobe, "zz", t0, &texts);
    assert_eq!(
        oobe.model.more.as_ref().unwrap().filter.text(),
        "",
        "没按 / 打的字不进搜索"
    );
    oobe.key(key(KeyCode::Char('/')), t0, &texts);
    typed(&mut oobe, "moon", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let more = oobe.model.more.as_ref().unwrap();
    assert!(!more.filter.editing(), "Enter 收起");
    assert_eq!(more.matches().len(), 2, "筛的留着");
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    assert_eq!(oobe.model.more.as_ref().unwrap().cursor, 1, "j 往下");
    oobe.key(key(KeyCode::Esc), t0, &texts);
    let more = oobe.model.more.as_ref().expect("Esc 先去掉筛的，窗还开着");
    assert_eq!(more.matches().len(), 3);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert!(oobe.model.more.is_none(), "再按关窗");
}

#[test]
fn a_failed_test_can_still_be_saved_and_asks_for_a_model_name_first() {
    // 2026-10-13 项目主人定：测不通时报错照样显示，多一个「仍然保存」，选了就照填的写进配置、进下一步。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.paste("sk-test\n");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.test");
    let failed = json!({"ok": false, "stage": "request", "error": {"class": "auth", "status": 401, "message": "bad key"}});
    oobe.answer(tag, Ok(failed), t0, &texts);
    let go_to = |oobe: &mut Oobe, slot: Slot| {
        for _ in 0..6 {
            let Phase::Form(form) = &oobe.model.phase else {
                panic!("{:?}", oobe.model.phase)
            };
            if form.slot() == Some(slot) {
                return;
            }
            oobe.key(key(KeyCode::Char('j')), t0, &texts);
        }
        panic!("走不到 {slot:?}");
    };
    assert!(oobe.model.error.is_some(), "报错照样写");
    go_to(&mut oobe, Slot::Anyway);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "没有模型名先不存");
    assert_eq!(
        oobe.model.warn.as_deref(),
        Some(texts.model.model_for_save.as_str())
    );
    let Phase::Form(form) = &oobe.model.phase else {
        panic!()
    };
    assert_eq!(form.slot(), Some(Slot::Model), "光标停到模型名");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "gpt-x", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    go_to(&mut oobe, Slot::Anyway);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "secret.set");
    assert_eq!(params, json!({"name": "openai", "value": "sk-test"}));
    oobe.answer(tag, Ok(json!({"replaced": false})), t0, &texts);
    let (tag, params) = find(&asks(&mut oobe), "config.set");
    let changes = params["changes"].as_array().unwrap();
    assert!(changes.contains(&json!({"key": "models.chat", "value": "openai/gpt-x"})));
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Currency, "存好往下走");
}

#[test]
fn a_custom_provider_takes_the_id_someone_types_and_checks_it() {
    // 2026-10-13 项目主人定（核心待定第 6 条，照推荐）：自定义一家让人起编号，默认是照地址自动起的那个，可以改。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_model(t0, &texts, None);
    for _ in 0..5 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "https://api.example.invalid/v1", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let Phase::Form(form) = &oobe.model.phase else {
        panic!()
    };
    assert_eq!(
        oobe.model.auto_id(form).as_deref(),
        Some("example"),
        "空着时照地址起"
    );
    // 编号那一格（最上面）：写法不对的不试，红字照配置页那一句。
    oobe.key(key(KeyCode::Char('k')), t0, &texts);
    oobe.key(key(KeyCode::Char('k')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "Bad_ID", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0..5 {
        oobe.key(key(KeyCode::Char('j')), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "编号不对不试");
    assert!(
        oobe.model
            .error
            .as_deref()
            .is_some_and(|e| e.contains("Bad_ID"))
    );
    // 光标跳回编号那一格；改成对的：试通了存的时候用它。
    let Phase::Form(form) = &oobe.model.phase else {
        panic!()
    };
    assert_eq!(form.slot(), Some(Slot::Id));
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0.."Bad_ID".len() {
        oobe.key(key(KeyCode::Backspace), t0, &texts);
    }
    typed(&mut oobe, "my-relay", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    // 显示名称填上的，存的时候一起写（2026-10-26 项目主人）。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "我的中转", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0..4 {
        oobe.key(key(KeyCode::Char('j')), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "provider.test");
    oobe.answer(
        tag,
        Ok(json!({"ok": true, "checked": "list", "models": ["m1"], "listed": "provider", "model": "m1"})),
        t0,
        &texts,
    );
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "config.set");
    let text = params.to_string();
    assert!(text.contains("providers.my-relay."), "{text}");
    assert!(text.contains("\"my-relay/m1\""), "{text}");
    assert!(
        params["changes"]
            .as_array()
            .unwrap()
            .contains(&json!({"key": "providers.my-relay.name", "value": "我的中转"})),
        "{text}"
    );
}
