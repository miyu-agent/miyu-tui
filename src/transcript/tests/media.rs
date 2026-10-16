//! 她发出去的图（核心 X-19，蓝图「她发的图」）。

use super::apply;
use crate::core::{Block, EndReason, Push, Sent, ToolStatus};
use crate::transcript::{Kind, Transcript};

fn sent(alt: &str) -> Sent {
    Sent {
        blob: format!("sha256:{alt}"),
        name: None,
        alt: Some(alt.into()),
        sticker: true,
    }
}

#[test]
fn a_sent_picture_comes_after_its_step_and_folds_the_segment_like_a_reply() {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::TurnStarted(1, None),
            Push::BlockStart {
                index: 0,
                block: Block::ToolCall("send_sticker".into()),
            },
            Push::BlockEnd(0),
            Push::Calls(vec!["c1".into()]),
            Push::ToolResult {
                call_id: "c1".into(),
                status: ToolStatus::Ok,
                text: String::new(),
                said: None,
                display: Default::default(),
            },
            Push::Media(vec![sent("摊手"), sent("比心")]),
        ],
    );
    let kinds: Vec<_> = t.entries.iter().map(|e| e.kind.clone()).collect();
    assert_eq!(
        kinds,
        [Kind::Steps, Kind::Media, Kind::Media],
        "一张一条，排在那一步后面"
    );
    let segment = t.entries[0].segment.as_ref().unwrap();
    assert_eq!(segment.steps.len(), 1, "那一步照旧在时间线里");
    assert!(!segment.expanded(true), "和回答一样收起前面那一段");
    assert_eq!(
        t.entries[2].media.as_ref().unwrap().alt.as_deref(),
        Some("比心")
    );
    assert!(
        t.entries[1..].iter().all(|e| e.turn == Some(1)),
        "跟着这一轮撤销、恢复"
    );
    // 再想、再调工具的另起一段，不接回收起的那段。
    apply(
        &mut t,
        vec![
            Push::BlockStart {
                index: 1,
                block: Block::Reasoning,
            },
            Push::BlockEnd(1),
            Push::TurnEnded(EndReason::Completed),
            Push::Reverted(vec![1]),
        ],
    );
    assert_eq!(t.entries[3].kind, Kind::Steps);
    assert!(t.entries.iter().all(|e| e.hidden), "撤销了这一轮跟着藏起来");
}
