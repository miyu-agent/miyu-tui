//! 「文件修改」那一页（发版前 check 9，2026-11-13 主人 1113 初验）：侧边栏列改过的文件，点开整屏看差异、右边完整列表，`↑` `↓`
//! 换文件（换了问它的完整差异），核心的差异回来了换上，`Esc` 回会话；没改过文件的 `/files` 说一句。

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use crate::core::{Command, ToolStatus, Update};
use crate::input::Draft;
use crate::transcript::{Entry, Kind, Segment, Step, StepKind, ToolState};

fn drained(sent: &mut tokio::sync::mpsc::UnboundedReceiver<Command>) -> Vec<Command> {
    std::iter::from_fn(|| sent.try_recv().ok()).collect()
}

/// 发出去的 `view.detail`：请求编号、问的调用。
fn details(sent: &[Command]) -> Vec<(u64, String)> {
    sent.iter()
        .filter_map(|c| match c {
            Command::Ask {
                tag,
                method: "view.detail",
                params,
            } => Some((
                *tag,
                params["call"].as_str().unwrap_or_default().to_string(),
            )),
            _ => None,
        })
        .collect()
}

fn edit(path: &str, call: &str, old: &str, new: &str) -> Step {
    let args = json!({"file_path": path, "edits": [{"old_string": old, "new_string": new}]});
    let mut step = Step::new(StepKind::Tool {
        name: "edit".into(),
        args: args.to_string(),
        parsed: args,
        state: ToolState::Done(ToolStatus::Ok),
        output: String::new(),
        said: None,
    });
    step.call_id = Some(call.into());
    step
}

fn press(app: &mut crate::app::App, code: KeyCode) {
    app.handle(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}

fn screen(app: &mut crate::app::App, width: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(width, 24)).unwrap();
    term.draw(|f| crate::ui::draw(f, app)).unwrap();
    let buffer = term.backend().buffer().clone();
    // 宽字后面那一格是占位的，拼字时跳过。
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut skip = false;
            for x in 0..buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                if !std::mem::take(&mut skip) {
                    line.push_str(symbol);
                    skip = unicode_width::UnicodeWidthStr::width(symbol) == 2;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn with_two_files() -> (
    crate::app::App,
    tokio::sync::mpsc::UnboundedReceiver<Command>,
) {
    let (mut app, sent) = crate::app::test_support::app();
    app.boxes.welcome_known = true;
    app.boxes.personas_known = true;
    app.talk.transcript.session = Some("s1".into());
    let mut e = Entry::new(1, Kind::Steps, String::new());
    let mut s = Segment::new();
    s.steps = vec![
        edit("/w/alpha.rs", "c1", "old line\n", "new line\n"),
        edit("/w/beta.rs", "c2", "b\n", "B\n"),
    ];
    s.finished = true;
    e.segment = Some(s);
    app.talk.transcript.entries.push(e);
    (app, sent)
}

#[test]
fn opening_asks_the_full_diff_switching_asks_the_next_and_esc_goes_back() {
    let (mut app, mut sent) = with_two_files();
    drained(&mut sent);
    app.submit(Draft::plain("/files"));
    assert!(app.boxes.files_page.is_some(), "/files 开那一页");
    let asked = details(&drained(&mut sent));
    assert_eq!(
        asked.iter().map(|(_, c)| c.as_str()).collect::<Vec<_>>(),
        ["c1"]
    );
    let text = screen(&mut app, 140);
    assert!(
        text.contains("alpha.rs") && text.contains("beta.rs"),
        "右边列表：\n{text}"
    );
    assert!(
        text.contains("+ new line") && text.contains("- old line"),
        "左边差异：\n{text}"
    );
    // 核心的完整差异回来了：带行号的换上
    let diff: Value = json!({"files": [{"path": "/w/alpha.rs", "action": "write",
        "diff": ["@@ -40,1 +40,1 @@", "-old line", "+brand new line"], "added": 1, "removed": 1}], "output": ""});
    app.core(Update::Answer {
        tag: asked[0].0,
        result: Ok(diff),
    });
    let text = screen(&mut app, 140);
    assert!(
        text.contains("40  + brand new line"),
        "照核心的差异、带行号：\n{text}"
    );
    press(&mut app, KeyCode::Down);
    let asked = details(&drained(&mut sent));
    assert_eq!(
        asked.iter().map(|(_, c)| c.as_str()).collect::<Vec<_>>(),
        ["c2"],
        "换了文件问它的"
    );
    assert!(screen(&mut app, 140).contains("+ B"));
    press(&mut app, KeyCode::Esc);
    assert!(app.boxes.files_page.is_none(), "Esc 回会话");
}

#[test]
fn the_sidebar_lists_changed_files_and_a_session_without_changes_says_so() {
    let (mut app, _sent) = with_two_files();
    let text = screen(&mut app, 140);
    let words = app.config.text.files.clone();
    assert!(
        text.contains(&words.title),
        "侧边栏有「文件修改」：\n{text}"
    );
    assert!(text.contains("+1 -1"), "{text}");
    let (mut empty, _sent) = crate::app::test_support::app();
    empty.talk.transcript.session = Some("s2".into());
    empty.submit(Draft::plain("/files"));
    assert!(empty.boxes.files_page.is_none());
    assert_eq!(
        empty.typing.notice.as_ref().map(|n| n.text.clone()),
        Some(words.none)
    );
}
