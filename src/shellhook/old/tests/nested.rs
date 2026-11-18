//! 记号没配上对的（蓝图「旧版那一份」第 1 条）：一段是一个结尾记号和它前面最近的那个开头记号之间；开头后面没等到
//! 结尾又来一个开头的，前一个算没有结尾，不动。被人动过的文件里不多删用户自己的行。

use std::fs;

use super::super::super::install::status_to;
use super::super::super::shells::Shell;
use super::{Home, says, shown, words};

const BEGIN: &str = "# >>> miyu bash hook >>>\n";
const SOURCE: &str = "[ -r '/h/.miyu/hooks/bash.sh' ] && source '/h/.miyu/hooks/bash.sh'\n";
const END: &str = "# <<< miyu bash hook <<<\n";

/// 第 1 行是孤儿开头（后面的结尾被人删了），2、3 行是用户自己的，4–6 行是完整的一段。
fn orphan_then_block() -> String {
    format!("{BEGIN}export PATH=/opt/bin:$PATH\nalias ll='ls -l'\n{BEGIN}{SOURCE}{END}")
}

#[test]
fn an_orphan_start_does_not_swallow_the_lines_after_it() {
    let home = Home::new("orphan-start", None);
    let profile = home.at(".bash_profile");
    let before = orphan_then_block();
    fs::write(&profile, &before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read_to_string(&profile).unwrap(),
        format!("{BEGIN}export PATH=/opt/bin:$PATH\nalias ll='ls -l'\n"),
        "第 1 条：只删结尾和它前面最近的开头之间（4–6 行），1–3 行逐字节还在"
    );
    assert_eq!(
        fs::read_to_string(home.at(".bash_profile.miyu-old-hook.bak")).unwrap(),
        before,
        "第 3 条：备份是原样"
    );
    let path = shown(&profile);
    assert!(
        says(&said, true, &[&path, "第 1 行", "没有结尾"]),
        "前一个开头算没有结尾，照旧提醒：{said:?}"
    );
    assert!(
        says(&said, false, &[&path, "已清掉", "第 4–6 行"]),
        "第 6 条：行号是原文件里的：{said:?}"
    );
}

#[test]
fn orphans_on_both_sides_are_each_mentioned_and_kept() {
    let home = Home::new("orphans-around", None);
    let profile = home.at(".bash_profile");
    // 1 孤儿开头、2 用户的、3–5 一段、6 用户的、7 孤儿开头、8 用户的。
    let before = format!("{BEGIN}mine 2\n{BEGIN}{SOURCE}{END}mine 6\n{BEGIN}mine 8\n");
    fs::write(&profile, &before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read_to_string(&profile).unwrap(),
        format!("{BEGIN}mine 2\nmine 6\n{BEGIN}mine 8\n"),
        "只删中间那一段，两个孤儿和它们之间用户的行逐字节不变"
    );
    let path = shown(&profile);
    for line in ["第 1 行", "第 7 行"] {
        assert!(
            says(&said, true, &[&path, line, "没有结尾"]),
            "每个孤儿各提醒一行（{line}）：{said:?}"
        );
    }
    assert!(says(&said, false, &[&path, "第 3–5 行"]), "{said:?}");
    assert_eq!(
        said.iter().filter(|s| s.text.contains("没有结尾")).count(),
        2,
        "{said:?}"
    );
}

#[test]
fn a_lone_end_mark_is_left_alone() {
    let home = Home::new("lone-end", None);
    let profile = home.at(".bash_profile");
    let before = format!("a\n{END}b\n");
    fs::write(&profile, &before).unwrap();
    let (_, said) = home.install(Shell::Bash);
    assert_eq!(
        fs::read_to_string(&profile).unwrap(),
        before,
        "孤立的结尾记号不算，文件一个字节不变"
    );
    assert!(home.backups().is_empty());
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn status_gives_the_same_lines_and_changes_nothing() {
    let home = Home::new("orphan-status", None);
    let profile = home.at(".profile");
    let before = orphan_then_block();
    fs::write(&profile, &before).unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    status_to(&[Shell::Bash], &home.env(), &words(), &mut out, &mut err).unwrap();
    let err = String::from_utf8(err).unwrap();
    let path = shown(&profile);
    assert!(
        err.lines()
            .any(|l| l.contains(&path) && l.contains("第 4–6 行")),
        "第 7 条：报的是完整那一段的行号：{err}"
    );
    assert!(
        err.lines()
            .any(|l| l.contains(&path) && l.contains("第 1 行") && l.contains("没有结尾")),
        "孤儿开头照旧提醒：{err}"
    );
    assert_eq!(
        fs::read_to_string(&profile).unwrap(),
        before,
        "status 不改文件"
    );
    assert!(home.backups().is_empty());
}
