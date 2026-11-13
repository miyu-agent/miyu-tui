//! 三页都能 `/` 筛（2026-11-13 主人 1113 初验第 11 条：终端软件包页只有「下载软件」认 `/`）：打字筛、`Enter` 收起留着、
//! `Esc` 不筛，换了页筛的字不带过去。

use ratatui::crossterm::event::KeyCode;

use super::{opened, press};
use crate::settings::pages::store::Tab;

#[test]
fn the_installed_and_backstage_pages_search_with_slash_too() {
    let mut page = opened();
    page.more.store.tab = Tab::Installed;
    assert_eq!(page.more.store.rows().len(), 2);
    press(&mut page, KeyCode::Char('/'));
    assert!(page.more.store.typing, "/ 进打字");
    press(&mut page, KeyCode::Char('m'));
    press(&mut page, KeyCode::Char('e'));
    press(&mut page, KeyCode::Enter);
    let ids: Vec<String> = page
        .more
        .store
        .rows()
        .iter()
        .map(|r| r.id().to_string())
        .collect();
    assert_eq!(ids, ["memes"], "筛出来的留着");
    assert!(!page.more.store.typing);
    // 换页：筛的字不带过去
    press(&mut page, KeyCode::Char('.'));
    assert_eq!(page.more.store.tab, Tab::Backstage);
    assert!(page.more.store.query.is_empty());
    press(&mut page, KeyCode::Char('/'));
    assert!(page.more.store.typing, "软件后台也认 /");
    press(&mut page, KeyCode::Esc);
    assert!(!page.more.store.typing && page.more.store.query.is_empty());
}
