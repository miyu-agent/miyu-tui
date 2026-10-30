use super::{expand, shown};

#[test]
fn tabs_go_to_the_next_stop_counted_from_each_line_start() {
    assert_eq!(expand("\tx", 4), "    x");
    assert_eq!(expand("ab\tc", 4), "ab  c");
    assert_eq!(expand("abcd\te", 4), "abcd    e");
    assert_eq!(expand("中\tx", 4), "中  x", "宽字算两列");
    assert_eq!(expand("a\n\tb", 4), "a\n    b", "换行以后从头数");
}

#[test]
fn other_control_characters_are_dropped_and_plain_text_is_borrowed() {
    assert_eq!(expand("a\rb\u{7}c", 4), "abc");
    assert!(matches!(
        expand("plain\ntext", 4),
        std::borrow::Cow::Borrowed(_)
    ));
}

/// 给人照着批的命令（发布前审查 T2-2）：看不见的字换成看得见的记号，不删。
#[test]
fn a_command_shows_every_hidden_character() {
    assert_eq!(
        shown("rm\t-rf\t~/build"),
        "rm      -rf     ~/build",
        "照 8 列的制表位"
    );
    assert_eq!(shown("a\u{1b}[31mb"), "a^[[31mb");
    assert_eq!(shown("x\ry\u{7}z\u{7f}"), "x^My^Gz^?");
    assert_eq!(shown("ls \u{202e}fdp.exe"), "ls <U+202E>fdp.exe");
    assert_eq!(shown("a\u{85}b\u{2066}c"), "a<U+0085>b<U+2066>c");
    assert_eq!(shown("一\n\tb"), "一\n        b", "换行照旧、以后从头数");
    assert!(matches!(shown("echo hi"), std::borrow::Cow::Borrowed(_)));
}

/// 外来的字里 kitty 的图格字和国旗以外的标签字符去掉（发布前审查 T2-4）；国旗照留。
#[test]
fn placeholders_and_stray_tags_are_dropped_but_flags_stay() {
    assert_eq!(
        expand("看\u{10EEEE}\u{305}\u{30d}这里", 4),
        "看这里",
        "连同写行列的附加符"
    );
    assert_eq!(expand("a\u{E0041}\u{E0042}b", 4), "ab");
    let scotland = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}";
    assert_eq!(
        expand(&format!("旗{scotland}x\u{E0041}"), 4),
        format!("旗{scotland}x")
    );
}
