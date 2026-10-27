//! 悬浮窗滚着露（蓝图「配置页」第 14 条）。

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::text::Line;

use super::{Body, Lines, centered};
use crate::caret::Caret;

/// 20 行、选中第 `focus` 行的窗，接着上一帧的 `top` 画一次：交回窗里第一行写的是哪一行、这一帧的 `top`。
fn draw(focus: usize, top: usize) -> (String, usize) {
    let mut lines: Lines<()> = Lines {
        top,
        ..Lines::default()
    };
    for i in 0..20 {
        lines.body.push(Body {
            line: Line::from(format!("row{i:02}")),
            focused: i == focus,
            hit: None,
        });
    }
    let mut term = Terminal::new(TestBackend::new(40, 14)).unwrap();
    term.draw(|frame| {
        let area = frame.area();
        centered(
            frame,
            area,
            (30, 12),
            &mut lines,
            (&mut Vec::new(), |_| ()),
            &mut Caret::default(),
        );
    })
    .unwrap();
    let buf = term.backend().buffer();
    let text: Vec<String> = (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
        .collect();
    let first = text
        .iter()
        .find_map(|l| l.find("row").map(|at| l[at..at + 5].to_string()))
        .unwrap_or_default();
    (first, lines.top)
}

#[test]
fn going_back_up_in_a_long_window_does_not_scroll_until_the_top_edge() {
    // 2026-10-27 项目主人（模型池的窗）：往下到底以后往回一下就整页往上翻，要碰到上边才翻。
    let mut top = 0;
    for focus in 0..20 {
        (_, top) = draw(focus, top);
    }
    let (bottom, _) = draw(19, top);
    assert_ne!(bottom, "row00", "放不下才有这一条");
    let low: usize = bottom[3..].parse().unwrap();
    for focus in (low..19).rev() {
        let (first, now) = draw(focus, top);
        assert_eq!(first, bottom, "往回第 {focus} 行：窗里不挪");
        top = now;
    }
    let (first, _) = draw(low - 1, top);
    assert_eq!(first, format!("row{:02}", low - 1), "碰到上边才挪");
}
