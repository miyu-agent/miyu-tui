//! 引导左边那一块画什么（「第一次打开的引导」第 11a 条，2026-10-15 项目主人）：人格那一步以前画吉祥物；有头像的画头像；
//! 主题那一步选了吉祥物不显示的什么都不画，退回上一步（没写下）又画吉祥物。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;

use super::theme::{at_theme, defaults};
use super::{fresh, key, texts};
use crate::oobe::figure::Shown;
use crate::oobe::theme::Slot;
use crate::oobe::{Move, Step};

#[test]
fn what_the_left_side_shows_follows_the_steps_the_avatar_and_the_mascot_switch() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Model;
    assert_eq!(
        oobe.wanted_figure(true),
        Shown::Mascot,
        "人格那一步以前画吉祥物"
    );
    oobe.step = Step::Persona;
    assert_eq!(
        oobe.wanted_figure(true),
        Shown::Avatar,
        "有头像的当场换成头像"
    );
    assert_eq!(oobe.wanted_figure(false), Shown::Mascot);
    at_theme(&mut oobe, t0, &texts, defaults());
    // 挪到吉祥物那一格，选不显示：当场不画。
    for _ in 0..3 {
        oobe.key(key(KeyCode::Down), t0, &texts);
    }
    assert_eq!(oobe.theme.slot, Slot::Mascot);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Down), t0, &texts);
    assert_eq!(
        oobe.wanted_figure(false),
        Shown::Nothing,
        "挪到不显示就收掉"
    );
    // 2026-10-29 项目主人：关了形象，头像也不画。
    assert_eq!(oobe.wanted_figure(true), Shown::Nothing, "有头像也不画");
    oobe.key(key(KeyCode::Enter), t0, &texts);
    // 退回上一步：没写下，吉祥物回来。
    oobe.moved(Move::Back, t0);
    assert_eq!(oobe.step, Step::Persona);
    assert_eq!(oobe.wanted_figure(false), Shown::Mascot);
}

#[test]
fn turning_the_figure_off_drops_the_left_column_after_the_color_step() {
    // 2026-10-29 项目主人：选了隐藏以后，左边原本属于吉祥物的位置应该去掉；配色方案那一步留着看它走出去。
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.step = Step::Profile;
    assert!(oobe.figure_column(), "没选不显示的留着");
    oobe.theme.mascot_off = true;
    assert!(oobe.figure_column(), "还没写下的留着");
    oobe.theme.saved = true;
    assert!(!oobe.figure_column(), "写下了：后面几步去掉");
    oobe.step = Step::Theme;
    assert!(oobe.figure_column(), "配色方案那一步留着");
}
