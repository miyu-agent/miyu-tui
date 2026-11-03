//! 功能树画出来的样子（「配置页」第 38 条「功能树」，核心设计 52）。

use ratatui::text::Span;
use serde_json::json;

use super::{Focus, draw};
use crate::config::Config;
use crate::features::{Row, read};
use crate::theme;

fn tree() -> crate::features::Tree {
    read(&json!({
    "features": [{"id": "web", "installed": false}],
    "sections": [
        {"id": "basesystem", "kind": "group", "name": "基础系统", "items": [
            {"id": "files", "name": "文件读写", "on": true, "tools": [
                {"id": "glob", "name": "找文件", "on": true},
                {"id": "grep", "name": "搜内容", "on": false}]},
            {"id": "commands", "name": "运行命令", "on": false}]},
        {"id": "skills", "kind": "skills", "name": "技能", "items": []},
        {"id": "research", "kind": "group", "name": "查资料", "items": [
            {"id": "web", "name": "联网", "on": true}]}
    ]}))
}

fn focus() -> Focus {
    Focus {
        lead: Span::styled(">", theme::accent()),
        bg: theme::row_focus(),
    }
}

fn text(lines: &[ratatui::text::Line]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect()
}

#[test]
fn a_wide_window_lays_sections_out_in_two_columns() {
    let texts = Config::builtin().unwrap().text.feature_tree;
    let drawn = draw(&tree(), None, 80, (&texts, &focus()));
    assert!(drawn.two);
    // 左列补到最宽那一格（「    [*] 找文件」14 列）、空四格接右列；工具缩进四格对齐在名字下面。
    assert_eq!(
        text(&drawn.lines),
        [
            "  基础系统  1/2       技能  0/0",
            "  [~] 文件读写        还没有技能",
            "      [*] 找文件",
            "      [ ] 搜内容      查资料  0/0",
            "  [ ] 运行命令            联网  未安装",
        ]
    );
    assert_eq!(drawn.focus, None, "只读的「看全」没有光标");
}

#[test]
fn a_narrow_window_stacks_one_column() {
    let texts = Config::builtin().unwrap().text.feature_tree;
    let drawn = draw(&tree(), None, 30, (&texts, &focus()));
    assert!(!drawn.two);
    let lines = text(&drawn.lines);
    assert_eq!(lines[4], "  [ ] 运行命令");
    assert_eq!(lines[5], "", "组和组之间空一行");
    assert_eq!(lines[6], "  技能  0/0");
}

#[test]
fn the_cursor_cell_alone_gets_the_lead_and_the_ground() {
    let texts = Config::builtin().unwrap().text.feature_tree;
    let drawn = draw(&tree(), Some(Row::Feature(1)), 80, (&texts, &focus()));
    assert_eq!(drawn.focus, Some(4));
    let line = &drawn.lines[4];
    assert!(line.to_string().starts_with("> [ ] 运行命令"), "{line}");
    let ground = theme::row_focus().bg;
    let lit: String = line
        .spans
        .iter()
        .filter(|s| s.style.bg == ground && ground.is_some())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(
        lit.contains("运行命令") && !lit.contains("联网"),
        "只铺光标那一格：{lit:?}"
    );
}

/// 光标停在一列最后一个停得上的行：窗照这一列的最底下滚，后面停不上的也露出来（写了没装的、空的组）。
#[test]
fn the_last_stop_in_a_column_scrolls_to_the_columns_bottom() {
    let texts = Config::builtin().unwrap().text.feature_tree;
    // 一列：基础系统 4 行、空行、技能 2 行、空行、查资料 2 行（联网没装，停不上）。
    let drawn = draw(&tree(), Some(Row::Feature(1)), 30, (&texts, &focus()));
    assert_eq!(
        drawn.focus,
        Some(drawn.lines.len() - 1),
        "{:?}",
        text(&drawn.lines)
    );
    let drawn = draw(&tree(), Some(Row::Feature(0)), 30, (&texts, &focus()));
    assert_eq!(drawn.focus, Some(1), "不是最后一个照光标那一行");
}
