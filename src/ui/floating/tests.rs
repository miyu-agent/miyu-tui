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

#[test]
fn the_buttons_stay_at_the_bottom_with_their_keys_while_a_long_list_scrolls() {
    // 发版前 check 第 30 条（2026-11-09 项目主人）：模型池成员很多时，不挪到最底下也看得到按钮、知道按 s 存。
    let mut lines: Lines<()> = Lines {
        buttons: vec![("保存".into(), false), ("取消".into(), false)],
        keys: vec!["s".into(), "Esc".into()],
        ..Lines::default()
    };
    for i in 0..40 {
        lines.body.push(Body {
            line: Line::from(format!("row{i:02}")),
            focused: i == 0,
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
    let row = text
        .iter()
        // 宽字后面那一格读出来是空格：去掉空格再认。
        .position(|l| {
            let l = l.replace(' ', "");
            l.contains("[保存s]") && l.contains("[取消Esc]")
        })
        .unwrap_or_else(|| panic!("按钮里写着快捷键：\n{}", text.join("\n")));
    assert!(
        text[..row].iter().any(|l| l.contains("row00")),
        "列表停在最上面"
    );
    let y = u16::try_from(row).unwrap();
    let at = |sym: &str| {
        (0..buf.area.width)
            .find(|&x| buf[(x, y)].symbol() == sym)
            .expect("在")
    };
    // 按钮名后面的快捷键暗着写（和名字颜色不同）。
    let key = buf[(at("s"), y)].fg;
    let name = buf[(at("保"), y)].fg;
    assert_ne!(key, name, "快捷键暗着写");
}
