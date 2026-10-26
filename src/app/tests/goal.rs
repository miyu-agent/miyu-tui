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
