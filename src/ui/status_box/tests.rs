//! `/status` 会话状态框（蓝图 `tui.md`「会话状态 `/status`」，2026-10-20 项目主人看了样板定）。

use super::{Facts, plain};
use crate::config::Config;
use crate::core::Usage;

/// 开着的一个会话：样板里那些数。
fn opened() -> Facts {
    Facts {
        title: "修通知图标".into(),
        session: Some("01a157e1-e547-7511-b8c8-7e993d987fdd".into()),
        persona: Some("Miyu".into()),
        preset: Some("全部功能".into()),
        level: Some("工作区".into()),
        cwd: "~/src".into(),
        model: Some(("deepseek-v4.1-flash".into(), "magpie".into())),
        effort: Some("high".into()),
        window: Some(1_000_000),
        speed: Some(76.4),
        context: 41_200,
        line: Some(967_000),
        compactions: 2,
        breaks: 1,
        usage: Usage {
            uncached: 8_000,
            cache_read: 92_000,
            output: 8_600,
            aux: 2_100,
            ..Usage::default()
        },
        spent: Some("¥0.83".into()),
        memory: (4, 2),
    }
}

#[test]
fn an_open_session_lists_three_groups_with_a_blank_line_between_them() {
    let config = Config::builtin().unwrap();
    let text = plain(&opened(), &config, 100);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "修通知图标", "标题在最前（框线上）");
    // 框里上下的空行由框留（`panel.rs` 的 `padded`）：这里从会话 ID 起，段和段之间空一行。
    assert!(
        lines[1].starts_with("会话 ID") && lines[1].ends_with("7e993d987fdd"),
        "{text}"
    );
    let keys: Vec<&str> = lines[1..]
        .iter()
        .map(|l| l.split_whitespace().next().unwrap_or(""))
        .collect();
    assert_eq!(
        keys,
        [
            "会话",
            "人格",
            "预设",
            "权限模式",
            "当前工作目录",
            "",
            "模型",
            "上下文",
            "压缩",
            "用量",
            "缓存命中",
            "花费",
            "",
            "记忆"
        ],
        "{text}"
    );
    assert!(text.contains("平均 76 tok/s"), "{text}");
    assert!(text.contains("到 967k 自动压缩"), "{text}");
    assert!(text.contains("缓存断裂 1 次"), "{text}");
    assert!(text.contains("回忆 4 次 · 记住 2 条"), "{text}");
    // 名字一列对齐：值都从同一列起。
    let column = |key: &str| {
        let line = lines.iter().find(|l| l.starts_with(key)).unwrap();
        unicode_width::UnicodeWidthStr::width(&line[..line.find("  ").unwrap()])
            + line[line.find("  ").unwrap()..].len()
            - line[line.find("  ").unwrap()..].trim_start().len()
    };
    assert_eq!(column("人格"), column("当前工作目录"));
}

#[test]
fn a_new_session_lists_only_what_it_already_knows() {
    let config = Config::builtin().unwrap();
    let new = Facts {
        title: "新会话".into(),
        session: None,
        ..opened()
    };
    let text = plain(&new, &config, 100);
    for gone in ["会话 ID", "上下文", "用量", "花费", "记忆"] {
        assert!(!text.contains(gone), "没开会话不写{gone}：{text}");
    }
    for kept in ["人格", "预设", "权限模式", "当前工作目录", "模型"] {
        assert!(text.contains(kept), "{text}");
    }
}

#[test]
fn a_value_too_long_wraps_under_its_own_column() {
    let config = Config::builtin().unwrap();
    let mut f = opened();
    f.cwd = "~/Documents/github/miyu-agent/miyu-tui/src/ui/status_box/a/very/deep/folder".into();
    let text = plain(&f, &config, 50);
    let at = text
        .lines()
        .position(|l| l.starts_with("当前工作目录"))
        .unwrap();
    let next = text.lines().nth(at + 1).unwrap();
    assert!(
        next.starts_with("              "),
        "折下来的照值那一列：{text}"
    );
    assert!(
        text.lines()
            .all(|l| unicode_width::UnicodeWidthStr::width(l) <= 48),
        "{text}"
    );
}
