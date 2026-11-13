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

#[test]
fn the_state_directory_says_where_and_how_big_when_the_core_tells() {
    // 核心 PK-4（11-05 core1）：`state` 从真假改成 `{path, size}`；旧核心的 `true` 照旧只写「状态目录」，没有的不写。
    let both = ("状态目录", "状态目录 {path}（{size}）");
    let home = std::env::home_dir().unwrap_or_default();
    let path = home.join(".miyu/var/lib/xx").display().to_string();
    let new = serde_json::json!({"path": path, "size": 3_355_443});
    assert_eq!(
        super::state_item(&new, both).as_deref(),
        Some("状态目录 ~/.miyu/var/lib/xx（3.2 MiB）")
    );
    assert_eq!(
        super::state_item(&serde_json::json!(true), both).as_deref(),
        Some("状态目录")
    );
    assert_eq!(super::state_item(&serde_json::Value::Null, both), None);
    assert_eq!(super::state_item(&serde_json::json!(false), both), None);
}

#[test]
fn removing_says_whether_the_config_directory_stays_or_goes() {
    // 核心 PK-5（11-05 core1）：预览带 `config: {path, kept}`，留着写留在哪、`purge` 的写会删掉哪；没有这一格的不写。
    let home = std::env::home_dir().unwrap_or_default();
    let path = home.join(".miyu/etc/im").display().to_string();
    let row = |config: serde_json::Value| {
        let preview = json!({"package": "im", "config": config});
        planned(json!({"tool": "package", "remove": "im", "preview": preview}))
            .unwrap()
            .rows
    };
    assert_eq!(
        row(json!({"path": path, "kept": true})),
        ["配置：留在 ~/.miyu/etc/im"]
    );
    assert_eq!(
        row(json!({"path": path, "kept": false})),
        ["配置：会删掉 ~/.miyu/etc/im"]
    );
    assert!(
        row(serde_json::Value::Null).is_empty(),
        "旧核心、没有配置目录的不写"
    );
}

#[test]
fn removing_a_system_package_says_the_files_stay() {
    // 核心 SD-1（11-05 core1）：卸系统安装的只记一笔、文件不动；主人定写一行提示。
    let rows = |layer: &str| {
        let preview = json!({"package": "im", "layer": layer});
        planned(json!({"tool": "package", "remove": "im", "preview": preview}))
            .unwrap()
            .rows
    };
    assert_eq!(
        rows("distro"),
        ["提示：系统安装的包只是不再加载，文件要用系统的包管理器删"]
    );
    assert!(rows("home").is_empty());
}

#[test]
fn installing_says_where_from_which_capabilities_are_held_and_the_notice() {
    // 核心 PK-3 上（11-06 core1）：来源、批过的标「已批准」、社区的「注意」。
    let got = planned(json!({"tool": "package", "install": "/home/alice/tarot", "preview": {
        "package": "tarot", "version": "1.0.0", "source": "community", "feed": "official",
        "capabilities": [{"id": "net", "name": "联网", "held": true}, {"id": "fs", "name": "读文件"}],
        "notice": ["这是个社区包，没有经过官方审核，可能存在风险。"]}}))
    .unwrap();
    assert_eq!(
        got.rows,
        [
            "来源：社区",
            "需要：联网（已批准）、读文件",
            "注意：这是个社区包，没有经过官方审核，可能存在风险。"
        ]
    );
    // 卸的时候只有设置项留下（核心 PK-5补）。
    let rows = planned(json!({"tool": "package", "remove": "im", "preview": {
        "package": "im", "config": {"kept": true, "settings": 2}}}))
    .unwrap()
    .rows;
    assert_eq!(rows, ["配置：2 项设置留在配置里"]);
}
