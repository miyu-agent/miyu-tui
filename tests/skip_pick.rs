//! 伪终端里的端到端测试：新会话不再询问人格预设（蓝图 `tui.md`「新会话：人格、工作区」第 1a 条）。开着时起来不闪选择框：
//! 等默认的读到以前照旧画输入框（2026-10-23 项目主人：「我打开 tui 还是会有一瞬间看到人格预设的选择框」）。

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn with_skip_pick_on_no_frame_shows_the_pick_box() {
    let home = Home::with_settings(Script::new([]), "[tui]\nskip_pick = true\n");
    // 两回：第一回没有记下的样子，第二回照记下的先画（`look.rs`）。
    for round in 0..2 {
        // 测具默认替人选好人格、预设；这里清掉，走真的新会话那一套。
        let mut tui = home.tui_with(
            "zh_CN.UTF-8",
            &[("MIYU_TUI_PERSONA", ""), ("MIYU_TUI_PRESET", "")],
        );
        let start = tui.record_from_here();
        tui.wait_for("工作区");
        tui.pump(Duration::from_millis(800));
        let recorded = tui.recorded();
        for frame in support::frames(start, &recorded) {
            assert!(
                !frame
                    .iter()
                    .any(|l| l.contains("请选择人格") || l.contains("请选择预设")),
                "第 {round} 回有一帧画了选择框：\n{}",
                frame.join("\n")
            );
        }
        assert!(tui.shows("Tab 切换权限级别"), "{}", tui.lines().join("\n"));
    }
}

#[test]
fn ctrl_c_and_ctrl_d_quit_from_the_pick_box_that_replaces_the_input() {
    // 2026-10-23 项目主人：「这个界面我期望能 ctrl+C/Ctrl+D 退出 tui」。
    let home = Home::new(Script::new([]));
    for key in [b"\x03", b"\x04"] {
        // 测具的家里没有人格（人格框不问），清掉替人选的预设，开的是预设框。
        let mut tui = home.tui_with("zh_CN.UTF-8", &[("MIYU_TUI_PRESET", "")]);
        // 框一出来就按：列表还在读（写「正在加载…」）时也照样退出。
        tui.wait_for("请选择预设");
        tui.key(key);
        assert!(tui.exited(), "{key:?} 没退出：\n{}", tui.lines().join("\n"));
    }
}
