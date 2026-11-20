//! 配置页的一页滚着露（蓝图「配置页」）：选中的碰到上下边才滚。

use ratatui::{Terminal, backend::TestBackend};

#[test]
fn a_settings_page_scrolls_only_when_the_selection_hits_an_edge() {
    // 2026-10-29 项目主人：触底往下滚以后，往上挪一行立刻往上滚，要碰到上边才滚。20 行露 5 行。
    let lines = |focus: usize| -> Vec<super::super::pages::Painted> {
        (0..20)
            .map(|i| (ratatui::text::Line::raw(format!("项{i}")), i == focus))
            .collect()
    };
    let mut term = Terminal::new(TestBackend::new(20, 5)).unwrap();
    let mut top = 0;
    let mut at = |focus: usize, top: &mut usize| {
        term.draw(|f| super::super::pages::paint(f, f.area(), &lines(focus), top))
            .unwrap();
        *top
    };
    for focus in 0..10 {
        at(focus, &mut top);
    }
    assert_eq!(top, 5, "往下碰到下边才滚");
    assert_eq!(at(8, &mut top), 5, "往上挪一行不滚");
    assert_eq!(at(5, &mut top), 5, "还在窗里不滚");
    assert_eq!(at(4, &mut top), 4, "碰到上边才滚");
}
