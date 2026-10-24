//! 注册机主那一步（「第一次打开的引导」第 5a 条）：用户名、密码、再输一遍三格，下面「注册」；不合的地方红字写在那一格
//! 下面，交了以后「注册」那一行换成转圈的「正在注册…」。

use unicode_width::UnicodeWidthStr;

use super::content::Content;
use crate::oobe::Texts;
use crate::oobe::register::{Problem, RegisterStep, Slot};
use crate::theme;

/// 三格和「注册」。`spinner` 是转圈这一拍的字。
pub fn content(step: &RegisterStep, texts: &Texts, width: u16, spinner: &str) -> Content {
    let words = &texts.register;
    let mut c = Content::new(width);
    c.cue = texts.keys.enter_edit.clone();
    c.heading(&words.head.title, &words.head.sub);
    let label_w = words.fields.iter().map(|f| f.width()).max().unwrap_or(0);
    let on = |slot| !step.busy && step.focus == slot;
    let problem = step.problem.as_ref().map(|p| (p.slot(), said(p, texts)));
    let rows = [
        (Slot::User, &step.user),
        (Slot::Password, &step.password),
        (Slot::Again, &step.again),
    ];
    for (i, (slot, field)) in rows.into_iter().enumerate() {
        if i > 0 {
            c.blank();
        }
        c.field(&words.fields[i], label_w, field, &words.hints[i], on(slot));
        if let Some((_, text)) = problem.as_ref().filter(|(at, _)| *at == slot) {
            c.note(text, theme::error());
        }
    }
    c.blank();
    if step.busy {
        c.note(&format!("{spinner} {}", words.busy), theme::dim());
    } else {
        c.action(
            on(Slot::Submit),
            &format!("{} {}", words.submit, super::next_mark()),
        );
    }
    c
}

/// 不合的地方写成的一句。
fn said(problem: &Problem, texts: &Texts) -> String {
    let words = &texts.register.problems;
    match problem {
        Problem::Username => words.username.clone(),
        Problem::Short => words.short.clone(),
        Problem::Blank => words.blank.clone(),
        Problem::Mismatch => words.mismatch.clone(),
        Problem::Taken => words.taken.clone(),
        Problem::Said(text) => text.clone(),
    }
}
