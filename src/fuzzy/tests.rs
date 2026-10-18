//! 模糊搜索：照先后、不必挨着、不分大小写、空格分段，越挨着、越在词头越靠前。

use super::{hit, rank, score};

#[test]
fn letters_in_order_match_even_with_gaps_and_any_case() {
    assert!(hit("opus4", ["anthropic/claude-opus-4"]), "不必挨着");
    assert!(hit("OPUS", ["claude-opus-4"]), "不分大小写");
    assert!(hit("dsflash", ["deepseek-v4.1-flash"]));
    assert!(!hit("4opus", ["claude-opus-4"]), "照先后");
    assert!(hit("", ["什么都行"]), "空的什么都对得上");
}

#[test]
fn spaced_terms_must_all_match_and_may_land_in_different_fields() {
    assert!(hit("cl op", ["claude-opus-4"]));
    assert!(hit(
        "opus anthropic",
        ["Claude Opus 4", "anthropic/claude-opus-4"]
    ));
    assert!(!hit("opus gemini", ["claude-opus-4"]), "每段都要对上");
}

#[test]
fn chinese_matches_character_by_character() {
    assert!(hit("英伟达崩溃", ["英伟达驱动崩溃搜索"]));
    assert!(
        hit("崩溃英伟达", ["英伟达驱动崩溃搜索"]),
        "整截出现的先后不限（2026-10-19）"
    );
}

#[test]
fn closer_and_word_start_matches_rank_first_and_ties_keep_their_order() {
    let all = vec![
        "openai/gpt-4o",
        "anthropic/claude-opus-4",
        "opus-mini",
        "deepseek/opus",
    ];
    let got = rank("opus", all, |s| vec![*s]);
    assert_eq!(got[0], "opus-mini", "开头就是");
    assert!(
        !got.contains(&"openai/gpt-4o"),
        "o、p、u、s 照先后对不上的拿掉"
    );
    let got = rank("", vec!["b", "a"], |s| vec![*s]);
    assert_eq!(got, ["b", "a"], "空的原样");
    assert!(
        score("flash", ["deepseek-v4.1-flash"]) > score("flash", ["f-l-a-s-h"]),
        "挨着的比散着的分高"
    );
}

#[test]
fn whole_pieces_match_in_any_order_but_rank_after_ordered_matches() {
    // 2026-10-19 项目主人：`clinedeepseek` 搜得到，`deepseekcline` 搜不到。
    let model = "clinepass/deepseek-v4.1-flash";
    assert!(hit("deepseekcline", [model]));
    assert!(hit("flashcline", [model]));
    assert!(
        score("clinedeepseek", [model]) > score("deepseekcline", [model]),
        "照先后对上的分高"
    );
    assert!(!hit("deepseekcline", ["deepseek-v4.1-flash"]), "每截都要有");
    assert!(!hit("kees", ["deepseek"]), "一个字一个字乱序的不算");
    let ranked = rank("deepseekcline", vec!["deepseek-v4.1-flash", model], |s| {
        vec![*s]
    });
    assert_eq!(ranked, [model]);
}
