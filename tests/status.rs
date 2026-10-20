//! 伪终端里的端到端测试：`/status` 会话状态框（蓝图 `tui.md`「会话状态 `/status`」，2026-10-20 项目主人看了样板定）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn status_lists_the_session_with_its_title_on_the_frame_and_closes_on_escape() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.say("/status");
    tui.wait_for("会话 ID");
    let screen = tui.lines().join("\n");
    for key in [
        "权限模式",
        "当前工作目录",
        "模型",
        "上下文",
        "用量",
        "缓存命中",
        "回忆 0 次 · 记住 0 条",
    ] {
        assert!(screen.contains(key), "少了「{key}」：\n{screen}");
    }
    // 框线上写会话标题，不写「会话状态」。
    let id_row = tui
        .lines()
        .iter()
        .position(|l| l.contains("会话 ID"))
        .unwrap();
    let frame = &tui.lines()[id_row - 2];
    assert!(
        frame.contains('╭') && !frame.contains("会话状态"),
        "{screen}"
    );
    assert!(
        tui.lines()[id_row - 1].trim_matches(['│', ' ']).is_empty(),
        "会话 ID 上面空一行：\n{screen}"
    );
    tui.key(b"\x1b");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("会话 ID") {
        assert!(
            std::time::Instant::now() < end,
            "Esc 关框：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

/// 框（框线上写着 `title`）里第一行、最后一行是空的。
fn padded(tui: &support::Tui, title: &str) -> bool {
    let lines = tui.lines();
    let Some(top) = lines
        .iter()
        .position(|l| l.contains('╭') && l.contains(title))
    else {
        return false;
    };
    let Some(bottom) = lines[top..]
        .iter()
        .position(|l| l.contains('╰'))
        .map(|b| top + b)
    else {
        return false;
    };
    let blank = |l: &str| l.trim_matches(['│', ' ']).is_empty();
    bottom > top + 2 && blank(&lines[top + 1]) && blank(&lines[bottom - 1])
}

#[test]
fn boxes_opened_by_a_command_have_a_blank_line_inside_top_and_bottom() {
    // 2026-10-20 项目主人：先给 /persona 加上下空行，看了又定别的框也加（打字时弹的列表不加）。
    let home = Home::new(Script::new([Play::Says("好。")]));
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", None);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    for (command, title) in [
        ("/persona", "请选择人格"),
        ("/theme", "配色"),
        ("/help", "帮助"),
    ] {
        tui.say(command);
        tui.wait_for(title);
        let end = std::time::Instant::now() + support::WAIT;
        while !padded(&tui, title) {
            assert!(
                std::time::Instant::now() < end,
                "{command} 的框上下没空行：\n{}",
                tui.lines().join("\n")
            );
            tui.pump(std::time::Duration::from_millis(50));
        }
        tui.key(b"\x1b");
        tui.pump(std::time::Duration::from_millis(200));
    }
}
