use super::rows;
use crate::config::Config;
use crate::core::{Goal, GoalState};

#[test]
fn the_box_lists_the_goal_its_state_rounds_time_and_usage() {
    // 2026-10-26 项目主人：目标全文、状态、轮数/上限、用时、用量。
    let config = Config::builtin().unwrap();
    let goal = Goal {
        state: GoalState::Blocked,
        round: 2,
        limit: Some(5),
        text: "把所有测试修绿".into(),
        reason: Some("缺权限".into()),
        resumes: 2,
        since: Some(jiff::Timestamp::UNIX_EPOCH),
        tokens: 12_300,
        ..Goal::default()
    };
    let now = jiff::Timestamp::from_second(65).unwrap();
    let text: Vec<String> = rows(&goal, Some("¥0.20"), &config, (60, now))
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    let has = |want: &str| text.iter().any(|l| l.contains(want));
    assert!(has("把所有测试修绿"), "{text:?}");
    assert!(
        has("受阻 · 缺权限 · 恢复 2 次"),
        "停了又续上过的接恢复次数：{text:?}"
    );
    assert!(has("2/5"), "{text:?}");
    assert!(has("1m 05s"), "{text:?}");
    assert!(has("12.3k token · ¥0.20"), "{text:?}");
    // 没给用时、用量的不写那两行。
    let bare = Goal {
        state: GoalState::Running,
        round: 1,
        text: "t".into(),
        ..Goal::default()
    };
    assert_eq!(rows(&bare, None, &config, (60, now)).len(), 3);
}
