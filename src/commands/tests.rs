//! 斜杠命令的清单（`commands.rs`）：认命令、筛、换上核心认的那一份。

use super::{Commands, Line, menu_typed, typed};

#[test]
fn a_command_that_takes_words_keeps_them() {
    let mut c = commands();
    c.take_core(&[]);
    match c.read("/compact 重点保留 数据库设计") {
        Line::Command(spec, Some(words)) => {
            assert_eq!(spec.name, "compact");
            assert_eq!(words, "重点保留 数据库设计");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(c.read("/compact"), Line::Command(s, None) if s.name == "compact"));
    assert!(
        matches!(c.read("/compact   "), Line::Command(_, None)),
        "只有空白：没写"
    );
    assert!(matches!(c.read("/undo"), Line::Command(s, None) if s.name == "undo"));
    assert!(
        matches!(c.read("/rewind"), Line::Command(s, None) if s.name == "undo"),
        "别名"
    );
    assert!(
        matches!(c.read("/theme 深色"), Line::Talk),
        "不带参数的命令后面跟了字：一句话"
    );
    assert!(
        matches!(c.read("/etc 目录是干什么的"), Line::Talk),
        "不弹命令不存在"
    );
    assert!(matches!(c.read("/nosuch"), Line::Unknown));
    assert!(matches!(c.read("你好"), Line::Talk));
    assert!(matches!(c.read("/你吃了吗"), Line::Talk));
    assert!(matches!(c.read("/ compact"), Line::Talk), "名字要紧挨着 /");
}

/// 核心的清单还没到：认不出的 `/名字 字` 交给核心认（刚连上就打 `/loop 10m`），到了以后照旧当一句话。
#[test]
fn before_the_core_list_arrives_an_unknown_name_with_words_goes_to_the_core() {
    let mut c = commands();
    assert!(matches!(c.read("/loop 10m 看一眼 CI"), Line::Unknown));
    assert!(matches!(c.read("你好"), Line::Talk));
    c.take_core(&[]);
    assert!(matches!(c.read("/etc 目录是干什么的"), Line::Talk));
}

#[test]
fn typing_words_after_the_name_closes_the_list() {
    assert_eq!(menu_typed("/comp"), Some("comp"));
    assert_eq!(menu_typed("/"), Some(""));
    assert_eq!(menu_typed("/compact "), None, "打了空格：名字打全了");
    assert_eq!(menu_typed("/compact 重点"), None);
    assert_eq!(menu_typed("/你吃了吗"), None);
    assert_eq!(menu_typed("/ compact"), None, "名字要紧挨着 /");
}
use crate::config::Config;

fn commands() -> Commands {
    Config::builtin().unwrap().commands
}

#[test]
fn prefix_matches_come_first() {
    let commands = commands();
    let names: Vec<_> = commands
        .filter("e")
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        names[..4],
        ["edit", "effort", "export", "exit"],
        "开头对得上的在前，照登记的先后"
    );
    assert!(names.contains(&"redo"), "含着 e 的也在：{names:?}");
}

#[test]
fn clear_can_also_be_typed_as_reset() {
    // 2026-09-30 项目主人：/clear 别名 /reset，会话不变、清空上下文。
    let commands = commands();
    assert_eq!(
        commands.find("clear").map(|s| s.run),
        Some(super::Run::Clear)
    );
    assert_eq!(
        commands.find("reset").map(|s| s.run),
        Some(super::Run::Clear)
    );
}

#[test]
fn undo_and_restore_follow_the_core_names() {
    // 2026-09-29 跟 main 的命令改名：恢复叫 /restore，撤销也可以打 /rewind。/redo 09-30 回来了，换成核心的
    // 重做（施工 4-7 再补），不是恢复。
    let commands = commands();
    assert_eq!(
        commands.find("restore").map(|s| s.run),
        Some(super::Run::Unrevert)
    );
    assert_eq!(
        commands.find("rewind").map(|s| s.run),
        Some(super::Run::Revert)
    );
    assert_eq!(commands.find("redo").map(|s| s.run), Some(super::Run::Redo));
    let names: Vec<_> = commands
        .filter("rew")
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, ["undo"], "筛的时候别名也算，列表里写正名");
}

#[test]
fn paths_are_not_commands() {
    assert_eq!(typed("/undo"), Some("undo"));
    assert_eq!(typed("/"), Some(""));
    assert_eq!(typed("/home/me/a.txt"), None);
    assert_eq!(typed("undo"), None);
    // 不像命令名的不算命令，照普通的话发（`tui.md`「斜杠命令列表」第 1、4 条）。
    assert_eq!(typed("/你吃了吗"), None);
    assert_eq!(typed("/1st"), None);
    assert_eq!(typed("/foo-bar_2"), Some("foo-bar_2"));
    assert_eq!(typed("/foo 带参数"), Some("foo"));
}

#[test]
fn the_cores_commands_follow_ours_without_the_ones_we_have() {
    // 核心 X-14：模板、扩展声明的接在终端自己的后面；终端有的名字、别名不要；能接参数。
    let mut c = commands();
    let core = |name: &str, aliases: &[&str]| crate::core::CoreCommand {
        name: name.into(),
        aliases: aliases.iter().map(ToString::to_string).collect(),
        summary: format!("{name} 的说明"),
        argument: (name == "review").then(|| "<路径>".to_string()),
        opens: None,
        usage: Vec::new(),
    };
    c.take_core(&[
        core("clear", &[]),
        core("review", &[]),
        core("tidy", &["reset"]),
        core("remember", &[]),
    ]);
    let names: Vec<&str> = c.core.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["review", "remember"],
        "同名、别名撞了终端自己的都不要"
    );
    let review = c.find("review").expect("找得到");
    assert_eq!((review.run, review.args), (super::Run::Core, true));
    assert_eq!(review.argument.as_deref(), Some("<路径>"));
    assert!(
        matches!(c.read("/review src"), Line::Command(s, Some("src")) if s.name == "review"),
        "接参数"
    );
    let filtered: Vec<&str> = c.filter("re").iter().map(|s| s.name.as_str()).collect();
    let at = |n: &str| filtered.iter().position(|f| *f == n);
    assert!(
        at("review") > at("restore"),
        "接在终端自己的后面：{filtered:?}"
    );
}

/// 2026-11-13 主人验收 1113：`/stop` 写的还是终端自己那句旧说明。核心清单列了的命令说明照核心，没列的照终端自己的。
#[test]
fn a_core_listed_command_shows_the_cores_summary() {
    let mut c = commands();
    let own = c.find("stop").expect("终端有 /stop").summary.clone();
    let stop = crate::core::CoreCommand {
        name: "stop".into(),
        aliases: Vec::new(),
        summary: "停止此会话所有处理中的东西".into(),
        argument: None,
        opens: None,
        usage: Vec::new(),
    };
    c.take_core(std::slice::from_ref(&stop));
    assert_eq!(c.find("stop").unwrap().said(), "停止此会话所有处理中的东西");
    // 核心清单没列它（旧核心、换了会话）：回到终端自己的
    c.take_core(&[]);
    assert_eq!(c.find("stop").unwrap().said(), own);
    assert_eq!(
        c.find("theme").unwrap().said(),
        c.find("theme").unwrap().summary,
        "终端自己的命令照 commands.json"
    );
}

#[test]
fn our_own_goal_takes_the_argument_and_usage_from_the_core() {
    // 2026-10-17 项目主人：菜单照 shell 的语法提要，两个头别各拼各的；终端自己有的 /goal 照核心那条的参数提示、几种写法。
    let mut c = commands();
    let goal = crate::core::CoreCommand {
        name: "goal".into(),
        aliases: Vec::new(),
        summary: "核心的说明".into(),
        argument: Some("[<目标> | pause | resume | clear]".into()),
        opens: None,
        usage: vec![
            ("/goal".into(), "看进度".into()),
            ("/goal pause".into(), "暂停".into()),
        ],
    };
    c.take_core(std::slice::from_ref(&goal));
    let ours = c.find("goal").expect("终端自己的");
    assert_eq!(ours.run, super::Run::Goal, "还是终端自己的");
    assert_eq!(
        ours.said(),
        "核心的说明",
        "属于核心的命令说明照核心清单（2026-11-13 主人验收 1113）"
    );
    assert_eq!(
        ours.argument.as_deref(),
        Some("[<目标> | pause | resume | clear]")
    );
    assert_eq!(ours.usage.len(), 2);
    assert!(c.core.is_empty(), "核心那条不另列");
    // 核心清单里没有（长期目标那个包卸掉了，核心 BS-1下下）：列表里不列，打了交给核心，带参数的也不当一句话。
    c.take_core(&[]);
    assert!(c.find("goal").is_none());
    assert!(!c.filter("go").iter().any(|s| s.name == "goal"));
    assert!(matches!(c.read("/goal"), Line::Unknown));
    assert!(matches!(c.read("/goal 写完报告"), Line::Unknown));
    // 又装上了：回来。
    c.take_core(std::slice::from_ref(&goal));
    assert!(c.find("goal").is_some());
}

#[test]
fn a_colon_inside_the_name_is_part_of_it() {
    // 核心 X-39：MCP 服务的提示词做成斜杠命令 `<服务名>:<提示词名>`（2026-10-21 核心会话告知）。
    assert_eq!(typed("/github:review-pr 123"), Some("github:review-pr"));
    assert_eq!(typed("/github:"), Some("github:"), "打到一半也开着列表");
    assert_eq!(typed("/:x"), None, "冒号不能打头");
    assert_eq!(typed("/a:b:c"), None, "最多一个冒号（同网页）");
}
