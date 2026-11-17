//! 伪终端里的端到端测试：核心起不来的每一步都带代码（核心 EN-2），原因只是会变的那一段（系统的原话、路径）。界面照它的语言写
//! 核心资源里的那一整句，不写取不到字时的中文原话「核心起不来（代码）：原因」；前面也不套「连不上核心」（2026-11-17 项目主人
//! 定：首页、shellhook、doctor、斜杠命令都只写核心那一句）。

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

/// 照英文界面起 `args`（空的是全屏的首页），核心起不来：等原因写出来，交回屏上的几行。
#[cfg(unix)]
fn said_when_the_core_cannot_start(name: &str, args: &[&str]) -> Vec<String> {
    let base = std::env::temp_dir().join(format!(
        "miyu-tui-start-reason-{name}-{}",
        std::process::id()
    ));
    let (home, work, bin) = (base.join("home"), base.join("work"), base.join("bin"));
    // 数据根单放一个空目录：测具把界面的缓存目录（`XDG_CACHE_HOME`）也放在 `home` 里，连核心以前里面先有了东西的，数据根
    // 不空、认不出是 Miyu 的，等不到核心起不来的那一句（2026-11-17 Ubuntu CI 上 doctor 撞见）。
    let root = base.join("root");
    for dir in [&home, &work, &bin, &root] {
        std::fs::create_dir_all(dir).expect("建得了");
    }
    let core = failing_core(&bin).display().to_string();
    let root = root.display().to_string();
    if let Some(input) = args.iter().position(|a| *a == "--input") {
        std::fs::write(work.join(args[input + 1]), "hi").expect("写得进去");
    }
    let mut tui = support::Tui::program(
        &home,
        &work,
        "en_US.UTF-8",
        &[("MIYU_CORE_BIN", &core), ("MIYU_HOME", &root)],
        args,
        env!("CARGO_BIN_EXE_miyu-tui"),
    );
    tui.wait_for("File exists");
    let lines = tui.lines();
    drop(tui);
    drop(std::fs::remove_dir_all(&base));
    lines
}

/// 只写核心那一整句：英文、不带中文，前面不套「连不上核心」。
#[cfg(unix)]
fn only_the_core_sentence(lines: &[String]) {
    let text = lines.join("\n");
    assert!(
        text.contains("cannot write its log")
            && !text.contains("起不来")
            && !text.contains("Can't connect")
            && !text.contains("Can't reach"),
        "只写核心资源里的那一整句：\n{text}"
    );
}

#[cfg(unix)]
#[test]
fn a_core_that_cannot_start_is_said_whole_in_the_interface_language() {
    only_the_core_sentence(&said_when_the_core_cannot_start("home", &[]));
}

#[cfg(unix)]
#[test]
fn the_shellhook_says_only_the_core_sentence() {
    let args = ["shellhook", "say", "--input", "said"];
    only_the_core_sentence(&said_when_the_core_cannot_start("say", &args));
}

#[cfg(unix)]
#[test]
fn doctor_says_only_the_core_sentence() {
    let lines = said_when_the_core_cannot_start("doctor", &["shellhook", "doctor"]);
    only_the_core_sentence(&lines);
}

#[cfg(unix)]
#[test]
fn the_tui_slash_command_says_only_the_core_sentence() {
    let lines = said_when_the_core_cannot_start("slash", &["shellhook", "tui"]);
    only_the_core_sentence(&lines);
}
