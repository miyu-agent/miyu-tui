//! 在跑的那一步的进度（蓝图 `tui.md`「时间线」第 8a 条，2026-10-26 shorin-03 转来，同网页 W-372）：标题后面暗色接最新的
//! 一行 `miyu-progress: `，原地刷新，有了结果就去掉。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 屏上写着 `text` 的那几行。
fn rows_with(tui: &Tui, text: &str) -> Vec<String> {
    tui.lines()
        .into_iter()
        .filter(|l| l.contains(text))
        .collect()
}

#[test]
fn a_running_step_shows_its_latest_progress_in_place_until_the_result() {
    // 输出的字照 `$i` 拼出来：命令预览里只有 `第 $i 步`、`普通$i输出`，屏上的「第 1 步」只能是进度，「普通1输出」只能是输出。
    let command =
        "for i in 1 2; do echo 普通$i输出; echo \"miyu-progress: 第 $i 步\"; sleep 1.5; done";
    let args = serde_json::json!({"command": command, "description": "分两步"}).to_string();
    let script = Script::new([
        Play::Calls(vec![("shell".into(), args)]),
        Play::Says("两步都好了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("跑一下");
    tui.wait_for("第 1 步");
    let rows = rows_with(&tui, "第 1 步");
    assert_eq!(rows.len(), 1, "{}", tui.lines().join("\n"));
    assert!(
        rows[0].contains("分两步 · 第 1 步"),
        "接在那一步的标题后面：{}",
        rows[0]
    );
    assert!(
        rows_with(&tui, "普通1输出").is_empty(),
        "不是进度行的不写：\n{}",
        tui.lines().join("\n")
    );
    tui.wait_for("第 2 步");
    assert!(
        rows_with(&tui, "第 1 步").is_empty(),
        "原地换成最新的一行：\n{}",
        tui.lines().join("\n")
    );
    tui.wait_for("两步都好了。");
    tui.pump(Duration::from_millis(300));
    assert!(
        rows_with(&tui, "第 2 步").is_empty(),
        "有了结果就去掉：\n{}",
        tui.lines().join("\n")
    );
}
