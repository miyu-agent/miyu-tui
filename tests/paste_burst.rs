//! 伪终端里的端到端测试：不发括号粘贴的终端（Windows 上全是这样），粘进来的多行字是一口气到的一串按键（蓝图 `tui.md`
//! 「输入框」第 11 条）。整串当一次粘贴，中间的回车是换行，不拆成好几条发出去；人按的回车照旧是发送。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn lines_arriving_as_keys_stay_one_message() {
    let home = Home::new(Script::new([Play::Says("收到。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    // 不带括号粘贴的记号，三行一口气写进去：和终端不发括号粘贴时粘贴一样。
    tui.send(b"line one\rline two\rline three");
    tui.pump(Duration::from_millis(500));
    let shown = tui.lines().join("\n");
    assert!(
        !tui.shows("收到。"),
        "中间的回车不该把前面的发出去：\n{shown}"
    );
    for line in ["line one", "line two", "line three"] {
        assert!(
            tui.shows(line),
            "三行都还在输入框里（缺 {line}）：\n{shown}"
        );
    }
    // 人按的回车：发出去，三行是一句。
    tui.key(b"\r");
    tui.wait_for("收到。");
    let shown = tui.lines().join("\n");
    let user_lines = tui
        .lines()
        .iter()
        .filter(|l| l.contains("line one") || l.contains("line two") || l.contains("line three"))
        .count();
    assert_eq!(user_lines, 3, "发出去的一句里三行都在，只有一份：\n{shown}");
}
