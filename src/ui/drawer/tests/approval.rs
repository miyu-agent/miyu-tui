//! 越过沙盒的命令确认：「此目录不再询问」照规则的命令前缀写清（核心 SB-3，反馈第 23 条）。

use super::lines;
use crate::drawer::Drawer;

fn outside(rule: serde_json::Value) -> Drawer {
    let approval = serde_json::from_value(serde_json::json!({
        "call_id": "c1", "access": "execute", "rule": rule,
        "detail": {"tool": "shell", "command": "docker ps -a"}
    }))
    .unwrap();
    Drawer::approval(None, approval)
}

#[test]
fn a_rule_with_a_prefix_names_the_commands_it_lets_through() {
    // 2026-10-27 项目主人照推荐定：「按命令前缀记住」；写成「以后 `docker ps` 开头的命令在这个目录不再询问」。
    let d = outside(
        serde_json::json!({"prefix": "docker ps", "tool": "shell", "write": ["/srv/proj"]}),
    );
    let shown = lines(&d, 80);
    assert!(
        shown
            .iter()
            .any(|l| l.ends_with("以后 `docker ps` 开头的命令在这个目录不再询问")),
        "{shown:#?}"
    );
    assert!(
        !shown
            .iter()
            .any(|l| l.ends_with("此目录不再询问") && !l.contains("docker"))
    );
    // 没有前缀的（连起来的几段不一样、包了 sudo 这类）照旧只写目录。
    let d = outside(serde_json::json!({"tool": "shell", "write": ["/srv/proj"]}));
    let shown = lines(&d, 80);
    assert!(
        shown.iter().any(|l| l.ends_with("2. 此目录不再询问")),
        "{shown:#?}"
    );
}
