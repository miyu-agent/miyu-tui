//! 窗口只有一行、零行高（审计 2026-11-28 第 9 条）：扩展整页原来在判高度以前就把面包屑写在「上边加一」那一行，
//! 落到缓冲区外 panic。

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::widgets::PageRef;

const TINY: [(u16, u16); 6] = [(40, 1), (3, 1), (80, 1), (120, 1), (40, 0), (120, 2)];

#[test]
fn an_extension_page_draws_in_one_or_zero_rows() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    app.open_widget_page(PageRef {
        package: "say".into(),
        widget: "voices".into(),
    });
    for (w, h) in TINY {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    }
}

#[test]
fn the_main_view_and_a_drawer_draw_in_a_tiny_window_without_panicking() {
    use serde_json::json;
    let (mut app, _sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    for (w, h) in TINY {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    }
    app.talk.transcript.user("在吗".into(), Vec::new());
    app.talk.transcript.running = Some(std::time::Instant::now());
    for (w, h) in TINY {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    }
    let asked: crate::drawer::Asked = serde_json::from_value(json!({
        "call_id": "q1",
        "questions": [{"question": "挑一个", "options": [{"label": "甲"}]}, {"question": "再挑", "options": [{"label": "乙"}]}],
    }))
    .unwrap();
    app.talk
        .drawers
        .reopen(crate::drawer::Drawer::question(None, asked));
    for (w, h) in TINY {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    }
}
