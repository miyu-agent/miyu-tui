//! 伪终端里的端到端测试：旧版的数据根（核心 FH-3，设计 55 第六节第 1 条，2026-11-03 core1）：核心不认、不动，界面把核心那句
//! 原话写出来（是旧版的、怎么办），不只说连不上。

mod support;

#[test]
fn an_old_data_root_is_named_as_such() {
    let base = std::env::temp_dir().join(format!("miyu-tui-old-root-{}", std::process::id()));
    let home = base.join("home");
    let work = base.join("work");
    std::fs::create_dir_all(home.join("state")).expect("建得了");
    std::fs::create_dir_all(&work).expect("建得了");
    // 布局 1、2 的标记（核心 `OLD_MARKER`），没有新版的 `.miyu-layout`。
    std::fs::write(
        home.join(".miyu-root"),
        "This directory is a Miyu data root.\n",
    )
    .expect("写得进去");
    let mut tui = support::Tui::at(&home, &work, "zh_CN.UTF-8", &[]);
    // 整句写全（「连核心」第 8 条子条）：后半句怎么办也看得到，底栏只写「连不上核心」。
    tui.wait_for("FH-7");
    for l in tui.lines() {
        println!("|{l}|");
    }
    assert!(
        tui.shows("旧版 Miyu 的数据根"),
        "{}",
        tui.lines().join("\n")
    );
    drop(tui);
    drop(std::fs::remove_dir_all(&base));
}
