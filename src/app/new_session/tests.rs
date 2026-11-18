//! 新会话开头的框起初停在哪一行。

use super::Pick;
use crate::core::Persona;

fn row(id: &str, problem: Option<&str>) -> Persona {
    Persona {
        id: id.into(),
        name: None,
        summary: None,
        problem: problem.map(str::to_string),
        avatar: None,
        seed: None,
        seeds: None,
        tokens: None,
    }
}

#[test]
fn the_box_starts_on_the_last_pick_then_the_default() {
    // 2026-10-09 项目主人：「照旧弹框，光标停在上一次的」。
    let pick = Pick {
        list: Some(vec![
            row("", None),
            row("miyu", None),
            row("broken", Some("写错了")),
            row("engineer", None),
        ]),
        default: Some("miyu".into()),
        ..Pick::default()
    };
    assert_eq!(pick.start(Some(Some("engineer"))), 3, "上一次选的");
    assert_eq!(pick.start(Some(None)), 0, "上一次没带人格：无人格");
    assert_eq!(
        pick.start(Some(Some("broken"))),
        1,
        "上一次的用不了：默认的"
    );
    assert_eq!(pick.start(Some(Some("gone"))), 1, "上一次的没了：默认的");
    assert_eq!(pick.start(None), 1, "没有会话：默认的");
}
