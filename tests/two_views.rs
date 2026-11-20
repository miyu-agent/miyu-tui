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
    first.wait_for("开放权限");
    first.say("看看笔记");
    first.wait_for_line("…");
    // 后开的在别的目录（herdr 的窗格）：不写「工作区：…」那一句（2026-10-18 项目主人），正在跑的这一轮照样接上。
    let elsewhere = home.work.join("sub");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let mut second = home.tui_wide_in("zh_CN.UTF-8", 100, &elsewhere);
    second.wait_for("看看笔记");
    second.pump(Duration::from_millis(500));
    assert!(
        !second.shows("切换到当前目录"),
        "{}",
        second.lines().join("\n")
    );
    // 接进来以前她已经说出来的（核心 9-8 八补：订阅时补推正在流的块）：后开的也有，那一段照她开口收起，和先开的一样。
    second.wait_for_line("…");
    assert!(
        first.shows("Used 1 tool") && second.shows("Used 1 tool"),
        "两边一样：\n{}\n---\n{}",
        first.lines().join("\n"),
        second.lines().join("\n")
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

/// 另一个终端新开的会话答完了：这边不弹「回答完了」（核心 HV-1：开着它的那个头负责）。2026-11-20 量内存时撞见：答得快的一轮
/// 在开会话的终端说「开着它」以前就答完了，别的终端个个当成后台会话弹一条，十个窗口各说一句弹了 90 条。
#[test]
fn a_session_answered_in_another_terminal_is_not_announced_here() {
    let home = Home::new(Script::new([Play::Says("答完了。")]));
    let mut here = home.tui("zh_CN.UTF-8");
    here.wait_for("开放权限");
    let mut there = home.tui("zh_CN.UTF-8");
    there.wait_for("开放权限");
    there.say("在吗");
    there.wait_for("答完了。");
    // 这边照会话列表看到那个会话忙过又答完：等它推过来。
    here.pump(Duration::from_secs(3));
    let log = std::fs::read_to_string(support::notify_log(home.root())).unwrap_or_default();
    assert!(
        !log.lines()
            .any(|l| l.starts_with("notify-send") || l.starts_with("osascript")),
        "开着它的是另一个终端，这边不弹：\n{log}"
    );
}
