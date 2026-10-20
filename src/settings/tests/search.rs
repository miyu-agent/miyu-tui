//! 配置页里的 `Ctrl+J`、`Ctrl+K` 和搜（蓝图「按键」表 `Ctrl+J`、`Ctrl+K` 那一行和「按键模式」那一行）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{opened, press, texts};
use crate::settings::popup::Popup;

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 开页、按 `/` 进这一栏的搜，再按这几个键：交回搜的字、选中第几个。
fn searched(keys: &[KeyEvent]) -> (String, usize) {
    let mut page = opened(false);
    press(&mut page, KeyCode::Char('/'));
    for key in keys {
        page.key(*key, &texts());
    }
    let search = page.nav.search.clone().unwrap();
    let col = search.col.unwrap();
    (search.text, page.nav.selected(col))
}

/// 开选默认模型的窗、按 `/`，再按这几个键：交回搜的字、选中第几行。
fn picked(keys: &[KeyEvent]) -> (String, usize) {
    let mut page = opened(false);
    press(&mut page, KeyCode::Char('.'));
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Char('/'));
    for key in keys {
        page.key(*key, &texts());
    }
    let Some(Popup::Pick(pick)) = &page.popup else {
        panic!("选择窗开着")
    };
    (pick.search.clone(), pick.sel)
}

#[test]
fn ctrl_j_and_k_move_like_the_arrows_on_the_settings_page() {
    // 2026-10-21 项目主人：「统一成 jk，ctrl+jk 都能用」（原来配置页带 Ctrl 的一律不管，按了没反应）。
    let down = plain(KeyCode::Down);
    let up = plain(KeyCode::Up);
    assert_ne!(searched(&[down]), searched(&[]), "这一栏不止一行，↓ 挪得动");
    assert_eq!(searched(&[ctrl('j')]), searched(&[down]));
    assert_eq!(searched(&[down, ctrl('k')]), searched(&[down, up]));
    assert_ne!(picked(&[down]), picked(&[]), "窗里不止一行，↓ 挪得动");
    assert_eq!(picked(&[ctrl('j')]), picked(&[down]));
    assert_eq!(picked(&[down, ctrl('k')]), picked(&[down, up]));
}

#[test]
fn a_second_slash_in_the_pick_window_starts_over() {
    // 2026-10-21 项目主人：「下次再 / 就清空搜索结果后再次进入搜索模式」。
    let mut page = opened(false);
    press(&mut page, KeyCode::Char('.'));
    press(&mut page, KeyCode::Enter);
    press(&mut page, KeyCode::Char('/'));
    press(&mut page, KeyCode::Char('f'));
    press(&mut page, KeyCode::Enter);
    let Some(Popup::Pick(pick)) = &page.popup else {
        panic!("选择窗开着")
    };
    assert_eq!(pick.search, "f");
    press(&mut page, KeyCode::Char('/'));
    let Some(Popup::Pick(pick)) = &page.popup else {
        panic!("选择窗开着")
    };
    assert!(pick.typing && pick.search.is_empty(), "再按 / 清空了重打");
}
