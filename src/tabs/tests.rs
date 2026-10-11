use super::expand;

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
