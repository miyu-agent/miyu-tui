//! 主题那张表（蓝图「主题」）。

use serde_json::json;

use super::parts::Line;
use super::{default_value, merged, overlay, parse};

#[test]
fn the_default_theme_reads_and_a_pack_only_writes_what_it_changes() {
    let whole = parse(&default_value()).expect("默认主题读得懂");
    assert_eq!(whole.frame.boxes, Line::Rounded, "出厂圆角");
    // 主题包只写一格：别的照默认主题。
    let pack = json!({"_": "说明不读", "frame": {"boxes": "ascii"}});
    let sheet = parse(&merged(&pack)).unwrap();
    assert_eq!(sheet.frame.boxes, Line::Ascii);
    assert_eq!(sheet.frame.input, whole.frame.input);
    assert_eq!(sheet.text, whole.text);
    assert_eq!(Line::Ascii.set().top_left, "+");
}

#[test]
fn the_theme_overlays_its_marks_on_the_resource_files() {
    // 原来写在 layout.json、timeline.json 里的记号挪进了默认主题，读配置时盖回去，和原来一样。
    let theme = default_value();
    let layout = overlay(
        include_str!("../../resources/layout.json"),
        "layout",
        &theme,
    )
    .unwrap();
    assert_eq!(layout["prompt"], "❯ ");
    assert_eq!(layout["bar"]["full"], "▰");
    assert!(
        layout["bar"]["width"].is_number(),
        "layout.json 自己的数照留"
    );
    let pack = merged(&json!({"timeline": {"line": "|"}}));
    let timeline = overlay(
        include_str!("../../resources/timeline.json"),
        "timeline",
        &pack,
    )
    .unwrap();
    assert_eq!(timeline["line"], "|");
    assert_eq!(timeline["omitted"], "⋮", "没写的照默认主题");
}

/// 包的编号就是文件夹名（核心设计 48）：配置里没设主题时认的默认主题是仓库里同名的那个文件夹。它编进了程序、不发，终端清单
/// 不推荐它（「主题」第 4 条，2026-10-30 项目主人定）。清单写 `prefix = "tui"`，配置键照旧是 `tui.*`。
#[test]
fn the_terminal_manifest_recommends_nothing_and_keeps_tui_keys() {
    let manifest = include_str!("../../tui/package.toml");
    assert!(
        !manifest.lines().any(|l| l.trim() == "[recommends]"),
        "默认主题编进了程序，不推荐"
    );
    assert!(
        manifest.lines().any(|l| l == "prefix = \"tui\""),
        "配置键要照旧是 tui.*"
    );
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packages")
        .join(super::DEFAULT_PACKAGE);
    assert!(
        folder.join("package.toml").exists(),
        "{} 不在",
        folder.display()
    );
}

#[test]
fn the_ascii_theme_leaves_no_symbol_outside_ascii() {
    // 2026-10-21 项目主人照推荐定：纯 ASCII 样板拿来验「所有样子都能换」。盖上它以后，表里每一个记号、`layout`、
    // `timeline` 两节盖的每一格、框线画出来的每一个字都是 ASCII。
    let pack: serde_json::Value =
        serde_json::from_str(include_str!("../../resources/looks/ascii.json")).unwrap();
    let whole = merged(&pack);
    let sheet = parse(&whole).expect("纯 ASCII 主题读得懂");
    let mut strings = Vec::new();
    for section in ["marks", "layout", "timeline"] {
        collect(&whole[section], section, &mut strings);
    }
    // 两节只看默认主题盖的那几格（`layout.json` 自己的字不归主题）。
    let default = super::default_value();
    strings.retain(|(path, _)| lookup(&default, path).is_some());
    let frames = [
        sheet.frame.input,
        sheet.frame.boxes,
        sheet.frame.toast,
        sheet.frame.code,
        sheet.frame.table,
    ];
    for line in frames {
        let set = line.set();
        let lines = line.lines();
        for s in [
            set.top_left,
            set.horizontal_top,
            set.vertical_left,
            lines.cross,
        ] {
            strings.push(("frame".to_string(), s.to_string()));
        }
    }
    let bad: Vec<_> = strings.iter().filter(|(_, s)| !s.is_ascii()).collect();
    assert!(bad.is_empty(), "还有不是 ASCII 的：{bad:?}");
    assert_eq!(sheet.pick, super::Pick::Mark);
}

/// 一份里所有的字，连同它在哪（`a.b.c`）。
fn collect(value: &serde_json::Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::String(s) => out.push((path.to_string(), s.clone())),
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect(v, path, out)),
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                collect(v, &format!("{path}.{k}"), out);
            }
        }
        _ => {}
    }
}

/// 照 `a.b.c` 找。
fn lookup<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(value, |v, k| v.get(k))
}

/// todolist 63：纯 ASCII 编进了程序（`ascii`），没写主题的在 Linux 控制台里照它，写过的照写的；`TERM=linux` 才算控制台。
#[test]
fn the_builtin_ascii_look_and_the_console_default() {
    use serde_json::json;
    assert!(super::builtin(super::ASCII).is_some());
    assert!(super::builtin("night").is_none());
    let unset = json!({"value": null, "origin": {"layer": "default"}});
    assert_eq!(super::chosen(&unset, false), None);
    assert_eq!(super::chosen(&unset, true).as_deref(), Some("ascii"));
    let default = json!({"value": "tui-theme-default", "origin": {"layer": "system"}});
    assert_eq!(super::chosen(&default, true), None, "写了默认主题的照写的");
    let night = json!({"value": "night"});
    assert_eq!(super::chosen(&night, true).as_deref(), Some("night"));
    assert!(super::written(&default) && !super::written(&unset) && !super::written(&json!({})));
    assert!(super::console_term(Some("linux")));
    assert!(!super::console_term(Some("xterm-256color")) && !super::console_term(None));
}
