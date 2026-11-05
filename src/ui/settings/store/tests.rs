use std::collections::BTreeMap;

use serde_json::json;
use unicode_width::UnicodeWidthStr;

use super::{asking, preview, rows};
use crate::config::Config;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::{Info, Listing, Local, Remote, Store, StoreTexts, Tab};

fn words() -> StoreTexts {
    Config::builtin().unwrap().text.settings.more.store
}

fn store() -> Store {
    let remote = |id: &str, name: &str, installed: Option<&str>| Remote {
        id: id.into(),
        name: name.into(),
        version: "1.0.0".into(),
        installed: installed.map(str::to_string),
        ..Remote::default()
    };
    let mut blocked = remote("bad", "坏包", None);
    blocked.blocked = Some("x".into());
    let local = |id: &str, name: &str, on: bool| Local {
        id: id.into(),
        name: name.into(),
        version: "0.8.4".into(),
        on,
        ..Local::default()
    };
    let mut store = Store {
        source: Listing::Got(vec![
            remote("weather", "天气", Some("1.0.0")),
            remote("im", "接入QQ", Some("0.8.4")),
            remote("tarot", "塔罗", None),
            blocked,
        ]),
        local: Some(vec![
            local("im", "接入QQ", true),
            local("render", "渲染", false),
        ]),
        ..Store::default()
    };
    store.upgrades.insert("im".into(), "0.9.0".into());
    store.picked.insert("tarot".into());
    store
}

fn texts(store: &Store, width: u16) -> Vec<String> {
    rows(store, &words(), width)
        .iter()
        .map(|(l, _)| l.to_string())
        .collect()
}

#[test]
fn a_source_row_is_mark_name_version_and_a_state_flush_right() {
    let store = store();
    let lines = texts(&store, 50);
    for line in &lines {
        assert_eq!(line.width(), 48, "状态贴右边、不顶出去：{line:?}");
    }
    assert!(lines[0].trim_end().ends_with("已安装"), "{lines:?}");
    assert!(lines[1].trim_end().ends_with("可升级"), "{lines:?}");
    assert!(lines[2].contains("✓ 塔罗"), "勾了的写记号：{lines:?}");
    assert!(lines[3].trim_end().ends_with("已拉黑"), "{lines:?}");
    assert!(!lines[0].contains("查"), "不写说明");
}

#[test]
fn an_installed_row_says_what_it_upgrades_to_and_off_is_dim() {
    let mut store = store();
    store.tab = Tab::Installed;
    let lines = texts(&store, 50);
    assert!(lines[0].ends_with("可升级 → 0.9.0"), "{lines:?}");
    assert!(lines[1].ends_with("关"), "{lines:?}");
    let painted = rows(&store, &words(), 50);
    let name = painted[1]
        .0
        .spans
        .iter()
        .find(|s| s.content.contains("渲染"))
        .unwrap();
    assert_eq!(name.style, crate::theme::dim(), "关着的名字暗着");
    store.tab = Tab::Backstage;
    assert_eq!(texts(&store, 50), ["还没有带后台页的软件包"]);
}

#[test]
fn the_preview_is_key_colon_value_aligned_and_skips_what_is_missing() {
    let mut store = store();
    store.go(1);
    if let Listing::Got(list) = &mut store.source {
        list[1].downloads = Some(1284);
        list[1].week = Some(96);
    }
    store.infos.insert(
        (true, "im".into()),
        Info {
            author: Some("miyu-agent".into()),
            download: Some(2048),
            ..Info::default()
        },
    );
    let lines: Vec<String> = preview::lines(&store, &words(), &BTreeMap::new())
        .iter()
        .map(ToString::to_string)
        .collect();
    let colon = |l: &String| l.find(" : ").map(|at| l[..at].width());
    assert!(
        lines.iter().all(|l| colon(l) == colon(&lines[0])),
        "键照最长的对齐：{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.ends_with(": 1,284（本周 96）")),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l.ends_with(": 2.0 KiB")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.ends_with(": miyu-agent")),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("上游")),
        "没有的那一行不写：{lines:?}"
    );
}

#[test]
fn asking_to_install_lists_versions_dependencies_needs_and_sizes() {
    let store = store();
    let previews = [
        json!({"package": "tarot", "version": "1.0.0", "download": 1024, "installed": 4096,
               "capabilities": [{"id": "net", "name": "联网"}],
               "with": [{"package": "weather", "version": "1.0.0"}],
               "recommends": [{"package": "im", "version": "0.9.0"}]}),
        json!({"package": "im", "version": "0.9.0", "replaces": {"version": "0.8.4"}, "download": 1024}),
    ];
    let doing = Doing::Install(vec!["tarot".into(), "im".into()]);
    let lines: Vec<String> = asking::lines(&store, &doing, &previews, &words())
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(lines[0], "要安装 2 个软件包：");
    assert!(
        lines[1].contains("塔罗") && lines[1].ends_with("1.0.0"),
        "{lines:?}"
    );
    assert!(
        lines[2].ends_with("0.8.4 → 0.9.0"),
        "换掉旧版的写旧到新：{lines:?}"
    );
    let has = |want: &str| lines.iter().any(|l| l.contains(want));
    assert!(has("一起装的依赖 : 天气 1.0.0"), "{lines:?}");
    assert!(has(": 联网"), "{lines:?}");
    let at = |want: &str| lines.iter().position(|l| l.contains(want));
    assert!(
        at(": 联网") < at("一起装的依赖"),
        "「需要」排在依赖前面，依赖再长也挤不走（发布前第五批审查 T5-1）：{lines:?}"
    );
    assert!(has(": 2.0 KiB"), "下载大小加起来：{lines:?}");
    assert!(has("推荐（不装） : 接入QQ"), "{lines:?}");
    // 卸的没有要一并删的：那一行不写。
    let remove = Doing::Remove("im".into());
    let lines: Vec<String> = asking::lines(
        &store,
        &remove,
        &[json!({"version": "0.8.4", "size": 2048})],
        &words(),
    )
    .iter()
    .map(ToString::to_string)
    .collect();
    assert!(!lines.iter().any(|l| l.contains("一并删除")), "{lines:?}");
    assert!(lines.iter().any(|l| l.ends_with(": 2.0 KiB")), "{lines:?}");
    assert_eq!(lines[0], "要卸载 接入QQ 0.8.4：");
    // 出厂包没写版本：只写名字，不留空格（2026-10-31 实测）。
    let bare = asking::lines(&store, &remove, &[json!({})], &words());
    assert_eq!(bare[0].to_string(), "要卸载 接入QQ：");
}

/// 「包含」里的程序写它是哪一种，不露核心的种类词（2026-10-31 实测见到「程序 builtin」）。
#[test]
fn a_program_is_named_by_what_it_is() {
    let mut store = store();
    store.tab = Tab::Installed;
    let provides = vec![
        ("program".to_string(), vec!["builtin".to_string()]),
        ("skills".to_string(), vec!["qq".to_string()]),
    ];
    let info = Info {
        provides,
        ..Info::default()
    };
    store.infos.insert((false, "im".into()), info);
    let lines: Vec<String> = preview::lines(&store, &words(), &BTreeMap::new())
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        lines.iter().any(|l| l.ends_with(": 内置功能、技能 qq")),
        "{lines:?}"
    );
}

#[test]
fn a_dependency_with_a_version_floor_is_named_by_its_package() {
    // 核心 PK-2：依赖可以写 `im>=0.8.0`，照 `>=` 前面那半截找名字，下限照原样接在后面；找不到的照原样。
    let store = store();
    assert_eq!(preview::name_of(&store, "im>=0.8.0"), "接入QQ >=0.8.0");
    assert_eq!(preview::name_of(&store, "im"), "接入QQ");
    assert_eq!(preview::name_of(&store, "gone>=1.0.0"), "gone >=1.0.0");
}

/// 核心 SD-1：被盖住的那份并进认的那一行（2026-11-05 项目主人）：状态格写「下面有新版」，预览写「盖住 : 系统安装 1.3.0」；
/// 卸系统安装的，问的时候多一行暗色提示。
#[test]
fn a_covered_copy_shows_on_the_row_that_covers_it() {
    use crate::settings::pages::store::read::Covered;
    let mut store = store();
    store.tab = Tab::Installed;
    let from: BTreeMap<String, String> = [("distro".to_string(), "系统安装".to_string())].into();
    if let Some(local) = &mut store.local {
        local[1].covers = Some(Covered {
            layer: "distro".into(),
            version: "1.3.0".into(),
            newer: true,
        });
    }
    store.go(1);
    let lines = texts(&store, 50);
    assert!(lines[1].ends_with("下面有新版"), "{lines:?}");
    let shown: Vec<String> = preview::lines(&store, &words(), &from)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        shown
            .iter()
            .any(|l| l.ends_with(": 系统安装 1.3.0（更新，卸掉这份就换回它）")),
        "{shown:?}"
    );
    if let Some(local) = &mut store.local {
        local[1].covers.as_mut().unwrap().newer = false;
    }
    assert!(!texts(&store, 50)[1].contains("下面有新版"));
    let shown: Vec<String> = preview::lines(&store, &words(), &from)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        shown.iter().any(|l| l.ends_with(": 系统安装 1.3.0")),
        "{shown:?}"
    );
    let remove = Doing::Remove("render".into());
    let ask = |layer: &str| -> Vec<String> {
        asking::lines(&store, &remove, &[json!({"layer": layer})], &words())
            .iter()
            .map(ToString::to_string)
            .collect()
    };
    let note = "系统安装的包只是不再加载，文件要用系统的包管理器删";
    assert!(
        ask("distro").iter().any(|l| l.ends_with(note)),
        "{:?}",
        ask("distro")
    );
    assert!(!ask("home").iter().any(|l| l.contains(note)));
}
