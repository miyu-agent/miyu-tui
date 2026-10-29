//! 守着主题那张表（蓝图「主题」第 7 条）：画东西的代码里新写了一个记号字面量（框线、箭头、圆点、方块、`…`、` · ` 这类），
//! 没进主题的表，测试就红。中文字、界面上的字（`resources/text`）、图标那一套不算。

use std::path::Path;

/// 不归主题管的几处，各带理由。
const EXEMPT: [(&str, &str); 10] = [
    (
        "src/ui/hint_keys.rs",
        "改写界面上的字里写好的箭头（↑/↓ 换成 j/k），箭头是那份字里的",
    ),
    ("src/sheet/guard.rs", "这道检查自己，例子里有记号"),
    ("src/linebreak.rs", "折行用的标点表，不是画的记号"),
    ("src/crash.rs", "崩溃日志的分隔，不上界面"),
    ("src/ui/rulers.rs", "调试用的量尺"),
    ("src/markdown/math.rs", "公式转出来的字（√ 这些是内容）"),
    ("src/fuzzy.rs", "模糊搜索的分词"),
    ("src/app/goal.rs", "拆核心推来的字（里面的 · 是核心写的）"),
    ("src/ui/oobe/screens.rs", "拆界面上的字的模板（· 在模板里）"),
    ("src/ui/done_row.rs", "照收尾那一行的模板里的 · 拆"),
];

/// 算记号的字：箭头、数学、框线、方块、几何、杂项符号、装饰、盲文，和中点、圆点、省略号、※。
fn is_mark(c: char) -> bool {
    matches!(
        u32::from(c),
        0x2190..=0x2BFF | 0x00B7 | 0x2022 | 0x2026 | 0x203B
    )
}

/// 一行里引号里的字（`"…"` 和 `'…'`），粗略地照引号分，够用：代码里的字面量不跨行。
fn literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if c == '"' || c == '\'' {
            let mut text = String::new();
            let mut escaped = false;
            for d in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if d == '\\' {
                    escaped = true;
                } else if d == c {
                    break;
                }
                text.push(d);
            }
            out.push(text);
        }
    }
    out
}

/// 一个源文件里不在测试里的记号字面量：行号和那一段。
fn marks_in(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        for literal in literals(line) {
            if literal.chars().any(is_mark) {
                out.push((i + 1, literal));
            }
        }
    }
    out
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "tests" {
                walk(&path, root, out);
            }
            continue;
        }
        let skip = !name.ends_with(".rs") || name.contains("test");
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if skip || EXEMPT.iter().any(|(f, _)| *f == rel) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        for (line, literal) in marks_in(&text) {
            out.push(format!("{rel}:{line}: {literal}"));
        }
    }
}

#[test]
fn drawing_code_takes_its_marks_from_the_theme() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    walk(&root.join("src"), root, &mut found);
    assert!(
        found.is_empty(),
        "这些记号写死在代码里，挪进主题的表（packages/miyu-tui-theme-default/theme.json）：\n{}",
        found.join("\n")
    );
}

#[test]
fn the_guard_sees_a_mark_in_a_literal_but_not_in_a_comment() {
    assert_eq!(marks_in("let a = \"❯ \"; // ●"), [(1, "❯ ".to_string())]);
    assert!(marks_in("// let a = \"❯\";").is_empty());
    assert!(marks_in("let a = \"中文\";").is_empty());
    assert!(
        marks_in("#[cfg(test)]\nlet a = \"●\";").is_empty(),
        "测试里的不管"
    );
}
