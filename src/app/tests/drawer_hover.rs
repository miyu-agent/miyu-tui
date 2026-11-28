//! 多道题的提问里点掉最后一道，还没重画鼠标又动了一下：手里的还是上一帧的行表，抽屉已经到了「确认」页（审计
//! 2026-11-28 第 1 条：原来悬停直接下标 `cursor[tab]`，到「确认」页越界 panic）。

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use serde_json::json;

use crate::drawer::{Asked, Drawer};

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn moving_after_clicking_the_last_question_away_does_not_panic() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    let asked: Asked = serde_json::from_value(json!({
        "call_id": "q1",
        "questions": [
            {"question": "第一道", "options": [{"label": "甲"}, {"label": "乙"}]},
            {"question": "第二道", "options": [{"label": "丙"}, {"label": "丁"}]},
        ],
    }))
    .unwrap();
    app.talk.drawers.reopen(Drawer::question(None, asked));
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    app.handle(Event::Key(KeyEvent::new(
        KeyCode::Char('1'),
        KeyModifiers::NONE,
    )));
    term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let d = app.talk.drawers.current.as_ref().expect("抽屉还开着");
    assert_eq!(d.tab, 1, "答了第一道，到第二道");
    let text = app.screen.areas.text;
    let row = app
        .screen
        .drawer_rows
        .iter()
        .position(Option::is_some)
        .expect("第二道有选项行");
    let (x, y) = (text.x + 2, text.y + u16::try_from(row).unwrap());
    app.handle(mouse(MouseEventKind::Down(MouseButton::Left), x, y));
    // 不重画：行表还是上一帧的。
    app.handle(mouse(MouseEventKind::Moved, x, y));
    let d = app.talk.drawers.current.as_ref().expect("抽屉还开着");
    assert!(d.on_review(), "两道都答了，停在「确认」页");
}
