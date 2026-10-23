//! 抽屉里贴图（「确认和提问的抽屉」第 3a 条，核心 Q-A；2026-10-22 项目主人）：贴给光标所在的那一道、确认只有「不允许」
//! 收；只贴了图也算答了；字删空了再按 `Backspace` 删最后一张；交出去、留痕都带着。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Approval, Asked, Decision, Drawer, Outcome, Pasted, Report, Step, said};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn image(n: usize) -> Pasted {
    Pasted {
        label: format!("[图片 {n}]"),
        file: format!("/tmp/{n}.png").into(),
    }
}

fn one_question() -> Asked {
    serde_json::from_value(serde_json::json!({
        "call_id": "c1",
        "questions": [{"question": "哪种配色？", "options": [{"label": "深色"}, {"label": "浅色"}]}]
    }))
    .unwrap()
}

#[test]
fn an_image_alone_answers_the_question_and_backspace_takes_the_last_back() {
    let mut d = Drawer::question(None, one_question());
    assert!(d.attach(image(1)));
    assert!(d.attach(image(2)));
    d.key(key(KeyCode::Backspace));
    assert_eq!(d.images[0], [image(1)], "没在写字时 Backspace 删最后一张");
    // 移到「其他」、什么都不写直接回车：只贴了图也算答了。
    d.key(key(KeyCode::Down));
    d.key(key(KeyCode::Down));
    d.key(key(KeyCode::Enter));
    let Step::Done(Outcome::Answered(a)) = d.key(key(KeyCode::Enter)) else {
        panic!("只贴了图也交得出去");
    };
    assert_eq!(a.answers[0].files, [image(1)]);
    assert_eq!(said(&a.answers[0]), "[图片 1]", "留痕写块上的字");
    let body = serde_json::to_value(&a.answers[0]).unwrap();
    assert_eq!(
        body,
        serde_json::json!({}),
        "图不跟着 body 走，连接那头另填 attachments"
    );
}

#[test]
fn a_picked_option_carries_the_images_of_its_question() {
    let mut d = Drawer::question(None, one_question());
    d.attach(image(1));
    let Step::Done(Outcome::Answered(a)) = d.key(key(KeyCode::Enter)) else {
        panic!("选了第一项就交");
    };
    assert_eq!(a.answers[0].picked, ["深色"]);
    assert_eq!(said(&a.answers[0]), "深色 [图片 1]");
}

#[test]
fn an_approval_takes_images_only_on_dont_allow() {
    // 钉住的核心检出里的协议样本（同 `tests.rs`）。
    let path = std::path::Path::new(miyu_client::WORKSPACE_ROOT)
        .join("docs/designs/samples/events/tool.approval_requested.jsonl");
    let line = std::fs::read_to_string(path).expect("核心检出里有这份样本");
    let event: serde_json::Value = serde_json::from_str(line.lines().next().unwrap()).unwrap();
    let approval: Approval = serde_json::from_value(event["body"].clone()).unwrap();
    let mut d = Drawer::approval(None, approval);
    assert!(!d.attach(image(1)), "光标在「允许」上不收");
    let deny = d
        .items(0)
        .iter()
        .position(|i| *i == super::Item::Decision(Decision::Deny))
        .unwrap();
    for _ in 0..deny {
        d.key(key(KeyCode::Down));
    }
    assert!(d.attach(image(1)));
    for c in "看图".chars() {
        d.key(key(KeyCode::Char(c)));
    }
    d.key(key(KeyCode::Backspace));
    d.key(key(KeyCode::Backspace));
    assert_eq!(d.images[0].len(), 1, "先删字");
    d.key(key(KeyCode::Backspace));
    assert!(d.images[0].is_empty(), "字删空了再删图");
    d.attach(image(2));
    let Step::Done(Outcome::Decided(decided)) = d.key(key(KeyCode::Enter)) else {
        panic!("不允许交了");
    };
    assert_eq!(decided.files, [image(2)]);
    let texts = crate::config::Config::builtin().unwrap().text.drawer;
    let report = d.report(&Outcome::Decided(decided), &texts);
    let Report::Denied(head, _) = report else {
        panic!("不允许留一行");
    };
    assert!(head.contains("[图片 2]"), "{head}");
}
