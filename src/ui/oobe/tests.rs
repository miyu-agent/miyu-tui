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

#[test]
fn the_semantic_step_shows_the_size_the_bar_and_a_failure() {
    // 2026-10-19（第 26b 条）：没装的「是」后面写要下多大；在下的换成进度条；装不上的写原话、两行「重试」「跳过」。
    use crate::oobe::Oobe;
    use crate::oobe::pick::Pick;
    use crate::oobe::semantic::download::{Offer, Phase};
    let config = Config::builtin().unwrap();
    let texts = &config.text.oobe;
    let bar = &config.layout.bar;
    let mut oobe = Oobe::new(
        std::time::Instant::now(),
        config.oobe.clone(),
        Pick::new(vec!["中文".into()], 0),
        "nerd",
        true,
    );
    oobe.semantic.installed = Some(false);
    oobe.semantic.offer = Some(Offer::Download(Some(59_768_832)));
    let c = super::pick::semantic(&oobe, texts, bar, 60);
    assert!(says(&c, "推荐 · 需下载 57.0 MiB"), "{:?}", c.lines);
    oobe.semantic.offer = Some(Offer::Download(None));
    let c = super::pick::semantic(&oobe, texts, bar, 60);
    assert!(says(&c, "推荐") && !says(&c, "需下载"), "连不上的不写大小");
    oobe.semantic.phase = Phase::Installing { done: 0, total: 0 };
    let c = super::pick::semantic(&oobe, texts, bar, 60);
    assert!(says(&c, "准备下载…") && !says(&c, "推荐"));
    oobe.semantic.phase = Phase::Installing {
        done: 29_884_416,
        total: 59_768_832,
    };
    let c = super::pick::semantic(&oobe, texts, bar, 60);
    assert!(says(&c, "50% · 28.5 MiB / 57.0 MiB"), "{:?}", c.lines);
    assert!(
        says(&c, &bar.full) && says(&c, &bar.empty),
        "半条满、半条空"
    );
    oobe.semantic.phase = Phase::Failed {
        said: "连不上软件源".into(),
        why: Some("timed out".into()),
    };
    let c = super::pick::semantic(&oobe, texts, bar, 60);
    assert!(says(&c, "连不上软件源") && says(&c, "timed out"));
    assert!(says(&c, "重试") && says(&c, "跳过") && !says(&c, "推荐"));
}
