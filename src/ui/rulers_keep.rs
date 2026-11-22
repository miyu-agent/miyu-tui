//! 量尺（`RULER_TURNS=300 cargo test --release ruler_keep -- --ignored --nocapture`）：长会话只留视口附近几屏排好的行
//! （施工 LW-22，蓝图「正文」第 8 条）。照 App 的做法画（测试终端，整帧 `ui::draw`），和全部留着（`keep_screens` 0）
//! 比（翻页、回到底部照按键做的调视口的方法，测试里的界面没连核心、按键被开场的框收走）：按条记着的行留着多少条、合计多少字节；逐行滚、翻页、跳远、记着的行整份清空（来一张链接卡片）各一帧花多久。
//! 只打数字，不断言。`RULER_TURNS` 不给的三档都量（30、150、300）。

use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::text::Span;
use serde_json::json;

use crate::app::App;
use crate::core::ToolStatus;
use crate::transcript::{Kind, Segment, Step, StepKind, ToolState};
use crate::ui::rows::Row;

/// 和 `ruler_memory` 一样的一轮：你说的话、一段想了 120 行再跑两条命令的时间线、一篇 20 段的回答。
fn session(app: &mut App, turns: usize) {
    let t0 = Instant::now();
    let think =
        "我们需要想清楚这个问题的每一步，然后再给出结论，顺便检查一下边界情况。\n".repeat(120);
    let reply = "## 结论\n\n- 第一点：**要紧的**先说\n- 第二点：`code` 放这里\n\n一段普通的回答文字，稍微长一点，好折几行。\n".repeat(20);
    let step = |kind, start| {
        let mut s = Step::new(kind);
        s.started = t0 + Duration::from_secs(start);
        s.took = Some(Duration::from_secs(1));
        s
    };
    let command = || StepKind::Tool {
        name: "shell".into(),
        args: String::new(),
        parsed: json!({"command": "ls", "description": "列目录"}),
        state: ToolState::Done(ToolStatus::Ok),
        output: "a.txt".into(),
        said: None,
    };
    let t = &mut app.talk.transcript;
    for _ in 0..turns {
        t.note(Kind::User, "帮我看看这个目录".into());
        t.note(Kind::Steps, String::new());
        let mut seg = Segment::new();
        seg.steps = vec![
            step(
                StepKind::Thought {
                    text: think.clone(),
                },
                0,
            ),
            step(command(), 3),
            step(command(), 4),
        ];
        seg.finished = true;
        t.entries.last_mut().unwrap().segment = Some(seg);
        t.note(Kind::Reply, reply.clone());
    }
}

/// 一行占的字节：结构本身、每段字（结构和文字）、复制用的纯文字（同 `ruler_memory`）。
fn bytes(row: &Row) -> usize {
    let spans = &row.line.spans;
    let text: usize = spans
        .iter()
        .map(|s| match &s.content {
            std::borrow::Cow::Owned(o) => o.capacity(),
            std::borrow::Cow::Borrowed(_) => 0,
        })
        .sum();
    std::mem::size_of::<Row>()
        + spans.capacity() * std::mem::size_of::<Span<'static>>()
        + text
        + row.plain().len()
}

/// 画一帧，交回花了几毫秒。
fn frame(app: &mut App, term: &mut Terminal<TestBackend>) -> f64 {
    let start = Instant::now();
    term.draw(|f| crate::ui::draw(f, app)).unwrap();
    start.elapsed().as_secs_f64() * 1000.0
}

fn mouse(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
    let modifiers = KeyModifiers::NONE;
    app.handle(Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    }));
}

/// 按在滚动条的第 `y` 行（滑块中间跳过去）再松开：拖滚动条跳远的那条路。
fn bar(app: &mut App, y: u16) {
    let x = app.screen.view.bar_column();
    let top = app.screen.areas.body.y;
    mouse(app, MouseEventKind::Down(MouseButton::Left), x, top + y);
    mouse(app, MouseEventKind::Up(MouseButton::Left), x, top + y);
}

/// 几帧的最慢、平均（毫秒）。
fn stats(times: &[f64]) -> String {
    let max = times.iter().cloned().fold(0.0, f64::max);
    let mean = times.iter().sum::<f64>() / times.len().max(1) as f64;
    format!("最慢 {max:.2}ms、平均 {mean:.2}ms（{} 帧）", times.len())
}

#[test]
#[ignore]
fn ruler_keep() {
    let only = std::env::var("RULER_TURNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());
    for turns in [30usize, 150, 300]
        .into_iter()
        .filter(|t| only.is_none_or(|o| o == *t))
    {
        for keep in [4usize, 0] {
            let (mut app, _sent) = crate::app::test_support::app();
            app.boxes.welcome_known = true;
            app.boxes.personas_known = true;
            app.config.layout.keep_screens = keep;
            session(&mut app, turns);
            let mut term = Terminal::new(TestBackend::new(120, 40)).unwrap();
            let first = frame(&mut app, &mut term);
            frame(&mut app, &mut term);
            let (entries, rows, size) = {
                let cache = app.media.row_cache.borrow();
                let kept: Vec<&[Row]> = cache.kept_rows().collect();
                let rows: usize = kept.iter().map(|r| r.len()).sum();
                let size: usize = kept.iter().flat_map(|r| r.iter()).map(bytes).sum();
                (format!("{}/{}", kept.len(), cache.kept_len()), rows, size)
            };
            let total = app.screen.view.rows.len();
            println!(
                "{turns} 轮、keep_screens {keep}：正文 {total} 行；行缓存留着 {entries} 条的行、{rows} 行、{} KB；头一帧 {first:.2}ms",
                size / 1024
            );
            let height = app.screen.areas.body.height;
            // 逐行滚：滚轮在底部往上一格（三行）一帧，滚十格。
            let mut wheel = Vec::new();
            for _ in 0..10 {
                let a = app.screen.areas.body;
                mouse(
                    &mut app,
                    MouseEventKind::ScrollUp,
                    a.x + 2,
                    a.y + a.height / 2,
                );
                wheel.push(frame(&mut app, &mut term));
            }
            // 翻页：往上翻半屏一帧，连翻二十次。
            let mut page = Vec::new();
            for _ in 0..20 {
                app.screen.view.page(false);
                page.push(frame(&mut app, &mut term));
            }
            // 跳远：拖滚动条在开头、四分之一、中间、四分之三、底部之间来回跳，每一下一帧。
            let mut jump = Vec::new();
            for y in [
                0,
                height / 4,
                0,
                height / 2,
                height - 1,
                0,
                height * 3 / 4,
                0,
            ] {
                bar(&mut app, y);
                jump.push(frame(&mut app, &mut term));
            }
            // 来一张链接卡片：记着的行整份清空（`app/cards.rs`），下一帧全部重排。
            app.screen.view.follow();
            frame(&mut app, &mut term);
            *app.media.row_cache.borrow_mut() = Default::default();
            let cleared = frame(&mut app, &mut term);
            println!(
                "  逐行滚 {}；翻页 {}；跳远 {}；整份清空以后那一帧 {cleared:.2}ms",
                stats(&wheel),
                stats(&page),
                stats(&jump)
            );
        }
    }
}
