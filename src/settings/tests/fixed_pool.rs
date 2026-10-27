//! 固定的三个池（核心 Q-1：`lite`、`normal`、`flagship` 配置里一个字没写也在）不能删，成员、分法照旧能改。

use ratatui::crossterm::event::KeyCode;

use super::{press, texts};
use crate::settings::popup::Popup;
use crate::settings::test_support::{config_all, model_list};
use crate::settings::{Settings, Tone};

/// 样例里另加一个固定的池 `lite`，进「供应商和模型」。
fn opened() -> Settings {
    let mut list = model_list();
    list["pools"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"name": "lite", "strategy": "pin", "models": [], "fixed": true}));
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => list.clone(),
            "config.get" => config_all(),
            _ => serde_json::json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    press(&mut page, KeyCode::Enter);
    page
}

#[test]
fn a_fixed_pool_cannot_be_deleted_but_still_opens_for_editing() {
    let mut page = opened();
    page.nav.page_step(true);
    page.nav.page_step(true);
    // 固定的池排在所有池最前面（2026-10-26 项目主人，`pools.rs`）。
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
