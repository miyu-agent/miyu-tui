use serde_json::json;

use super::{Covered, info, locals, remotes, upgrades};

#[test]
fn a_source_package_keeps_what_the_three_pages_show() {
    let got = json!({"packages": [
        {"package": "miyu-scripts-weather", "version": "1.2.0", "name": "天气", "summary": "查天气",
         "installed": "1.1.0", "blocked": null, "depends": ["miyu-base-net"],
         "provides": {"scripts": ["weather"]}, "source": "official",
         "published": "2026-10-30T00:00:00Z", "downloads": 1904, "downloads_week": 233},
        {"package": "odd", "version": "0.1.0", "installed": "", "blocked": "bad author"},
        {"package": "new", "version": "1.0.0", "installed": null, "source": "mine"},
        {"name": "no id"}
    ]});
    let list = remotes(&got);
    assert_eq!(list.len(), 3, "没有编号的不要");
    let w = &list[0];
    assert_eq!(
        (w.name.as_str(), w.installed.as_deref()),
        ("天气", Some("1.1.0"))
    );
    assert_eq!(
        w.provides,
        [("scripts".to_string(), vec!["weather".to_string()])]
    );
    assert_eq!((w.downloads, w.week), (Some(1904), Some(233)));
    assert_eq!(w.published.as_deref(), Some("2026-10-30T00:00:00Z"));
    assert_eq!(list[1].name, "odd", "没有名字的照编号");
    assert_eq!(
        list[1].installed.as_deref(),
        Some(""),
        "装着、没写版本的是空的字"
    );
    assert_eq!(list[1].blocked.as_deref(), Some("bad author"));
    assert_eq!(list[2].installed, None);
    assert_eq!(list[2].source, "mine");
    assert_eq!(
        (list[2].downloads, list[2].published.clone()),
        (None, None),
        "旧索引没有这三格"
    );
}

#[test]
fn installed_packages_are_one_flat_table_by_name() {
    let got = json!({"packages": [
        {"package": "miyu-base-shell", "name": "跑命令", "version": "0.8.0"},
        {"package": "miyu-im", "name": "接入QQ", "version": "0.8.4", "enabled": false, "page": true},
        {"package": "miyu-base-write", "name": "写文件", "removed": true},
        {"package": "miyu-tui", "name": "终端", "version": "0.8.0"}
    ]});
    let list = locals(&got);
    let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "照名字排");
    let im = list.iter().find(|p| p.id == "miyu-im").unwrap();
    assert!(!im.on && im.page, "开关关着的是关；有后台页");
    let write = list.iter().find(|p| p.id == "miyu-base-write").unwrap();
    assert!(!write.on, "卸掉的出厂包是关");
    assert!(list.iter().find(|p| p.id == "miyu-tui").unwrap().on);
}

#[test]
fn upgrades_and_info_are_read() {
    let ups = upgrades(
        &json!({"upgrades": [{"package": "miyu-im", "from": "0.8.4", "to": "0.9.0"}],
        "moved": [], "gone": ["x"]}),
    );
    assert_eq!(ups.get("miyu-im").map(String::as_str), Some("0.9.0"));
    assert_eq!(ups.len(), 1);
    let got = info(
        &json!({"package": "miyu-im", "author": "miyu-agent", "from": "official",
        "installed": "2026-10-29T08:00:00.000Z", "size": 19_000_000, "required_by": ["miyu-memes"],
        "capabilities": [{"id": "net.listen", "name": "开端口"}],
        "provides": {"program": "process", "skills": ["qq"], "page": true, "settings": 12}}),
    );
    assert_eq!(got.author.as_deref(), Some("miyu-agent"));
    assert_eq!(got.size, Some(19_000_000));
    assert_eq!(got.required_by, ["miyu-memes"]);
    assert_eq!(got.needs, ["开端口"]);
    let kinds: Vec<&str> = got.provides.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        kinds,
        ["page", "program", "settings", "skills"],
        "照种类排，开着的、数的只写种类"
    );
    let source =
        info(&json!({"package": "x", "from": "source", "installed_size": 2048, "download": 1024}));
    assert_eq!(
        (source.size, source.download),
        (Some(2048), Some(1024)),
        "没装的照安装后大小"
    );
}

/// 核心 SD-1：被盖住的那份不单列，并进认的那一行（2026-11-05 项目主人）；盖住几份的记更新的那一份。
#[test]
fn a_covered_copy_joins_the_row_that_covers_it() {
    let got = json!({"packages": [
        {"package": "im", "layer": "distro", "version": "1.3.0",
         "covered": {"by": "home", "version": "1.3.0", "over": "1.2.0", "newer": true}},
        {"package": "im", "layer": "shipped", "version": "1.0.0",
         "covered": {"by": "home", "version": "1.0.0", "over": "1.2.0", "newer": false}},
        {"package": "im", "layer": "home", "version": "1.2.0", "name": "接入QQ"},
        // F-9补：认的那份带 `covered: true`（包里的 MCP 全被自制的替代），不是被盖住。
        {"package": "memes", "layer": "home", "version": "0.1.0", "covered": true},
    ]});
    let list = locals(&got);
    assert_eq!(list.len(), 2, "一包一行：{list:?}");
    let im = list.iter().find(|p| p.id == "im").unwrap();
    assert_eq!(im.version, "1.2.0", "认的那份");
    assert_eq!(
        im.covers,
        Some(Covered {
            layer: "distro".into(),
            version: "1.3.0".into(),
            newer: true,
        })
    );
    let memes = list.iter().find(|p| p.id == "memes").unwrap();
    assert_eq!(memes.covers, None);
}

/// `command` 是第一个子命令、`commands` 是全部：两格都有的只留全部的（2026-11-06 实测见到「命令 memes、命令 memes」）。
#[test]
fn the_first_command_is_not_listed_again_beside_all_of_them() {
    let got = info(&json!({"package": "miyu-memes",
        "provides": {"command": "memes", "commands": ["memes"], "page": true}}));
    let kinds: Vec<&str> = got.provides.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, ["commands", "page"]);
    let only = info(&json!({"package": "x", "provides": {"command": "x"}}));
    assert_eq!(
        only.provides,
        [("command".to_string(), vec!["x".to_string()])]
    );
}

/// 核心 SD-1补：卸掉的出厂、系统安装的包记着 `removed`，空格照编号装回。
#[test]
fn a_removed_package_is_marked() {
    let list = locals(&json!({"packages": [
        {"package": "tarot", "layer": "distro", "removed": true},
        {"package": "im", "enabled": false}]}));
    let removed: Vec<(&str, bool, bool)> = list
        .iter()
        .map(|p| (p.id.as_str(), p.removed, p.on))
        .collect();
    assert_eq!(removed, [("im", false, false), ("tarot", true, false)]);
}

/// 核心 PK-6：装着的照 `source` 写从哪来；没装的（`from` 是 `source`）那一格是软件源的名字，照旧读 `from`。
#[test]
fn where_a_package_came_from_follows_source_when_installed() {
    let installed = info(&json!({"package": "x", "from": "official", "source": "community"}));
    assert_eq!(installed.from.as_deref(), Some("community"));
    let remote = info(&json!({"package": "x", "from": "source", "source": "my-feed"}));
    assert_eq!(remote.from.as_deref(), Some("source"));
    let old = info(&json!({"package": "x", "from": "shipped"}));
    assert_eq!(old.from.as_deref(), Some("shipped"));
}
