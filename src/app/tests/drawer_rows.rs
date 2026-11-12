//! 抽屉开着时框下面那一行、运行状态行和排队的消息让给抽屉（发版前 check 44，2026-11-12 项目主人：「claudecode 的 tui
//! 在弹出确认抽屉的时候不是仅替换掉输入框」）。

use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde_json::json;

use crate::drawer::{Asked, Drawer};

#[test]
fn an_open_drawer_takes_the_footer_and_the_running_rows() {
    let (mut app, _sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    app.talk.transcript.user("在吗".into(), Vec::new());
    app.talk.transcript.running = Some(Instant::now());
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let plain = app.screen.areas;
    assert_eq!(
        (plain.footer.height, plain.pulse.height),
        (1, 1),
        "没开抽屉照旧"
    );
    // 一道选项很多的题：抽屉排得满它能占的行数。
    let options: Vec<_> = (1..=30)
        .map(|i| json!({"label": format!("选项{i}")}))
        .collect();
    let asked: Asked = serde_json::from_value(json!({
        "call_id": "q1",
        "questions": [{"question": "挑一个", "options": options}],
    }))
    .unwrap();
    app.talk.drawers.reopen(Drawer::question(None, asked));
    term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
    let open = app.screen.areas;
    assert_eq!(open.footer.height, 0, "框下面那一行让给抽屉");
    assert_eq!(
        (open.pulse.height, open.queued.height),
        (0, 0),
        "运行状态行让给抽屉"
    );
    assert_eq!(open.frame.bottom(), 24, "抽屉贴到最底下");
    assert_eq!(open.body.bottom() + 1, open.frame.y, "上面只空一行");
    // 半屏（24 / 2 - 2 = 10 行）再加让出来的三行（框下面那一行、运行状态行和它上下的空行少掉的两行）。
    assert_eq!(open.text.height, 13, "让出几行抽屉就多几行");
    let bottom: String = (0..100)
        .map(|x| term.backend().buffer()[(x, 23)].symbol().to_string())
        .collect();
    assert!(
        !bottom.contains("权限"),
        "最底下一行是抽屉的边，不是框下面那一行：{bottom}"
    );
}
