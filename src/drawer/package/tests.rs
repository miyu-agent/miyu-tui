//! 装、卸软件包，打开扩展的确认写成什么（2026-10-12 项目主人定照 `miyu pkg install` 列清楚）。

use serde_json::json;

use super::{Plan, plan};
use crate::config::Config;
use crate::drawer::event::Detail;

/// `detail` 写成的样子，照中文的字。
fn planned(detail: serde_json::Value) -> Option<Plan> {
    let detail: Detail = serde_json::from_value(detail).unwrap();
    plan(&detail, &Config::builtin().unwrap().text.drawer.package)
}

#[test]
fn installing_lists_what_it_carries_what_it_cannot_read_needs_and_size() {
    let got = planned(
        json!({"tool": "package", "install": "/home/alice/weather", "preview": {
        "package": "weather", "version": "1.2.0", "replaces": {"version": "1.1.0"},
        "program": "process", "mcp": ["weather"], "skills": ["forecast"], "scripts": ["radar"],
        "settings": 2, "capabilities": [{"id": "net", "name": "联网", "summary": "连外网"}],
        "skipped": [{"file": "skills/bad/SKILL.md", "why": "missing name"}],
        "files": 12, "size": 1_468_006,
        // 核心 SU-4 中以前的预览还带这一格，只给一个人用以后不写。
        "system_account": true}}),
    );
    assert_eq!(
        got,
        Some(Plan {
            title: "安装 weather 1.2.0（替换 1.1.0）".into(),
            rows: vec![
                "包含：扩展程序、MCP 服务 weather、技能 forecast、脚本 radar、2 项设置".into(),
                "读不成：skills/bad/SKILL.md（missing name）".into(),
                "需要：联网".into(),
                "大小：1.4 MiB，12 个文件".into(),
            ],
        })
    );
}

#[test]
fn bringing_back_a_shipped_package_says_restore() {
    let got = planned(json!({"tool": "package", "install": "memory",
        "preview": {"package": "memory", "restores": true}}));
    assert_eq!(
        got,
        Some(Plan {
            title: "装回 memory".into(),
            rows: Vec::new(),
        })
    );
}

#[test]
fn removing_lists_what_goes_with_it() {
    let got = planned(json!({"tool": "package", "remove": "weather", "preview": {
        "package": "weather", "layer": "home", "version": "1.2.0", "files": 1, "size": 2048,
        "settings": ["weather.city", "weather.unit"], "state": true}}));
    assert_eq!(
        got,
        Some(Plan {
            title: "卸载 weather 1.2.0".into(),
            rows: vec![
                "一并删除：设置 weather.city、weather.unit、状态目录".into(),
                "大小：2.0 KiB，1 个文件".into(),
            ],
        })
    );
}

#[test]
fn enabling_an_extension_says_what_it_needs() {
    let got = planned(json!({"tool": "package", "enable": "weather", "preview": {
        "package": "weather", "program": "process", "capabilities": [{"name": "联网"}]}}));
    assert_eq!(
        got,
        Some(Plan {
            title: "打开扩展 weather".into(),
            rows: vec!["包含：扩展程序".into(), "需要：联网".into()],
        })
    );
}

#[test]
fn without_a_preview_the_title_takes_the_folder_name_and_other_asks_are_not_this_kind() {
    let got = planned(json!({"tool": "package", "install": "/home/alice/weather"}));
    assert_eq!(got.map(|p| p.title).as_deref(), Some("安装 weather"));
    assert_eq!(planned(json!({"tool": "write", "paths": []})), None);
}
