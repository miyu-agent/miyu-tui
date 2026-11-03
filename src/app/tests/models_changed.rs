//! 模型列表在核心后台变了（核心 V-26 的 `models.changed`，「配置与模型」第 1 条子规则）：`/model` 的框开着的再要一次、光标
//! 留在原来那一行；底栏再要一次模型资料。

use super::super::test_support::app;
use crate::app::Panel;
use crate::core::{Choice, ChoiceState, Command, Current, Update};
use crate::model_list::ModelList;

fn choice(reference: &str) -> Choice {
    Choice {
        reference: reference.into(),
        name: reference.into(),
        detail: String::new(),
        state: ChoiceState::Ok,
    }
}

#[test]
fn the_open_model_box_asks_again_and_keeps_its_row() {
    let (mut app, mut sent) = app();
    // 正在用的是 dev/a：后台再要来的不该停回它。
    let current = Current {
        endpoint: Some("dev".into()),
        model: Some("a".into()),
        reference: "dev/a".into(),
        effort: None,
        effort_own: false,
    };
    app.talk
        .transcript
        .update(Update::CurrentModel(current), &app.config.text);
    app.boxes.model_list = Some(ModelList::default());
    app.boxes.panel = Some(Panel::Models);
    let all = || vec![choice("dev/a"), choice("dev/b"), choice("dev/c")];
    app.models_listed(all());
    app.boxes.model_list.as_mut().unwrap().step(true);
    app.boxes.model_list.as_mut().unwrap().step(true);
    while sent.try_recv().is_ok() {}
    app.core(Update::ModelsChanged);
    let mut asked = Vec::new();
    while let Ok(command) = sent.try_recv() {
        asked.push(command);
    }
    assert!(
        asked.iter().any(|c| matches!(c, Command::ListChoices)),
        "框开着的再要一次：{asked:?}"
    );
    let mut fresh = all();
    fresh.insert(0, choice("dev/new"));
    app.models_listed(fresh);
    let picked = app
        .boxes
        .model_list
        .as_ref()
        .unwrap()
        .picked()
        .map(|c| c.reference.clone());
    assert_eq!(picked.as_deref(), Some("dev/c"), "光标留在原来那一行");
}

#[test]
fn with_no_box_open_only_the_footer_asks_again() {
    let (mut app, mut sent) = app();
    while sent.try_recv().is_ok() {}
    app.core(Update::ModelsChanged);
    let mut asked = Vec::new();
    while let Ok(command) = sent.try_recv() {
        asked.push(command);
    }
    assert!(
        !asked.iter().any(|c| matches!(c, Command::ListChoices)),
        "{asked:?}"
    );
}
