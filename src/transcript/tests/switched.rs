//! 会话中途换人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 2a 条，核心 P-13）：照推来的换掉会话用的那一个，正文记一行
//! 「已换成」，名字照列表、没到的写编号。

use std::collections::BTreeMap;

use super::super::{Kind, Transcript, set_names};
use super::apply;
use crate::core::Push;

#[test]
fn a_switch_takes_effect_and_leaves_a_note_with_the_name() {
    set_names(
        true,
        BTreeMap::from([("sw-xue".to_string(), "小雪".to_string())]),
    );
    let mut t = Transcript {
        persona: Some("sw-old".into()),
        ..Transcript::default()
    };
    apply(&mut t, vec![Push::PersonaSet("sw-xue".into())]);
    assert_eq!(t.persona.as_deref(), Some("sw-xue"), "侧边栏、头像照新的");
    apply(&mut t, vec![Push::PersonaSet(String::new())]);
    assert_eq!(t.persona, None, "空的是无人格");
    apply(&mut t, vec![Push::PresetSet("sw-unlisted".into())]);
    assert_eq!(t.preset.as_deref(), Some("sw-unlisted"));
    let notes: Vec<(&str, Kind)> = t
        .entries
        .iter()
        .map(|e| (e.text.as_str(), e.kind.clone()))
        .collect();
    assert_eq!(
        notes,
        [
            ("人格已换成「小雪」", Kind::Note),
            ("人格已换成「无人格」", Kind::Note),
            ("预设已换成「sw-unlisted」", Kind::Note),
        ],
        "列表里没有的写编号"
    );
}
