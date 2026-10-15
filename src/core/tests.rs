use serde_json::json;

use super::request::request;
use super::{Command, Level, Update};

#[test]
fn switching_the_level_writes_only_what_changes() {
    // 「权限级别」第 2 条：工作区、开放权限写常用的那一级、关掉只读；只读只开只读。
    let (method, params) = request(Command::Level(Level::Full), "s1").unwrap();
    assert_eq!(method, "session.set_permission_level");
    assert_eq!(
        params,
        json!({"session": "s1", "level": "full", "read_only": false})
    );
    let (_, params) = request(Command::Level(Level::ReadOnly), "s1").unwrap();
    assert_eq!(params, json!({"session": "s1", "read_only": true}));
    let (_, params) = request(Command::Level(Level::Workspace), "s1").unwrap();
    assert_eq!(
        params,
        json!({"session": "s1", "level": "workspace", "read_only": false})
    );
}

#[test]
fn clear_goes_to_command_run_and_new_asks_for_nothing() {
    let (method, params) = request(Command::Run("/clear".into()), "s1").unwrap();
    assert_eq!(
        (method, params),
        ("command.run", json!({"session": "s1", "text": "/clear"}))
    );
    assert!(request(Command::New { keep: false }, "s1").is_none());
}

#[test]
fn a_goal_goes_to_command_run_with_what_follows() {
    // 蓝图「长期目标」第 1 条：照核心认，`/goal` 后面的字原样带上。
    let (method, params) = request(Command::Goal("写完测试".into()), "s1").unwrap();
    assert_eq!(
        (method, params),
        (
            "command.run",
            json!({"session": "s1", "text": "/goal 写完测试"})
        )
    );
    let (_, params) = request(Command::Goal(String::new()), "s1").unwrap();
    assert_eq!(params["text"], "/goal", "只打 /goal 是看进度");
}

#[test]
fn a_command_that_opens_a_page_says_so() {
    use serde_json::json;
    let opened = super::take::ran(
        &json!({"command": "week", "opens": {"package": "wx", "widget": "week"}, "said": ""}),
    );
    assert!(
        matches!(&opened, Update::OpenPage(p) if p.package == "wx" && p.widget == "week"),
        "{opened:?}"
    );
    let ran = super::take::ran(&json!({"command": "stop", "said": "已停止。"}));
    assert!(
        matches!(&ran, Update::CommandRan { command, said } if command == "stop" && said == "已停止。"),
        "{ran:?}"
    );
}
