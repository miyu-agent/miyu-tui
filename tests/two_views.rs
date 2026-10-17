//! 同一个会话开两个终端（2026-10-17 项目主人报：外面的 kitty 跑着一轮，herdr 里再开同一个会话，后开的把正在跑的这一轮
//! 收成一行、之后不再刷新）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_second_view_opened_mid_turn_keeps_the_turn_open_and_keeps_up() {
    let args = serde_json::json!({"path": "notes.md"}).to_string();
    let home = Home::with_tools(
        Script::new([
            Play::Calls(vec![("read".into(), args)]),
            Play::Holds,
            Play::Says("接着说完了。"),
        ]),
        "[ui]\nstartup = \"recent\"\n",
    );
    std::fs::write(home.work.join("notes.md"), "第一行\n").unwrap();
    let mut first = home.tui("zh_CN.UTF-8");
    first.wait_for("工作区");
    first.say("看看笔记");
    first.wait_for_line("…");
    // 后开的在别的目录（herdr 的窗格）：要写一句「工作区：…」。
    let elsewhere = home.work.join("sub");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let mut second = home.tui_wide_in("zh_CN.UTF-8", 100, &elsewhere);
    second.wait_for("切换到当前目录");
    second.pump(Duration::from_millis(500));
    let lines = second.lines();
    let row = |word: &str| lines.iter().position(|l| l.contains(word));
    let (note, said) = (row("切换到当前目录"), row("看看笔记"));
    assert!(
        note.is_some() && note < said,
        "那一句插在正在跑的这一轮上面：\n{}",
        lines.join("\n")
    );
    assert!(
        !second.shows("Used 1 tool") && second.shows("读取"),
        "正在跑的那一段不收起：\n{}",
        lines.join("\n")
    );
    // 之后照样跟得上：先开的打断、再说一句。
    first.key(b"\x1b");
    first.pump(Duration::from_millis(100));
    first.key(b"\x1b");
    first.wait_for("已中断");
    second.wait_for("已中断");
    first.say("继续");
    first.wait_for("接着说完了。");
    second.wait_for("接着说完了。");
}
