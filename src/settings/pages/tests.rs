use serde_json::json;

use super::schema::Schema;
use super::{Entry, More};

fn schema(pages: &[&str]) -> Schema {
    let pages_json: Vec<_> = pages.iter().map(|p| json!({"id": p, "name": p})).collect();
    let groups: Vec<_> = pages
        .iter()
        .map(|p| json!({"id": format!("g-{p}"), "name": "组", "page": p}))
        .collect();
    let items: Vec<_> = pages
        .iter()
        .map(|p| json!({"key": format!("{p}.x"), "control": "text", "layers": ["system"], "name": "x", "page": p, "group": format!("g-{p}")}))
        .collect();
    Schema::read(&json!({"pages": pages_json, "groups": groups, "items": items}))
}

#[test]
fn the_menu_puts_general_first_and_personas_before_advanced() {
    // 2026-10-07 项目主人：通用第一行，人格、预设放在下面。
    let mut more = More::default();
    assert_eq!(
        more.entries(),
        [Entry::Models, Entry::Personas],
        "没读到清单以前"
    );
    // 核心 F-4 再补、F-6 上起只给这几页：「权限」并进了「通用」，「接入」并进了「软件包」（2026-10-10 项目主人）。
    more.schema = Some(schema(&[
        "general", "models", "packages", "advanced", "voice",
    ]));
    assert_eq!(
        more.entries(),
        [
            Entry::Page("general".into()),
            Entry::Models,
            Entry::Personas,
            Entry::Page("packages".into()),
            Entry::Page("voice".into()),
            Entry::Page("advanced".into()),
        ]
    );
    // 2026-10-14：终端自己加的「外观」排在通用后面；2026-10-21 项目主人：外观紧跟通用、在供应商和模型前面。
    more.schema = Some(schema(&["advanced", "appearance", "general", "models"]));
    assert_eq!(
        more.entries(),
        [
            Entry::Page("general".into()),
            Entry::Page("appearance".into()),
            Entry::Models,
            Entry::Personas,
            Entry::Page("advanced".into()),
        ]
    );
    // 2026-10-16：核心的「用户」页、外观接在后面；2026-10-19 项目主人：供应商和模型挪到用户上面。
    more.schema = Some(schema(&[
        "general",
        "user",
        "advanced",
        "appearance",
        "models",
    ]));
    assert_eq!(
        more.entries(),
        [
            Entry::Page("general".into()),
            Entry::Page("appearance".into()),
            Entry::Models,
            Entry::Page("user".into()),
            Entry::Personas,
            Entry::Page("advanced".into()),
        ]
    );
    // 2026-10-21：核心的「隐私与安全」排在用户后面、外观前面（项目主人照推荐定）。
    more.schema = Some(schema(&[
        "general",
        "privacy",
        "user",
        "advanced",
        "appearance",
        "models",
    ]));
    assert_eq!(
        more.entries(),
        [
            Entry::Page("general".into()),
            Entry::Page("appearance".into()),
            Entry::Models,
            Entry::Page("user".into()),
            Entry::Page("privacy".into()),
            Entry::Personas,
            Entry::Page("advanced".into()),
        ]
    );
    more.menu_at = 99;
    assert_eq!(
        more.selected(),
        Entry::Page("advanced".into()),
        "越界停在最后"
    );
}

#[test]
fn a_removed_mascot_package_is_named_as_not_installed() {
    // 「吉祥物包」第 3 条（2026-10-11）：选着的吉祥物包卸掉了，值不写裸编号，写「pudding（未安装）」。
    let texts = crate::config::Config::builtin().unwrap().text.settings.more;
    let s = Schema::read(&json!({"items": [
        {"key": "tui.mascot", "control": "text", "layers": ["system"], "name": "吉祥物", "page": "general", "group": "miyu-tui"}
    ]}));
    let item = &s.items[0];
    let installed = vec![crate::core::Persona {
        id: "bun".into(),
        name: Some("小圆".into()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
    }];
    assert_eq!(
        super::shown(item, &json!("bun"), Some(&installed), &texts),
        "小圆"
    );
    assert_eq!(
        super::shown(item, &json!("pudding"), Some(&installed), &texts),
        "pudding（未安装）"
    );
    assert_eq!(
        super::shown(item, &json!(null), Some(&installed), &texts),
        "内置"
    );
}

#[test]
fn icon_sets_are_named_in_the_interface_language() {
    // 2026-10-14 项目主人：外观页「图标」照界面语言写「Nerd Font 图标」「普通字符」，不写清单里的 nerd、plain。
    let texts = crate::config::Config::builtin().unwrap().text.settings.more;
    let s = Schema::read(&json!({"items": [
        {"key": "tui.icons", "control": "select", "layers": ["system"], "name": "图标", "page": "general", "group": "miyu-tui",
         "options": [{"value": "nerd", "name": "nerd"}, {"value": "plain", "name": "plain"}]}
    ]}));
    let item = &s.items[0];
    assert_eq!(
        super::shown(item, &json!("nerd"), None, &texts),
        "Nerd Font 图标"
    );
    assert_eq!(
        super::shown(item, &json!("plain"), None, &texts),
        "普通字符"
    );
}

#[test]
fn the_colors_list_hides_the_wallpaper_until_one_is_found() {
    // 2026-10-14 项目主人定：找不到壁纸的，配色里不列「跟随壁纸」。
    let choice = |value: &str| crate::settings::pages::choose::Choice {
        value: json!(value),
        label: value.into(),
        note: None,
        off: None,
    };
    let list = || vec![choice("persona"), choice("wallpaper"), choice("tokyonight")];
    let mut more = More::default();
    assert_eq!(more.offered("tui.theme", list()).len(), 2);
    assert_eq!(
        more.offered("tui.theme_mode", list()).len(),
        3,
        "别的项不管"
    );
    more.wallpaper = true;
    assert_eq!(more.offered("tui.theme", list()).len(), 3);
}

#[test]
fn private_directories_show_how_many_not_every_path() {
    // 2026-10-22 项目主人：「加的私密目录」把路径一条条铺出来、「默认私密目录」写「—」，都该写个数；别的列表照旧列。
    let texts = crate::config::Config::builtin().unwrap().text.settings.more;
    let s = Schema::read(&json!({"items": [
        {"key": "sandbox.private.add", "type": "list", "control": "list", "element": "text", "layers": ["system"],
         "name": "加的私密目录", "page": "privacy", "group": "private"},
        {"key": "sandbox.private.remove", "type": "list", "control": "list", "element": "text", "layers": ["system"],
         "name": "默认私密目录", "page": "privacy", "group": "private", "ticks": "unlisted",
         "options": [{"value": "~/.ssh", "name": "~/.ssh"}, {"value": "~/.gnupg", "name": "~/.gnupg"},
                     {"value": "~/.aws", "name": "~/.aws"}]},
        {"key": "packages.sources", "type": "list", "control": "list", "element": "text", "layers": ["system"],
         "name": "软件源", "page": "packages", "group": "source"},
        {"key": "sandbox.readonly.add", "type": "list", "control": "list", "element": "text", "layers": ["system"],
         "name": "加的受保护路径", "page": "privacy", "group": "readonly"},
        {"key": "sandbox.readonly.remove", "type": "list", "control": "list", "element": "text", "layers": ["system"],
         "name": "默认受保护路径", "page": "privacy", "group": "readonly", "ticks": "unlisted",
         "options": [{"value": ".git/hooks", "name": ".git/hooks"}, {"value": ".envrc", "name": ".envrc"}]}
    ]}));
    let shown =
        |i: usize, value: serde_json::Value| super::shown(&s.items[i], &value, None, &texts);
    assert_eq!(shown(0, json!(["~/Documents", "~/Downloads"])), "2 个");
    assert_eq!(shown(0, json!(null)), "—", "没加的照旧写「—」");
    assert_eq!(shown(1, json!(null)), "3 个", "一个没去掉：出厂的全勾着");
    assert_eq!(shown(1, json!(["~/.gnupg"])), "2 个", "数勾着的");
    assert_eq!(
        shown(2, json!(["https://a.invalid", "https://b.invalid"])),
        "https://a.invalid、https://b.invalid",
        "别的列表照旧列"
    );
    // 受保护的路径（核心 5-17 下上）同一天照私密目录写个数。
    assert_eq!(shown(3, json!([".miyu/hooks"])), "1 个");
    assert_eq!(shown(4, json!([".envrc"])), "1 个", "数勾着的");
}
