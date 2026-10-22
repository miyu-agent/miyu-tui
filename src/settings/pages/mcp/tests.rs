//! 「MCP」那一页（「配置页」第 43b 条，2026-10-22 项目主人定）：服务两处合成一份，要登录的 `Enter` 去要授权页、交给 App 开。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::{Value, json};

use super::super::{Entry, Section};
use crate::config::Config;
use crate::core::Refusal;
use crate::settings::test_support::{config_all, model_list};
use crate::settings::{Settings, Tone};

/// 个人设置里两个服务：`notes` 要登录，`ctx` 在跑、两件工具。
fn home() -> Value {
    json!({"servers": [
        {"name": "notes", "url": "https://notes.invalid/mcp", "state": "stopped", "reason": "needs_login"},
        {"name": "ctx", "command": ["ctx-mcp"], "state": "running", "tools": ["mcp__ctx__a", "mcp__ctx__b"]}]})
}

/// 包里带两个：同名的 `ctx` 被个人设置里的盖住，`fs` 在跑。
fn packages() -> Value {
    json!({"packages": [{"package": "files", "name": "文件", "mcp": [
        {"name": "ctx", "state": "stopped", "reason": "failed_repeatedly"},
        {"name": "fs", "state": "running", "tools": ["mcp__fs__read"]}]}]})
}

fn opened(mcp: Result<Value, Refusal>) -> (Settings, Config) {
    let config = Config::builtin().unwrap();
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => Ok(model_list()),
            "config.get" => Ok(config_all()),
            "config.schema" => Ok(json!({"pages": [{"id": "packages", "name": "软件包"}],
                "groups": [{"id": "files", "name": "文件", "page": "packages"}],
                "items": [{"key": "files.x", "control": "text", "layers": ["personal"], "name": "x",
                           "page": "packages", "group": "files"}]})),
            "package.list" => Ok(packages()),
            "mcp.list" => mcp.clone(),
            _ => Ok(json!({"secrets": []})),
        };
        page.answer(tag, got, &config.text.settings);
    }
    (page, config)
}

fn press(page: &mut Settings, config: &Config, code: KeyCode) {
    page.key(
        KeyEvent::new(code, KeyModifiers::NONE),
        &config.text.settings,
    );
}

/// 进 MCP 那一页。
fn enter_mcp(page: &mut Settings, config: &Config) {
    let at = page.more.entries().iter().position(|e| *e == Entry::Mcp);
    page.more.menu_at = at.expect("主菜单列了 MCP");
    press(page, config, KeyCode::Enter);
    assert_eq!(page.more.section, Some(Section::Mcp(0)));
}

#[test]
fn servers_from_settings_and_packages_are_one_list_above_packages() {
    let (page, _) = opened(Ok(home()));
    let names: Vec<(String, bool)> = page
        .more
        .servers()
        .into_iter()
        .map(|s| (s.name.clone(), s.needs_login()))
        .collect();
    assert_eq!(
        names,
        [
            ("notes".to_string(), true),
            ("ctx".to_string(), false),
            ("fs".to_string(), false),
        ],
        "个人设置里的在前，包里同名的 ctx 不再列"
    );
    let entries = page.more.entries();
    let mcp = entries.iter().position(|e| *e == Entry::Mcp).unwrap();
    assert_eq!(
        entries[mcp + 1],
        Entry::Page("packages".into()),
        "放在软件包上面"
    );
}

#[test]
fn an_old_core_without_mcp_list_still_lists_the_packaged_servers() {
    let refused = Refusal {
        reason: Some("unknown_method".into()),
        message: "unknown method".into(),
        data: Value::Null,
    };
    let (page, _) = opened(Err(refused));
    let names: Vec<String> = page.more.servers().into_iter().map(|s| s.name).collect();
    assert_eq!(names, ["ctx", "fs"]);
}

#[test]
fn enter_on_a_server_that_needs_login_asks_for_the_page_and_hands_it_to_the_app() {
    let (mut page, config) = opened(Ok(home()));
    enter_mcp(&mut page, &config);
    // 在跑的那一行 Enter 不做什么。
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Enter);
    assert!(page.take_asks().is_empty(), "在跑的不登录");
    press(&mut page, &config, KeyCode::Up);
    press(&mut page, &config, KeyCode::Enter);
    let asks = page.take_asks();
    assert_eq!(asks.len(), 1, "{asks:?}");
    let (tag, method, params) = &asks[0];
    assert_eq!(*method, "mcp.login");
    assert_eq!(params, &json!({"service": "notes"}));
    assert!(matches!(page.status, Some((_, Tone::Busy))));
    let url = "https://auth.invalid/authorize?client_id=x&state=y";
    page.answer(*tag, Ok(json!({"url": url})), &config.text.settings);
    assert_eq!(page.take_opening().as_deref(), Some(url), "交给 App 开");
    assert_eq!(page.take_opening(), None, "只交一次");
    // App 开不了：状态行写出网址。
    page.open_failed(url, &config.text.settings);
    let (said, _) = page.status.clone().unwrap();
    assert!(said.contains(url), "{said}");
}

#[test]
fn a_refused_login_says_why_on_the_status_line() {
    let (mut page, config) = opened(Ok(home()));
    enter_mcp(&mut page, &config);
    for (reason, data, expect) in [
        (
            "oauth_failed",
            json!({"why": "invalid_client"}),
            "登录失败：invalid_client",
        ),
        (
            "oauth_unregistered",
            Value::Null,
            config.text.settings.more.mcp.unregistered.as_str(),
        ),
        ("unknown_server", Value::Null, "没有这个服务"),
    ] {
        press(&mut page, &config, KeyCode::Enter);
        let (tag, _, _) = page.take_asks().remove(0);
        let refusal = Refusal {
            reason: Some(reason.into()),
            message: "没有这个服务".into(),
            data,
        };
        page.answer(tag, Err(refusal), &config.text.settings);
        assert_eq!(page.take_opening(), None);
        assert_eq!(
            page.status.clone(),
            Some((expect.to_string(), Tone::Bad)),
            "{reason}"
        );
    }
}

#[test]
fn a_tools_change_rereads_the_servers_and_a_signed_in_one_stops_asking() {
    let (mut page, config) = opened(Ok(home()));
    page.tools_changed();
    let asks = page.take_asks();
    let methods: Vec<&str> = asks.iter().map(|(_, m, _)| *m).collect();
    assert_eq!(methods, ["mcp.list", "package.list"]);
    for (tag, method, _) in asks {
        let got = if method == "mcp.list" {
            json!({"servers": [{"name": "notes", "state": "running", "tools": ["mcp__notes__find"]}]})
        } else {
            packages()
        };
        page.answer(tag, Ok(got), &config.text.settings);
    }
    let notes = page.more.servers().remove(0);
    assert_eq!(
        (notes.name.as_str(), notes.needs_login(), notes.tools),
        ("notes", false, Some(1))
    );
}

#[test]
fn signing_out_says_so_and_a_server_not_signed_in_says_why() {
    let signed_in = json!({"servers": [
        {"name": "notes", "url": "https://notes.invalid/mcp", "state": "running", "tools": ["a"], "logged_in": true}]});
    let (mut page, config) = opened(Ok(signed_in));
    assert!(page.more.servers()[0].logged_in, "读到 logged_in");
    enter_mcp(&mut page, &config);
    press(&mut page, &config, KeyCode::Enter);
    let (tag, method, params) = page.take_asks().remove(0);
    assert_eq!(
        (method, params),
        ("mcp.logout", json!({"service": "notes"}))
    );
    assert!(matches!(page.status, Some((_, Tone::Busy))));
    page.answer(tag, Ok(json!({})), &config.text.settings);
    assert_eq!(
        page.status.clone(),
        Some(("已登出 notes".to_string(), Tone::Note))
    );
    press(&mut page, &config, KeyCode::Enter);
    let (tag, _, _) = page.take_asks().remove(0);
    let refusal = Refusal {
        reason: Some("not_logged_in".into()),
        message: "not logged in".into(),
        data: Value::Null,
    };
    page.answer(tag, Err(refusal), &config.text.settings);
    assert_eq!(
        page.status.clone(),
        Some(("notes 没有登录过".to_string(), Tone::Bad))
    );
    // 没带这一格的（没登录过、在跑的）Enter 不做什么。
    let (mut page, config) = opened(Ok(home()));
    enter_mcp(&mut page, &config);
    press(&mut page, &config, KeyCode::Down);
    press(&mut page, &config, KeyCode::Enter);
    assert!(page.take_asks().is_empty());
}
