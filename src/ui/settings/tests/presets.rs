//! 预设、人格的窗（蓝图「配置页」第 38 条）。

use serde_json::json;

use super::{page, screen};
use crate::config::Config;
use crate::core::Persona;
use crate::settings::pages::choose::{PersonaView, PresetView};
use crate::settings::pages::{personas, presets};
use crate::settings::popup::Popup;

#[test]
fn a_new_preset_or_persona_window_is_titled_by_its_name_before_the_list_is_read_again() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    // 列表还是新建以前的那一份：只有一个别的。
    let other = Persona {
        id: "dev".into(),
        name: Some("开发".into()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
    };
    p.more.presets = Some(vec![other.clone()]);
    p.more.personas = Some(vec![other]);
    // 新建回来了，窗开着（`preset.set`、`persona.set` 回的详情带着名字），`*.list` 还没回来。
    let made = json!({"preset": "preset-1", "persona": "persona-1", "name": "我的预设"});
    p.popup = Some(Popup::Preset(PresetView {
        id: "preset-1".into(),
        detail: Some(presets::detail(&made)),
        error: None,
        cursor: 0,
    }));
    let (text, _) = screen(&mut p, &config, 100, 30);
    assert!(!text.contains("preset-1"), "编号不出现：\n{text}");
    assert!(
        text.lines().any(|l| l.trim() == "我的预设"),
        "标题写名字：\n{text}"
    );
    p.popup = Some(Popup::Persona(PersonaView {
        id: "persona-1".into(),
        detail: Some(personas::detail(&made)),
        error: None,
        cursor: 0,
    }));
    let (text, _) = screen(&mut p, &config, 100, 30);
    assert!(!text.contains("persona-1"), "编号不出现：\n{text}");
}

#[test]
fn the_preset_window_names_the_tools_under_the_cursor_in_its_corner() {
    // 2026-10-22 项目主人：选中的那一行在右上角写工具名；功能那一行写它下面的几件，放不下截掉。
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let got = json!({"preset": "mine", "name": "极简开发", "features": [
        {"id": "files", "name": "文件读写", "on": true, "installed": true, "tools": [
            {"name": "glob", "label": "找文件", "on": true},
            {"name": "grep", "label": "搜内容", "on": true}]},
        {"id": "commands", "name": "运行命令", "on": false, "installed": true,
         "tools": [{"name": "shell", "label": "执行命令", "on": false}]}]});
    let mut at = |cursor: usize, width: u16| {
        p.popup = Some(Popup::Preset(PresetView {
            id: "mine".into(),
            detail: Some(presets::detail(&got)),
            error: None,
            cursor,
        }));
        let (text, _) = screen(&mut p, &config, width, 30);
        let head = text
            .lines()
            .find(|l| l.contains("极简开发"))
            .unwrap_or_default();
        head.trim().to_string()
    };
    assert!(at(0, 100).ends_with("极简开发"), "名字那一行不写");
    assert!(at(1, 100).ends_with("极简开发"), "描述那一行不写");
    assert!(at(2, 100).ends_with("glob · grep"), "{}", at(2, 100));
    assert!(at(3, 100).ends_with("glob"), "{}", at(3, 100));
    assert!(at(5, 100).ends_with("shell"), "一件工具的功能写那一件");
    let narrow = at(2, 22);
    assert!(
        narrow.ends_with('…') && narrow.starts_with("极简开发"),
        "{narrow}"
    );
}
