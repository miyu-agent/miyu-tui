//! 伪终端里的端到端测试：旧版的数据根（核心 FH-3，设计 55 第六节第 1 条，2026-11-03 core1）：核心不认、不动，界面照它的语言把那句
//! 话写出来（是旧版的、怎么办；排期第 43 条），不只说连不上。

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
    // 整句写全（「连核心」第 8 条子条）：后半句怎么办也看得到，底栏只写「连不上核心」。照界面的语言写核心资源里的那一句
    // （排期第 43 条），不是核心的原话（原话的后半句是「照施工单 FH-7 自己搬」）。
    tui.wait_for("迁移程序");
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

#[test]
fn an_old_data_root_is_explained_in_the_interface_language() {
    // 排期第 43 条（核心 LC-1）：英文界面写核心资源里英文的那一句，不写核心的中文原话；数据根一个字节都没动。
    let base = std::env::temp_dir().join(format!("miyu-tui-old-root-en-{}", std::process::id()));
    let (home, work) = (base.join("home"), base.join("work"));
    std::fs::create_dir_all(&home).expect("建得了");
    std::fs::create_dir_all(&work).expect("建得了");
    std::fs::write(home.join(".miyu-root"), "").expect("写得进去");
    let mut tui = support::Tui::at(&home, &work, "en_US.UTF-8", &[("MIYU_CORE_BIN", "miyu")]);
    tui.wait_for("migration");
    let lines = tui.lines();
    assert!(
        !lines.iter().any(|l| l.contains("旧版")),
        "不写核心的中文原话：\n{}",
        lines.join("\n")
    );
    drop(tui);
    assert!(!home.join(".miyu-layout").exists(), "旧版的数据根没动");
    drop(std::fs::remove_dir_all(&base));
}
