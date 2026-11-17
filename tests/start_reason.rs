//! 伪终端里的端到端测试：核心起不来的每一步都带代码（核心 EN-2），原因只是会变的那一段（系统的原话、路径）。界面照它的语言写
//! 核心资源里的那一整句，不写取不到字时的中文原话「核心起不来（代码）：原因」。

mod support;

/// 假的核心：照核心起不来时写的那一行（`error <代码>: <原因>`，`miyu_ipc::Ready::line`）写完就退。
#[cfg(unix)]
fn failing_core(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let core = dir.join("miyu");
    std::fs::write(
        &core,
        "#!/bin/sh\necho 'error log_failed: File exists (os error 17)'\n",
    )
    .expect("写得进去");
    std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o755)).expect("改得了权限");
    core
}

#[cfg(unix)]
#[test]
fn a_core_that_cannot_start_is_said_whole_in_the_interface_language() {
    let base = std::env::temp_dir().join(format!("miyu-tui-start-reason-{}", std::process::id()));
    let (home, work, bin) = (base.join("home"), base.join("work"), base.join("bin"));
    for dir in [&home, &work, &bin] {
        std::fs::create_dir_all(dir).expect("建得了");
    }
    let core = failing_core(&bin);
    let core = core.display().to_string();
    let mut tui = support::Tui::at(&home, &work, "en_US.UTF-8", &[("MIYU_CORE_BIN", &core)]);
    tui.wait_for("File exists");
    let lines = tui.lines();
    assert!(
        tui.shows("cannot write its log") && !lines.iter().any(|l| l.contains("起不来")),
        "整句照英文界面写核心资源里的那一句：\n{}",
        lines.join("\n")
    );
    drop(tui);
    drop(std::fs::remove_dir_all(&base));
}
