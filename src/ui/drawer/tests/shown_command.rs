//! 确认抽屉里的命令画到屏上以后，要和真正要跑的一致（发布前审查 2026-10-30）：画面库把控制字符（制表符也是）整个
//! 丢掉，参数之间只隔一个制表符的命令在屏上连成一个词。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};

use super::{look, texts, view};
use crate::drawer::Drawer;

/// 照 `draw` 的画法（一个 `Paragraph`）画进格子，读回屏上看到的每一行。
fn screen(d: &Drawer, width: u16) -> Vec<String> {
    let v = view(d, &texts(), &look(), width, false);
    let area = Rect::new(0, 0, width, u16::try_from(v.lines.len()).unwrap());
    let mut buf = Buffer::empty(area);
    Paragraph::new(v.lines).render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            let row: String = (0..width).map(|x| buf[(x, y)].symbol()).collect();
            row.trim_end().to_string()
        })
        .collect()
}

#[test]
fn a_tab_between_arguments_still_separates_them_on_screen() {
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "execute", "detail": {"tool": "shell"}
    }))
    .unwrap();
    let mut d = Drawer::approval(None, approval);
    d.command = Some((None, "rm\t-rf\t~/build".into()));
    let shown = screen(&d, 60);
    let at = shown
        .iter()
        .find(|l| l.contains("$ rm"))
        .unwrap_or_else(|| panic!("{shown:#?}"));
    assert!(
        !at.contains("rm-rf"),
        "屏上的命令和要跑的不一样（shell 照制表符分参数）：{at:?}"
    );
}
