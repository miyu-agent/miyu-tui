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

#[test]
fn marks_point_at_the_matched_letters_in_each_field() {
    use super::marks;
    // 照先后对上：`open` 落在 `opencode-zen/test` 开头四个字。
    assert_eq!(marks("open", "opencode-zen/test"), [0, 1, 2, 3]);
    // 两段各自对上，可以落在不同的地方。
    assert_eq!(marks("cl op", "Claude Opus 5.5"), [0, 1, 7, 8]);
    // 这个字段对不上的段不标。
    assert!(marks("zzz", "Claude").is_empty());
    // 切成几截才对上的：每截原样出现的那一处。
    assert_eq!(
        marks("clinedeepseek", "clinepass/deepseek-v4"),
        [0, 1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17]
    );
    assert!(marks("", "anything").is_empty());
}

#[test]
fn punctuation_in_what_is_typed_is_skipped() {
    // 2026-11-10 项目主人：搜 `opus5.5` 找不到 `claude-opus-5-5`（同 im I-191、网页 W-1142）。
    let model = "claude-opus-5-5";
    for typed in ["opus5.5", "OPUS 5-5", "opus55", "opus-5.5"] {
        assert!(hit(typed, [model]), "{typed}");
    }
    assert!(hit("- .", ["anything"]), "只有标点的一段不算");
    assert!(!hit("opus5.6", [model]), "字母、数字照旧要对上");
    // 只隔着标点的不扣分：比隔着字母的分高，比字段里本来挨着的低（`f-l-a-s-h` 不排到 `flash` 前面）。
    assert!(score("55", ["x5-5"]) > score("55", ["x5a5"]));
    assert!(score("flash", ["deepseek-v4.1-flash"]) > score("flash", ["f-l-a-s-h"]));
}

#[test]
fn marks_skip_punctuation_and_word_starts_still_count() {
    use super::marks;
    // `opus5.5` 标在 o、p、u、s 和两个 5 上，不标横线。
    assert_eq!(marks("opus5.5", "claude-opus-5-5"), [7, 8, 9, 10, 12, 14]);
    // 词头分照旧：横线后面的 5 算词头，`claude-opus-5-5` 排在字母挤在一起、没有词头的前面。
    let ranked = rank("opus55", vec!["xopusx55y", "claude-opus-5-5"], |s| vec![*s]);
    assert_eq!(ranked[0], "claude-opus-5-5", "落在词头的排前面");
}
