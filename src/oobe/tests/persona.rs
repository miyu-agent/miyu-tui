//! 建人格那一步（「第一次打开的引导」第 21–23 条）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts, typed};
use crate::oobe::{Move, Oobe, Step, Texts};

/// 走到建人格：默认人格没设。
fn at_persona(t0: Instant, texts: &Texts, default: Option<&str>) -> Oobe {
    let mut oobe = fresh(t0);
    oobe.step = Step::Currency;
    oobe.moved(Move::Next, t0);
    assert_eq!(oobe.step, Step::Persona);
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "config.get");
    assert_eq!(params, json!({"keys": ["persona.default"]}));
    let value = default.map_or(
        json!({"items": {}}),
        |d| json!({"items": {"persona.default": {"value": d}}}),
    );
    oobe.answer(tag, Ok(value), t0, texts);
    oobe
}

#[test]
fn a_new_persona_is_created_in_one_go_and_becomes_the_default() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, None);
    assert!(!oobe.persona.busy);
    // 名称：回车开始编辑，再回车写好（第 7 条）。
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "小助手", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(!oobe.persona.name.editing());
    // 简单描述（名字下面，2026-10-17）：同名称，回车编辑、回车写好。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "会写诗", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    // 具体人格提示词（头像下面）：回车开大编辑浮窗，写完 Esc 收起（第 21a 条）。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.persona.writing.is_some());
    typed(&mut oobe, "你是小助手。", t0, &texts);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert!(
        oobe.persona.writing.is_none() && oobe.step == Step::Persona,
        "Esc 只收浮窗"
    );
    // 示范对话：回车开列表，a 加一轮，两格都写了回车存。
    oobe.key(key(KeyCode::Down), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.persona.listing);
    oobe.key(key(KeyCode::Char('a')), t0, &texts);
    typed(&mut oobe, "在吗", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "在", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.persona.editing.is_none() && oobe.persona.pairs.len() == 1);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert!(
        !oobe.persona.listing && oobe.step == Step::Persona,
        "Esc 只关列表"
    );
    // 人设提醒短语空着；到「下一步」回车（vim 的 j 也认：这一格不打字）。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "persona.set");
    assert_eq!(
        params,
        json!({"changes": [{"key": "persona.name", "value": "小助手"},
                           {"key": "persona.summary", "value": "会写诗"}],
               "prompts": {"persona": {"text": "你是小助手。"},
                           "examples": {"pairs": [{"user": "在吗", "assistant": "在"}]}}}),
        "不写编号、空的不带"
    );
    oobe.answer(
        tag,
        Ok(json!({"persona": "persona-1", "name": "小助手"})),
        t0,
        &texts,
    );
    let sent = asks(&mut oobe);
    let (tag, params) = find(&sent, "config.set");
    assert_eq!(
        params,
        json!({"layer": "personal", "changes": [{"key": "persona.default", "value": "persona-1"}]})
    );
    oobe.answer(tag, Ok(json!({})), t0, &texts);
    assert_eq!(oobe.step, Step::Theme, "人格后面是主题（配色跟着头像）");
    assert_eq!(
        oobe.badge.as_ref().map(|b| b.0.as_str()),
        Some("小助手"),
        "名字写到吉祥物脚下"
    );
    assert_eq!(oobe.summary().persona, "小助手");
    assert_eq!(
        oobe.picks().0.as_deref(),
        Some("persona-1"),
        "紧接着的新会话照它开"
    );
}

#[test]
fn a_name_is_needed_and_the_sheet_writes_lines_and_hands_over_to_the_editor() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, None);
    // 具体人格提示词（名字、简单描述、头像下面）：大编辑浮窗里 Enter 换行（第 21a 条，2026-10-09 项目主人：复杂的提示词要能写）。
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "一", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "二", t0, &texts);
    // Ctrl+G：交给编辑器，回来写回浮窗、接着在浮窗里。
    let effects = oobe.key(super::ctrl('g'), t0, &texts);
    assert_eq!(effects, [crate::oobe::Effect::Editor("一\n二".into())]);
    oobe.edited(Some("三\n四\n".into()));
    assert!(oobe.persona.writing.is_some(), "回来还在浮窗里");
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.persona.prompt.text(), "三\n四");
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "名字空着不发");
    assert_eq!(
        oobe.persona.error.as_deref(),
        Some(texts.persona.name_needed.as_str())
    );
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Name);
    // 停在名字上打字不进去，j、k 挪光标；回车编辑以后 j 是字。
    typed(&mut oobe, "jk", t0, &texts);
    assert_eq!(oobe.persona.name.text(), "");
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Name);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "jk", t0, &texts);
    assert_eq!(oobe.persona.name.text(), "jk");
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Name);
    // Esc 不改：回到编辑前的字。
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.persona.name.text(), "");
    assert_eq!(oobe.step, Step::Persona, "编辑时 Esc 不回上一步");
}

#[test]
fn an_existing_default_is_filled_in_and_only_changes_are_saved_with_versions() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, Some("persona-1"));
    let sent = asks(&mut oobe);
    let (tag, _) = find(&sent, "persona.get");
    oobe.answer(
        tag,
        Ok(json!({"persona": "persona-1", "name": "小助手", "summary": "会写诗"})),
        t0,
        &texts,
    );
    let sent = asks(&mut oobe);
    assert_eq!(
        sent.iter().filter(|(_, m, _)| *m == "persona.read").count(),
        3
    );
    for (tag, _, params) in sent {
        let reply = match params["prompt"].as_str() {
            Some("persona") => json!({"text": "你是小助手。\n", "version": "v1"}),
            Some("reminders") => json!({"text": null, "version": null}),
            _ => json!({"text": null, "version": null, "pairs": []}),
        };
        oobe.answer(tag, Ok(reply), t0, &texts);
    }
    assert_eq!(oobe.persona.name.text(), "小助手");
    assert_eq!(oobe.persona.summary.text(), "会写诗", "简单描述也填好");
    assert_eq!(oobe.persona.prompt.text(), "你是小助手。");
    assert!(oobe.persona.existing_name.is_some());
    // 没改：直接写默认人格。
    for _ in 0..6 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let sent = asks(&mut oobe);
    assert!(sent.iter().all(|(_, m, _)| *m != "persona.set"), "{sent:?}");
    find(&sent, "config.set");
    // 改了人设：只交人设、带版本。
    let mut oobe = at_persona(t0, &texts, Some("persona-1"));
    let sent = asks(&mut oobe);
    let (tag, _) = find(&sent, "persona.get");
    oobe.answer(
        tag,
        Ok(json!({"persona": "persona-1", "name": "小助手", "summary": "会写诗"})),
        t0,
        &texts,
    );
    for (tag, _, params) in asks(&mut oobe) {
        let reply = match params["prompt"].as_str() {
            Some("persona") => json!({"text": "你是小助手。", "version": "v1"}),
            _ => json!({"text": null, "version": null, "pairs": []}),
        };
        oobe.answer(tag, Ok(reply), t0, &texts);
    }
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "很开朗。", t0, &texts);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "persona.set");
    assert_eq!(
        params,
        json!({"persona": "persona-1", "prompts": {"persona": {"text": "你是小助手。很开朗。", "expect": "v1"}}})
    );
}

#[test]
fn the_input_method_switches_only_while_a_field_is_being_edited() {
    // 「配置页」第 30 条（2026-10-10 项目主人：「fcitx5 的适配你也要做一下」）：引导里不在编辑时关成英文。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, None);
    assert!(!oobe.editing(), "停在名称上不算打字");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(oobe.editing(), "回车进编辑");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert!(!oobe.editing());
    // 名字下面的简单描述、头像路径一样（2026-10-26 项目主人报：进第二格输入法停在英文，编辑里的状态没记住）。
    for slot in [
        crate::oobe::persona::Slot::Summary,
        crate::oobe::persona::Slot::Avatar,
    ] {
        oobe.key(key(KeyCode::Down), t0, &texts);
        assert_eq!(oobe.persona.focus, slot);
        assert!(!oobe.editing(), "停着不算");
        oobe.key(key(KeyCode::Enter), t0, &texts);
        assert!(oobe.editing(), "{slot:?} 回车进编辑算在打字");
        oobe.key(key(KeyCode::Enter), t0, &texts);
        assert!(!oobe.editing());
    }
}

#[test]
fn an_avatar_path_is_uploaded_then_set_on_the_new_persona() {
    // 「第一次打开的引导」第 21 条（2026-10-10 项目主人：创建人格时给一个头像文件路径的选项）。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, None);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "小助手", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    // 头像在名字、简单描述下面：拖进来的路径带引号的去掉引号。
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Avatar);
    // 照这台机器的写法拼一个绝对路径（`/tmp/...` 在 Windows 上会被补上盘符）；文件不在，原样交。
    let avatar = std::env::temp_dir()
        .join("miyu-avatar.png")
        .display()
        .to_string();
    oobe.paste(&format!("'{avatar}'"));
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.persona.avatar.text(), avatar);
    for _ in 0..4 {
        oobe.key(key(KeyCode::Char('j')), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let sent = asks(&mut oobe);
    assert!(sent.iter().all(|(_, m, _)| *m != "persona.set"), "先传头像");
    let (tag, params) = find(&sent, "blob.put");
    assert_eq!(params, json!({"path": avatar}));
    oobe.answer(
        tag,
        Ok(json!({"blob": "sha256:ab", "kind": "image", "media_type": "image/png", "name": "miyu-avatar.png"})),
        t0,
        &texts,
    );
    let (_, params) = find(&asks(&mut oobe), "persona.set");
    assert_eq!(params["avatar"], json!({"blob": "sha256:ab"}));
    assert_eq!(
        params["changes"],
        json!([{"key": "persona.name", "value": "小助手"}])
    );
}

#[test]
fn an_avatar_that_cannot_be_uploaded_says_why_and_goes_back_to_it() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, None);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    typed(&mut oobe, "小助手", t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    oobe.paste("/tmp/没有这个文件.png");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0..4 {
        oobe.key(key(KeyCode::Char('j')), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "blob.put");
    let refusal = crate::core::Refusal {
        reason: Some("attachment_unreadable".into()),
        message: "读不了这个文件".into(),
        data: serde_json::Value::Null,
    };
    oobe.answer(tag, Err(refusal), t0, &texts);
    assert!(asks(&mut oobe).is_empty(), "传不上不建");
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Avatar);
    assert!(
        oobe.persona
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("读不了")
    );
}

#[test]
fn the_last_screen_shows_the_avatar_that_was_set_and_otherwise_the_mascot() {
    // 2026-10-13 项目主人定：引导结尾的动画，人格设了头像的画头像，没设的照旧吉祥物。
    let none = |_: &str| None;
    let mut step = crate::oobe::persona::PersonaStep::default();
    assert_eq!(step.finale_avatar(none), None, "没设头像：吉祥物");
    let dir = std::env::temp_dir().join(format!("miyu-finale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("头像.png");
    std::fs::write(&file, b"\x89PNG\r\n\x1a\n").unwrap();
    step.avatar = crate::oobe::field::Field::line().with(&file.display().to_string());
    assert_eq!(step.finale_avatar(none), Some(file.clone()));
    step.avatar =
        crate::oobe::field::Field::line().with(&dir.join("没有这张.png").display().to_string());
    assert_eq!(step.finale_avatar(none), None, "填的文件不在：吉祥物");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn clearing_the_short_description_of_an_existing_default_removes_it() {
    // 2026-10-17：简单描述是 `persona.summary`；清空了原来有的去掉，别的没改不带。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_persona(t0, &texts, Some("persona-1"));
    let (tag, _) = find(&asks(&mut oobe), "persona.get");
    oobe.answer(
        tag,
        Ok(json!({"persona": "persona-1", "name": "小助手", "summary": "会写诗"})),
        t0,
        &texts,
    );
    for (tag, _, _) in asks(&mut oobe) {
        let reply = json!({"text": null, "version": null, "pairs": []});
        oobe.answer(tag, Ok(reply), t0, &texts);
    }
    oobe.key(key(KeyCode::Char('j')), t0, &texts);
    assert_eq!(oobe.persona.focus, crate::oobe::persona::Slot::Summary);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0.."会写诗".chars().count() {
        oobe.key(key(KeyCode::Backspace), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    for _ in 0..5 {
        oobe.key(key(KeyCode::Char('j')), t0, &texts);
    }
    oobe.key(key(KeyCode::Enter), t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "persona.set");
    assert_eq!(
        params,
        json!({"persona": "persona-1", "changes": [{"key": "persona.summary", "unset": true}]})
    );
}
