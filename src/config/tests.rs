//! 出厂资源读得进来、几句提示的写法。

use super::Config;

#[test]
fn builtin_resources_parse() {
    let config = Config::builtin().unwrap();
    assert!(config.layout.pad_right >= 1, "行尾的光标要有地方待");
    assert!(config.layout.max_rows >= 1);
    assert!((1..=100).contains(&config.layout.width_percent));
    // 每一档都有图标；只读是暂停符号（蓝图「权限级别」第 3 条）。
    let icons = &config.layout.level_icons;
    assert!(
        config
            .layout
            .level_cycle
            .iter()
            .all(|l| icons.contains_key(l))
    );
    assert_eq!(
        icons.get(&crate::core::Level::ReadOnly).map(String::as_str),
        Some("⏸ ")
    );
}

#[test]
fn a_sealed_session_says_so_in_a_hint() {
    // 核心 C-8 下（2026-10-11）：分叉出来的会话跑满轮数以后封存，往它发话拒 `session_sealed`。
    let text = Config::builtin().unwrap().text;
    assert_eq!(
        text.refusal_hints.get("session_sealed").map(String::as_str),
        Some("会话已封存")
    );
}

#[test]
fn nothing_to_compact_is_a_short_hint_not_a_line() {
    // 2026-09-30 项目主人：没必要在正文里打一行，给个简短的通知就行。
    let text = Config::builtin().unwrap().text;
    assert_eq!(
        text.refusal_hints
            .get("nothing_to_compact")
            .map(String::as_str),
        Some("没有可压缩的内容")
    );
}

#[test]
fn no_text_speaks_of_accounts_or_personal_settings() {
    // 2026-10-28 项目主人定只给一个人用（核心 SU-1 到 SU-6）：没有账号、管理员、个人设置这一层了，界面上的字也不许再提。
    let words = [
        "个人设置",
        "终端管理员",
        "管理员模式",
        "账号",
        "personal setting",
        "admin mode",
        "account",
        "個人設定",
        "管理者",
        "アカウント",
    ];
    for (name, json) in [
        ("zh", include_str!("../../resources/text/zh.json")),
        ("en", include_str!("../../resources/text/en.json")),
        ("ja", include_str!("../../resources/text/ja.json")),
        ("commands", include_str!("../../resources/commands.json")),
    ] {
        let lower = json.to_lowercase();
        let found: Vec<_> = words.iter().filter(|w| lower.contains(*w)).collect();
        assert!(found.is_empty(), "{name} 还在提：{found:?}");
    }
}

#[test]
fn only_the_new_tool_names_are_known() {
    // 2026-10-01 核心改名 agent → subagent、message_agent → send_message，同一天项目主人定不认旧名、不留兼容。
    let config = Config::builtin().unwrap();
    let kinds = &config.timeline.kinds;
    assert!(kinds.get("subagent").is_some() && kinds.get("send_message").is_some());
    assert!(kinds.get("agent").is_none() && kinds.get("message_agent").is_none());
    assert_eq!(
        config.icons.tool("agent"),
        config.icons.tool("没登记的工具")
    );
    assert_ne!(
        config.icons.tool("subagent"),
        config.icons.tool("没登记的工具")
    );
}

#[test]
fn loading_does_not_eat_the_callers_stack() {
    // 界面上的字那几份 serde 拆起来一层层的栈帧近 300 KB，读配置另起线程拆（施工 TM-4）：调它的线程只有 64 KB 的栈也读得了。
    // 改回在调用方的线程上拆，这里当场栈溢出。
    let reading = std::thread::Builder::new()
        .stack_size(192 * 1024)
        .spawn(Config::builtin)
        .unwrap();
    let config = reading.join().unwrap().unwrap();
    assert_eq!(config.language.code(), "zh");
}
