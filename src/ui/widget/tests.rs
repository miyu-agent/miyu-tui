use ratatui::text::Line;
use serde_json::json;

use super::{Look, Mode, block, content};
use crate::config::Config;
use crate::widgets::{Action, Segment};

fn text(line: &Line) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .map(|l| text(l).trim_end().to_string())
        .collect()
}

fn segments(value: serde_json::Value) -> Vec<Segment> {
    let block = crate::widgets::block(&json!({
        "package": "ci", "id": "builds", "place": "sidebar", "source": "CI Watch", "title": "构建",
        "running": true, "content": value
    }))
    .expect("读得出");
    block.content.expect("有内容")
}

fn with_look<T>(mode: Mode, width: u16, run: impl FnOnce(&Look) -> T) -> T {
    let config = Config::builtin().expect("出厂配置读得出");
    let look = Look {
        width,
        mode,
        marks: &config.layout.widgets.marks,
        bar: &config.layout.bar,
        texts: &config.text.widget,
    };
    run(&look)
}

fn builds() -> serde_json::Value {
    json!([
        {"type": "list", "items": [
            {"title": "#125", "state": "done"},
            {"title": "#126", "state": "done"},
            {"title": "#128", "detail": "main", "right": "3m", "state": "doing", "action": {"call": "build.open", "params": {"id": 128}}},
            {"title": "#127", "detail": "pkg-approval", "right": "1h", "state": "failed"},
            {"title": "#129", "state": "todo"},
            {"title": "#130", "state": "todo"}
        ]},
        {"type": "buttons", "items": [
            {"label": "刷新", "action": {"call": "refresh"}},
            {"label": "全部构建", "action": {"open": "all"}}
        ]}
    ])
}

#[test]
fn a_sidebar_block_has_a_title_then_indented_content_and_clickable_rows() {
    let b = crate::widgets::block(&json!({
        "package": "ci", "id": "builds", "place": "sidebar", "source": "CI Watch", "title": "构建",
        "running": true, "content": builds()
    }))
    .expect("读得出");
    let drawn = with_look(Mode::Sidebar, 36, |look| block(&b, look, None));
    assert_eq!(
        texts(&drawn.lines),
        [
            "构建 · CI Watch",
            "  ☑ #125",
            "  ☑ #126",
            "  ■ #128 · main                   3m",
            "  ✗ #127 · pkg-approval           1h",
            "  ☐ #129",
            "  ☐ #130",
            "  刷新  全部构建",
        ]
    );
    assert!(!drawn.cut);
    let call = &drawn.hits[0];
    assert_eq!((call.row, call.from, call.to), (3, 2, 36), "能点的整行");
    assert_eq!(
        call.action,
        Action::Call {
            method: "build.open".into(),
            params: json!({"id": 128})
        }
    );
    let buttons: Vec<_> = drawn.hits[1..]
        .iter()
        .map(|h| (h.row, h.from, h.to))
        .collect();
    assert_eq!(buttons, [(7, 2, 6), (7, 8, 16)], "按钮各是各的那几列");
}

#[test]
fn a_crowded_list_folds_like_the_todo_list() {
    let got = with_look(Mode::Home, 40, |look| {
        content(&segments(builds()), look, Some(4))
    });
    assert_eq!(
        texts(&got.lines),
        [
            "■ #128 · main                         3m",
            "✗ #127 · pkg-approval                 1h",
            "… 还有 2 项",
            "☑ 已完成 2 项"
        ],
        "做完的收成最底下一行（2026-10-18 项目主人：同待办），放不下的收成它上面一行；按钮整段放不下不画"
    );
    assert!(got.cut);
    assert_eq!(got.hits.len(), 1, "收起来的那几行不能点");
    assert_eq!(got.hits[0].row, 0, "能点的那一行跟着排到最上面");
}

#[test]
fn long_text_is_cut_with_an_ellipsis_and_later_segments_are_left_out() {
    let s = segments(json!([
        {"type": "text", "text": "一二三四五六七八九十一二三四五六七八九十"},
        {"type": "heading", "text": "后面的"}
    ]));
    let got = with_look(Mode::Home, 10, |look| content(&s, look, Some(1)));
    assert_eq!(texts(&got.lines), ["一二三四…"]);
    assert!(got.cut);
}

#[test]
fn pairs_line_up_and_progress_marks_its_line() {
    let s = segments(json!([
        {"type": "pairs", "items": [{"label": "湿度", "value": "40%"}, {"label": "风", "value": "东南 3 级", "tone": "warn"}]},
        {"type": "progress", "value": 0.5, "text": "索引 50%", "mark": 0.8},
        {"type": "progress", "value": 0.25}
    ]));
    let got = with_look(Mode::Home, 30, |look| content(&s, look, None));
    let lines = texts(&got.lines);
    assert_eq!(lines[0], "湿度  40%");
    assert_eq!(lines[1], "风    东南 3 级");
    assert_eq!(lines[2], "索引 50%");
    assert_eq!(lines[3], "▰▰▰▰▰▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱", "画法同上下文那根，二十格");
    assert_eq!(
        got.lines[3].spans[16].style,
        crate::theme::warn(),
        "标记落在的那一格警示色"
    );
    assert_eq!(lines[4], "▰▰▰▰▰▱▱▱▱▱▱▱▱▱▱▱▱▱▱▱ 25%", "没写字的后面接百分比");
}

#[test]
fn normal_text_is_dim_in_the_sidebar_only() {
    let s = segments(json!([{"type": "text", "text": "晴"}]));
    let side = with_look(Mode::Sidebar, 20, |look| content(&s, look, None));
    let home = with_look(Mode::Home, 20, |look| content(&s, look, None));
    // 画出来的样子：行的样子叠上这一段的。
    let shown = |line: &Line, i: usize| line.style.patch(line.spans[i].style);
    assert_eq!(shown(&side.lines[0], 1), crate::theme::dim());
    assert_eq!(shown(&home.lines[0], 0), ratatui::style::Style::new());
}

#[test]
fn toned_runs_on_the_right_keep_their_own_colors() {
    let s = segments(json!([{"type": "list", "items": [
        {"title": "b.rs", "right": [{"text": "+12", "tone": "good"}, {"text": "−3", "tone": "bad"}]},
        {"title": "c.rs", "right": "3m"}
    ]}]));
    let got = with_look(Mode::Home, 30, |look| content(&s, look, None));
    let lines = texts(&got.lines);
    assert!(lines[0].ends_with("+12 −3"), "{lines:?}");
    let style_of = |line: &Line, text: &str| {
        line.spans
            .iter()
            .find(|s| s.content == text)
            .map(|s| s.style)
    };
    assert_eq!(style_of(&got.lines[0], "+12"), Some(crate::theme::good()));
    assert_eq!(style_of(&got.lines[0], "−3"), Some(crate::theme::error()));
    assert_eq!(
        style_of(&got.lines[1], "3m"),
        Some(crate::theme::dim()),
        "一句字的照旧暗"
    );
}
