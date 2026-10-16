//! 你说的话记下什么时候说的（`/jump` 的列表右边写它，蓝图「跳到说过的话」第 1 条）。

use super::apply;
use crate::core::Push;
use crate::transcript::{Kind, Transcript};

#[test]
fn a_replayed_line_you_said_keeps_the_time_of_its_event() {
    let at: jiff::Timestamp = "2026-10-16T08:00:00Z".parse().unwrap();
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::Clock(Some(at)),
            Push::Said {
                seq: 3,
                text: "老会话里的一句".into(),
                attached: Vec::new(),
            },
            Push::Clock(None),
        ],
    );
    let said = t.entries.iter().find(|e| e.kind == Kind::User).unwrap();
    assert_eq!(said.at, Some(at));
}
