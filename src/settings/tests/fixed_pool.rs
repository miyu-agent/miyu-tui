//! 固定的三个池（核心 Q-1：`lite`、`normal`、`flagship` 配置里一个字没写也在）不能删，成员、分法照旧能改。

use ratatui::crossterm::event::KeyCode;

use super::{opened, press};
use crate::settings::Tone;
use crate::settings::popup::Popup;

#[test]
fn a_fixed_pool_cannot_be_deleted_but_still_opens_for_editing() {
    let mut page = opened(false);
    page.nav.page_step(true);
    page.nav.page_step(true);
    page.nav.vertical(1, &page.view);
    assert_eq!(
        page.nav.pool(&page.view).map(|p| p.name.as_str()),
        Some("lite")
    );
    press(&mut page, KeyCode::Char('d'));
    assert!(page.popup.is_none(), "不问删不删");
    let (text, tone) = page.status.clone().expect("说一句");
    assert_eq!(tone, Tone::Note);
    assert!(text.contains("无法删除"), "{text}");
    assert!(page.view.pool("lite").is_some(), "还在");
    press(&mut page, KeyCode::Enter);
    assert!(
        matches!(page.popup, Some(Popup::Form(_))),
        "照旧开编辑窗改成员、分法"
    );
}
