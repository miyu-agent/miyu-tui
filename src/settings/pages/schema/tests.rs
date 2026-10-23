//! 读配置清单、终端自己的几项分组分页。

use serde_json::json;

use super::{Row, Schema};

fn sample() -> Schema {
    Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}, {"id": "permissions", "name": "权限"}, {"id": "models", "name": "模型"}],
        "groups": [{"id": "display", "name": "显示", "page": "general"}, {"id": "persona", "name": "人格", "page": "general"},
                   {"id": "sessions", "name": "会话", "page": "permissions"}, {"id": "empty", "name": "空", "page": "general"},
                   {"id": "providers", "name": "供应商", "page": "models"}],
        "items": [
            {"key": "ui.startup", "control": "select", "layers": ["system", "personal"], "name": "启动时", "page": "general", "group": "display",
             "options": [{"value": "new", "name": "新会话"}, {"value": "recent", "name": "最近的"}], "default": "new"},
            {"key": "ui.language", "control": "select", "layers": ["system", "personal"], "name": "界面语言", "page": "general", "group": "display", "common": true},
            {"key": "persona.default", "control": "select", "layers": ["system", "personal"], "name": "默认人格", "page": "general", "group": "persona"},
            {"key": "permission.start_read_only", "control": "toggle", "layers": ["system"], "name": "只读开始", "page": "permissions", "group": "sessions"},
            {"key": "providers.<id>.base_url", "control": "text", "layers": ["personal"], "name": "地址", "page": "models", "group": "providers"},
            {"key": "web.theme", "control": "select", "layers": ["personal"], "name": "网页主题", "page": "general", "group": "display"},
            {"key": "ui.ticket", "control": "text", "layers": ["system"], "name": "票据", "page": "general", "group": "display", "hidden": true}
        ]
    }))
}

#[test]
fn pages_list_their_groups_with_common_items_first_and_models_stay_out() {
    let s = sample();
    assert_eq!(s.items.len(), 4, "人起的名字的、web 的、hidden 的不要");
    assert_eq!(
        s.rows("general"),
        vec![
            Row::Group(0),
            Row::Item(1),
            Row::Item(0),
            Row::Group(1),
            Row::Item(2)
        ]
    );
    assert_eq!(s.group_names("general"), ["显示", "人格"], "空的组不写");
    assert_eq!(
        s.listed_pages(),
        ["general", "permissions"],
        "模型那一页不另列"
    );
    assert!(!s.items[3].personal(), "只能写系统配置");
    assert_eq!(s.page_name("permissions"), Some("权限"));
}

#[test]
fn the_terminals_own_items_sit_on_the_general_page() {
    // 「配置页」第 33 条（2026-10-10 项目主人：时间线的配置项应在通用里、又是终端自己的）。
    let s = Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}, {"id": "packages", "name": "软件包"}],
        "groups": [{"id": "display", "name": "显示", "page": "general"},
                   {"id": "im", "name": "接入通讯平台", "page": "packages"},
                   {"id": "tui", "name": "终端界面", "page": "packages"}],
        "items": [
            {"key": "ui.language", "control": "select", "layers": ["personal"], "name": "界面语言", "page": "general", "group": "display"},
            {"key": "im.listen", "control": "number", "layers": ["system"], "name": "端口", "page": "packages", "group": "im"},
            {"key": "tui.timeline_fold", "control": "toggle", "layers": ["system", "personal"], "name": "做完收起", "page": "packages", "group": "tui"}
        ]
    }));
    assert_eq!(s.group_names("general"), ["显示", "终端界面"]);
    assert_eq!(s.group_names("packages"), ["接入通讯平台"]);
}

#[test]
fn the_terminals_look_items_move_to_an_appearance_page_after_general() {
    // 「配置页」第 33 条（2026-10-11 项目主人：时间线相关的配置项单独一个标题，吉祥物也分出去；2026-10-14 照推荐定：
    // 主题、吉祥物、图标单开一页「外观」，排在通用后面，时间线留在通用）。
    let item = |key: &str, name: &str| json!({"key": key, "control": "toggle", "layers": ["personal"], "name": name, "page": "packages", "group": "tui"});
    let mut s = Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}, {"id": "packages", "name": "软件包"}],
        "groups": [{"id": "display", "name": "显示", "page": "general"},
                   {"id": "tui", "name": "终端界面", "page": "packages"}],
        "items": [
            {"key": "ui.language", "control": "select", "layers": ["personal"], "name": "界面语言", "page": "general", "group": "display"},
            item("tui.icons", "图标"),
            item("tui.theme", "配色"),
            item("tui.theme_mode", "明暗"),
            item("tui.timeline_fold", "回复完成后折叠时间线"),
            item("tui.keys", "按键模式"),
            item("tui.skip_pick", "新会话不再询问人格预设"),
            item("tui.mascot_home", "首页显示吉祥物"),
            item("tui.mascot", "吉祥物")
        ]
    }));
    s.split_own(
        &super::OwnGroups {
            look: "主题".into(),
            theme: "配色方案".into(),
            timeline: "时间线".into(),
            mascot: "吉祥物".into(),
            icons: "图标".into(),
            keys: "交互".into(),
        },
        "外观",
    );
    assert_eq!(s.group_names("general"), ["显示", "时间线", "交互"]);
    assert_eq!(s.group_names("appearance"), ["配色方案", "吉祥物", "图标"]);
    assert_eq!(s.listed_pages(), ["general", "appearance"]);
    assert_eq!(s.page_name("appearance"), Some("外观"));
    let group = |key: &str| {
        s.items
            .iter()
            .find(|i| i.key == key)
            .map(|i| (i.page.clone(), i.group.clone()))
    };
    assert_eq!(
        group("tui.theme_mode"),
        Some(("appearance".into(), "tui.theme".into()))
    );
    assert_eq!(
        group("tui.timeline_fold"),
        Some(("general".into(), "tui.timeline".into()))
    );
    assert_eq!(
        group("tui.keys"),
        Some(("general".into(), "tui.keys".into()))
    );
    // 不问人格预设并进「交互」，不另开一组（2026-10-22）。
    assert_eq!(
        group("tui.skip_pick"),
        Some(("general".into(), "tui.keys".into()))
    );
    assert_eq!(s.groups.iter().filter(|g| g.id == "tui.keys").count(), 1);
}

#[test]
fn skip_pick_joins_the_cores_new_session_group_when_there_is_one() {
    // 2026-10-23 项目主人：默认人格、默认预设、不问人格预设、开局只读做成「新会话」一组；核心的组是 `new_session`。
    let item = |key: &str, page: &str, group: &str| json!({"key": key, "control": "toggle", "layers": ["personal"], "name": key, "page": page, "group": group});
    let mut s = Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}, {"id": "packages", "name": "软件包"}],
        "groups": [{"id": "new_session", "name": "新会话", "page": "general"},
                   {"id": "tui", "name": "终端界面", "page": "packages"}],
        "items": [
            item("persona.default", "general", "new_session"),
            item("tui.keys", "packages", "tui"),
            item("tui.skip_pick", "packages", "tui")
        ]
    }));
    s.split_own(
        &super::OwnGroups {
            look: "主题".into(),
            theme: "配色方案".into(),
            timeline: "时间线".into(),
            mascot: "吉祥物".into(),
            icons: "图标".into(),
            keys: "交互".into(),
        },
        "外观",
    );
    let at = |key: &str| {
        s.items
            .iter()
            .find(|i| i.key == key)
            .map(|i| (i.page.clone(), i.group.clone()))
    };
    assert_eq!(
        at("tui.skip_pick"),
        Some(("general".into(), "new_session".into()))
    );
    assert_eq!(at("tui.keys"), Some(("general".into(), "tui.keys".into())));
}
