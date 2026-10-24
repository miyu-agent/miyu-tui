//! 注册机主那一步（「第一次打开的引导」第 5a 条，2026-10-24 项目主人定，核心 Y-12 中上 `account.setup`）。

use std::time::Instant;

use ratatui::crossterm::event::KeyCode;
use serde_json::json;

use super::{asks, find, fresh, key, texts, typed};
use crate::core::Refusal;
use crate::oobe::register::{Problem, Slot, good_username};
use crate::oobe::{Oobe, Step, Texts};

/// 还没有机主：走到注册那一步。
fn at_register(t0: Instant, texts: &Texts) -> Oobe {
    let mut oobe = fresh(t0);
    oobe.account(true, t0);
    oobe.key(key(KeyCode::Enter), t0, texts);
    assert_eq!(oobe.step, Step::Language);
    oobe.key(key(KeyCode::Enter), t0, texts);
    assert_eq!(oobe.step, Step::Register, "语言后面是注册");
    oobe
}

/// 光标挪到 `slot` 那一格，`Enter` 开始编辑，换成 `text`，`Enter` 写好。
fn fill(oobe: &mut Oobe, slot: Slot, text: &str, t0: Instant, texts: &Texts) {
    oobe.register.focus = slot;
    oobe.key(key(KeyCode::Enter), t0, texts);
    for _ in 0..64 {
        oobe.key(key(KeyCode::Backspace), t0, texts);
    }
    typed(oobe, text, t0, texts);
    oobe.key(key(KeyCode::Enter), t0, texts);
}

/// 光标挪到「注册」回车。
fn submit(oobe: &mut Oobe, t0: Instant, texts: &Texts) {
    oobe.register.focus = Slot::Submit;
    oobe.key(key(KeyCode::Enter), t0, texts);
}

#[test]
fn usernames_follow_the_account_id_rules() {
    for good in ["alice", "a", "bob-2", "x_y", &"a".repeat(32)] {
        assert!(good_username(good), "{good}");
    }
    for bad in [
        "",
        "Alice",
        "2bob",
        "-a",
        "名字",
        "a b",
        "a.b",
        &"a".repeat(33),
    ] {
        assert!(!good_username(bad), "{bad}");
    }
}

#[test]
fn with_an_owner_language_goes_straight_to_controls() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = fresh(t0);
    oobe.account(false, t0);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    oobe.key(key(KeyCode::Enter), t0, &texts);
    assert_eq!(oobe.step, Step::Keys, "有机主的没有注册这一步");
}

#[test]
fn the_form_is_checked_before_anything_is_sent() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_register(t0, &texts);
    let cases = [
        (
            "",
            "longenough",
            "longenough",
            Problem::Username,
            Slot::User,
        ),
        (
            "Alice",
            "longenough",
            "longenough",
            Problem::Username,
            Slot::User,
        ),
        ("alice", "short", "short", Problem::Short, Slot::Password),
        (
            "alice",
            "         ",
            "         ",
            Problem::Blank,
            Slot::Password,
        ),
        (
            "alice",
            "longenough",
            "longenougH",
            Problem::Mismatch,
            Slot::Again,
        ),
    ];
    for (user, password, again, problem, slot) in cases {
        fill(&mut oobe, Slot::User, user, t0, &texts);
        fill(&mut oobe, Slot::Password, password, t0, &texts);
        fill(&mut oobe, Slot::Again, again, t0, &texts);
        submit(&mut oobe, t0, &texts);
        assert_eq!(
            oobe.register.problem,
            Some(problem.clone()),
            "{user} {password}"
        );
        assert_eq!(oobe.register.focus, slot, "光标挪到不合的那一格");
        assert!(asks(&mut oobe).is_empty(), "不合的不发：{problem:?}");
    }
    fill(&mut oobe, Slot::Again, "longenough", t0, &texts);
    submit(&mut oobe, t0, &texts);
    let (_, params) = find(&asks(&mut oobe), "account.setup");
    assert_eq!(
        params,
        json!({"username": "alice", "password": "longenough"})
    );
    assert!(oobe.register.busy && oobe.register.problem.is_none());
}

#[test]
fn a_taken_name_is_said_under_the_username_and_the_form_stays() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_register(t0, &texts);
    fill(&mut oobe, Slot::User, "users", t0, &texts);
    fill(&mut oobe, Slot::Password, "longenough", t0, &texts);
    fill(&mut oobe, Slot::Again, "longenough", t0, &texts);
    submit(&mut oobe, t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "account.setup");
    let refused = Refusal {
        reason: Some("username_taken".into()),
        message: "用户名已被使用。".into(),
        data: json!({"reason": "username_taken"}),
    };
    oobe.answer(tag, Err(refused), t0, &texts);
    assert_eq!(oobe.register.problem, Some(Problem::Taken));
    assert_eq!(oobe.register.focus, Slot::User);
    assert!(!oobe.register.busy, "表单留着，能改了再交");
    assert_eq!(oobe.register.password.text(), "longenough");
    assert_eq!(oobe.step, Step::Register);
}

#[test]
fn after_registering_the_next_handshake_moves_on_and_back_skips_it() {
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_register(t0, &texts);
    fill(&mut oobe, Slot::User, "alice", t0, &texts);
    fill(&mut oobe, Slot::Password, "longenough", t0, &texts);
    fill(&mut oobe, Slot::Again, "longenough", t0, &texts);
    submit(&mut oobe, t0, &texts);
    let (tag, _) = find(&asks(&mut oobe), "account.setup");
    oobe.answer(
        tag,
        Ok(json!({"username": "alice", "account": "alice"})),
        t0,
        &texts,
    );
    assert_eq!(oobe.step, Step::Register, "核心重起、重连上以前接着等");
    assert!(oobe.register.busy);
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.step, Step::Register, "等的时候不让走");
    // 重连上，握手说有机主了：往下走，密码不留着。
    oobe.account(false, t0);
    assert_eq!(oobe.step, Step::Keys);
    assert!(oobe.register.password.text().is_empty() && oobe.register.again.text().is_empty());
    // 往回走跨过注册；进度条里还画着走过的「注册」。
    oobe.key(key(KeyCode::Esc), t0, &texts);
    assert_eq!(oobe.step, Step::Language);
    assert_eq!(
        Step::Keys.bar(oobe.register.needed).map(|(_, now)| now),
        Some(2)
    );
}

#[test]
fn a_handshake_that_already_has_an_owner_moves_on_even_without_the_answer() {
    // 核心答完就断开：回应被断开抢先、照被拒办了的，重连上握手说有机主了照样往下走。
    let texts = texts();
    let t0 = Instant::now();
    let mut oobe = at_register(t0, &texts);
    fill(&mut oobe, Slot::User, "alice", t0, &texts);
    fill(&mut oobe, Slot::Password, "longenough", t0, &texts);
    fill(&mut oobe, Slot::Again, "longenough", t0, &texts);
    submit(&mut oobe, t0, &texts);
    drop(asks(&mut oobe));
    oobe.offline("核心断开连接", t0, &texts);
    assert!(!oobe.register.busy && oobe.register.problem.is_some());
    oobe.account(false, t0);
    assert_eq!(oobe.step, Step::Keys);
    assert!(oobe.register.problem.is_none());
}
