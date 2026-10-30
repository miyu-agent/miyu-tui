use serde_json::json;

use super::{info, locals, remotes, upgrades};

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
