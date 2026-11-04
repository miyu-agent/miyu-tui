use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Ahead;
use crate::config::Config;

fn press(ahead: &mut Ahead, code: KeyCode, modifiers: KeyModifiers, config: &Config) {
    ahead.key(KeyEvent::new(code, modifiers), config);
}

fn typed(ahead: &mut Ahead, text: &str, config: &Config) {
    for c in text.chars() {
        press(ahead, KeyCode::Char(c), KeyModifiers::NONE, config);
    }
}

#[test]
fn keys_edit_like_the_full_screen_input_box() {
    // 排期第 46 条（2026-11-04 项目主人：光标不能左右挪、不能换行）：左右挪了在中间插字，Shift+回车换行，光标那一格跟着。
    let config = Config::builtin().unwrap();
    let mut ahead = Ahead::default();
    typed(&mut ahead, "ac", &config);
    press(&mut ahead, KeyCode::Left, KeyModifiers::NONE, &config);
    typed(&mut ahead, "b", &config);
    assert_eq!(ahead.take().text, "abc");
    typed(&mut ahead, "一", &config);
    press(&mut ahead, KeyCode::Enter, KeyModifiers::SHIFT, &config);
    typed(&mut ahead, "二", &config);
    let (rows, caret) = ahead.rows(40, 8);
    assert_eq!(rows.len(), 2, "换了行");
    assert_eq!(caret, (1, 2), "光标在第二行「二」后面（中文两列宽）");
    press(&mut ahead, KeyCode::Home, KeyModifiers::NONE, &config);
    assert_eq!(ahead.rows(40, 8).1, (1, 0), "Home 到这一行开头");
}

#[test]
fn a_long_paste_folds_into_a_block_and_goes_out_in_full() {
    // 同全屏：大段粘贴收成「[已粘贴 N 行]」，插话时换回原文。
    let config = Config::builtin().unwrap();
    let mut ahead = Ahead::default();
    let long: String = (1..=30).map(|n| format!("第 {n} 行\n")).collect();
    ahead.paste(&long, &config);
    let draft = ahead.take();
    assert!(draft.text.contains('['), "收成块：{}", draft.text);
    assert_eq!(draft.expand().trim_end(), long.trim_end());
}

#[test]
fn words_handed_back_after_a_withdrawal_come_first_one_per_line() {
    let config = Config::builtin().unwrap();
    let mut ahead = Ahead::default();
    typed(&mut ahead, "接着敲的", &config);
    ahead.put_back(vec!["插的一".into(), "插的二".into()]);
    assert_eq!(ahead.hand_back(), "插的一\n插的二\n接着敲的");
}
