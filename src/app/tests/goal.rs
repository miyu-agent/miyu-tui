//! 点目标那一行开长期目标的浮窗（2026-10-26 项目主人：同 `/status` 的浮窗，Esc 关）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Position, Rect};

use super::app;
use crate::app::Panel;
use crate::core::{Goal, GoalState};

#[test]
fn clicking_the_goal_text_opens_its_box_and_esc_closes_it() {
    let (mut app, _sent) = app();
    app.talk.transcript.goal = Some(Goal {
        state: GoalState::Running,
        round: 2,
        limit: Some(5),
        text: "把测试修绿".into(),
        ..Goal::default()
    });
    app.screen.areas.goal = Rect::new(0, 20, 80, 1);
    // 画的时候记下右对齐的那一截字（`ui/goal_tag.rs`）。
    let text = crate::ui::goal_tag::goal(
        app.talk.transcript.goal.as_ref(),
        &app.config,
        jiff::Timestamp::now(),
    )
    .expect("有目标");
    app.screen.areas.goal_text = crate::ui::goal_tag::spot(app.screen.areas.goal, &text);
    // 悬停在字上指针变手（2026-10-27 项目主人报：原来不变）。
    app.screen.pointer = Some(Position::new(79, 20));
    assert!(app.pointing(), "悬停在字上：手");
    app.screen.pointer = Some(Position::new(5, 20));
    assert!(!app.pointing(), "左边空白：不是手");
    assert!(
        !app.goal_clicked(Position::new(5, 20)),
        "点在左边空白：不开"
    );
    assert!(
        app.goal_clicked(Position::new(79, 20)),
        "点在右对齐的字上：开"
    );
    assert!(matches!(app.boxes.panel, Some(Panel::Goal { scroll: 0 })));
    app.goal_key(0, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(app.boxes.panel.is_none());
    app.talk.transcript.goal = None;
    assert!(!app.goal_clicked(Position::new(79, 20)), "没有目标：不开");
}
