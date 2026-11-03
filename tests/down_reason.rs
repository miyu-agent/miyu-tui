//! 伪终端里的端到端测试：连不上核心的原因写全（蓝图 `tui.md`「连核心」第 8 条，2026-11-03 项目主人：旧版数据根那句在左下角
//! 被截掉了后半句，选「首页中间红字折行写全」）：整句红字折行写在输入框上面，底栏只写前面那一截。

mod support;

#[test]
fn a_long_reason_for_being_down_is_written_in_full_above_the_input_box() {
    let base = std::env::temp_dir().join(format!("miyu-tui-down-{}", std::process::id()));
    let (home, work) = (base.join("home"), base.join("work"));
    std::fs::create_dir_all(&home).expect("建得了");
    std::fs::create_dir_all(&work).expect("建得了");
    // 不存在的核心程序，路径长得一行放不下：「找不到核心程序：<路径>」折成两行，最后一节在第二行。
    let missing = base
        .join("a-rather-long-directory-name-so-the-reason-wraps")
        .join("endmarker");
    let missing = missing.display().to_string();
    let mut tui = support::Tui::at(&home, &work, "zh_CN.UTF-8", &[("MIYU_CORE_BIN", &missing)]);
    tui.wait_for("endmarker");
    // 等底栏那一截也画出来：上面那几行先到、底栏还在路上时读屏只读到半截（2026-11-03 macOS CI 读到底栏只有「找」）。
    let end = std::time::Instant::now() + support::WAIT;
    while tui
        .lines()
        .iter()
        .filter(|l| l.contains("找不到核心程序"))
        .count()
        < 2
        && std::time::Instant::now() < end
    {
        tui.pump(std::time::Duration::from_millis(50));
    }
    let lines = tui.lines();
    let footer = lines
        .iter()
        .rev()
        .find(|l| l.contains("找不到核心程序"))
        .expect("底栏写着")
        .clone();
    assert!(
        !footer.contains("endmarker")
            && lines
                .iter()
                .filter(|l| l.contains("找不到核心程序"))
                .count()
                == 2,
        "上面写全、底栏只写前面那一截：\n{}",
        lines.join("\n")
    );
    drop(tui);
    drop(std::fs::remove_dir_all(&base));
}
