use super::dollars;

#[test]
fn brackets_and_parens_become_dollars() {
    assert_eq!(dollars("面积 \\( \\pi r^2 \\) 是"), "面积 $\\pi r^2$ 是");
    assert_eq!(dollars("\\[\nE = mc^2\n\\]"), "$$\nE = mc^2\n$$");
    assert_eq!(dollars("前面 \\[a+b\\] 后面"), "前面 $$a+b$$ 后面");
}

#[test]
fn code_escapes_and_unfinished_ones_are_left_alone() {
    let fenced = "```latex\n\\[x\\]\n```\n";
    assert_eq!(dollars(fenced), fenced, "代码块里的不动");
    let tilde = "~~~\n\\(x\\)\n~~~\n";
    assert_eq!(dollars(tilde), tilde, "波浪线围的代码块也不动");
    assert_eq!(
        dollars("写 `\\(x\\)` 这样"),
        "写 `\\(x\\)` 这样",
        "行内代码不动"
    );
    assert_eq!(dollars("还在流 \\[ a +"), "还在流 \\[ a +", "没收尾的不换");
    assert_eq!(dollars("\\\\(x\\\\)"), "\\\\(x\\\\)", "转义的反斜杠不算");
    assert_eq!(dollars("没有公式"), "没有公式");
    let after = "```\ncode\n```\n然后 \\(x\\)";
    assert_eq!(
        dollars(after),
        "```\ncode\n```\n然后 $x$",
        "代码块后面的照换"
    );
}
