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

/// 提示框浮在在跑的那一步上（2026-11-02 项目主人报：「盲文会导致通知错位」）：框占的那几行整行清掉，框左边不露转圈、右边
/// 不露被盖住那一步的字。窄窗口走紧凑版面，框左边一列正好是时间线的记号列。
#[test]
fn a_toast_over_a_running_step_clears_its_rows() {
    let command =
        "for i in 1 2 3; do echo 普通$i输出; echo \"miyu-progress: 第 $i 步\"; sleep 1.5; done";
    let args = serde_json::json!({"command": command, "description": "分两步"}).to_string();
    let long: String = (1..=60).map(|i| format!("第{i}行\n\n")).collect();
    let script = Script::new([
        Play::Says(Box::leak(long.into_boxed_str())),
        Play::Calls(vec![("shell".into(), args)]),
        Play::Says("好了。"),
    ]);
    let home = Home::with_tools(script, "[permission]\nlevel = \"full\"\n");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 80);
    tui.wait_for("开放权限");
    tui.say("先说很多");
    tui.wait_for("第60行");
    tui.say("跑一下");
    tui.wait_for("第 1 步");
    tui.key(b"\x1b");
    tui.wait_for("再按一次");
    let lines = tui.lines();
    let at = lines.iter().position(|l| l.contains("再按一次")).unwrap();
    let braille = |c: char| ('\u{2800}'..='\u{28ff}').contains(&c);
    for row in &lines[at - 1..=at + 1] {
        assert!(
            !row.chars().any(braille) && !row.contains("echo") && !row.contains("第 1 步"),
            "框那几行只有框：{row}\n{}",
            lines.join("\n")
        );
    }
}
