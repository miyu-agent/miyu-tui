//! 引导几屏画出来的字（2026-10-19 项目主人，同网页）：什么都没填的「下一步」写成「跳过」；好了那一屏不写跳过的人格。

use crate::config::Config;
use crate::oobe::field::Field;
use crate::oobe::persona::PersonaStep;
use crate::oobe::profile::ProfileStep;
use crate::oobe::{Summary, Texts};

fn texts() -> Texts {
    Config::builtin().unwrap().text.oobe
}

/// 一块里有没有这几个字。
fn says(content: &super::content::Content, words: &str) -> bool {
    content.lines.iter().any(|l| l.to_string().contains(words))
}

#[test]
fn an_empty_step_says_skip_instead_of_next() {
    let texts = texts();
    let mut persona = PersonaStep::default();
    assert!(says(
        &super::persona::content(&persona, &texts, 60, 6),
        "跳过 →"
    ));
    persona.name = Field::line().with("小雪");
    assert!(says(
        &super::persona::content(&persona, &texts, 60, 6),
        "下一步 →"
    ));
    let mut profile = ProfileStep::default();
    assert!(says(
        &super::profile::content(&profile, &texts, 60),
        "跳过 →"
    ));
    profile.about = Field::lines().with("喜欢猫");
    assert!(says(
        &super::profile::content(&profile, &texts, 60),
        "下一步 →"
    ));
}

#[test]
fn the_last_screen_leaves_out_a_skipped_persona() {
    let texts = texts();
    let summary = Summary {
        model: "deepseek-flash".into(),
        persona: String::new(),
        preset: "全部功能".into(),
    };
    let line = super::screens::summary_line(&texts.done.summary, &summary);
    assert_eq!(line, "模型 deepseek-flash · 预设 全部功能");
    let summary = Summary {
        persona: "小雪".into(),
        ..summary
    };
    let line = super::screens::summary_line(&texts.done.summary, &summary);
    assert_eq!(line, "模型 deepseek-flash · 人格 小雪 · 预设 全部功能");
}
