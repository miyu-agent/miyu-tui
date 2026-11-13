//! 同一个路径又展示了一次（蓝图「展示」第 5 条）：前面那张算被换掉的；撤掉了新的那一轮，旧的回来。

use super::super::Transcript;
use super::apply;
use crate::core::{Push, Shown};

fn file(modified: &str) -> Shown {
    Shown {
        path: "/w/index.html".into(),
        modified: Some(modified.into()),
        name: "index.html".into(),
        media_type: "text/html".into(),
        size: 1,
        title: None,
        language: None,
    }
}

#[test]
fn the_earlier_showing_of_a_path_is_replaced_until_the_later_one_is_undone() {
    let mut t = Transcript::default();
    apply(&mut t, vec![Push::Shown(file("2026-11-13T01:00:00Z"))]);
    assert!(t.replaced_shown().is_empty());
    apply(&mut t, vec![Push::Shown(file("2026-11-13T02:00:00Z"))]);
    let first = t
        .entries
        .iter()
        .find(|e| {
            e.shown
                .as_ref()
                .is_some_and(|f| f.modified.as_deref() == Some("2026-11-13T01:00:00Z"))
        })
        .unwrap()
        .id;
    assert_eq!(t.replaced_shown().into_iter().collect::<Vec<_>>(), [first]);
    // 撤掉了新的那一轮：藏起来的不算，旧的回来。
    t.entries.last_mut().unwrap().hidden = true;
    assert!(t.replaced_shown().is_empty());
}
