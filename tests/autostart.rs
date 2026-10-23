//! 伪终端里的端到端测试：配置页的「开机启动」（蓝图 `tui.md`「配置页」第 49 条，核心 Z-1 下）。核心程序换成假的：只记下
//! 参数、照参数印一行 JSON，不碰本机真的服务管理器。

mod support;

use std::path::Path;
use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::{Home, Tui};

/// 假的核心程序：参数记进 `log`，装上以后 status 也说装着。
fn fake_core(dir: &Path) -> std::path::PathBuf {
    let program = dir.join("fake-miyu");
    let log = dir.join("calls.log");
    let on = dir.join("on");
    let script = format!(
        "#!/bin/sh\necho \"$*\" >> '{log}'\ncase \"$2\" in\n  enable) touch '{on}' ;;\n  disable) rm -f '{on}' ;;\nesac\nif [ -e '{on}' ]; then i=true; else i=false; fi\necho \"{{\\\"file\\\":\\\"/fake/miyu.service\\\",\\\"installed\\\":$i,\\\"manager\\\":\\\"systemd\\\",\\\"on\\\":$i}}\"\n",
        log = log.display(),
        on = on.display()
    );
    std::fs::write(&program, script).expect("写得了假程序");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("改得了权限");
    }
    program
}

/// 往下挪到光标停在名字是 `name` 的那一行。
fn move_to(tui: &mut Tui, name: &str) {
    for _ in 0..80 {
        if tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains(name))
        {
            return;
        }
        tui.key(b"j");
    }
    panic!("挪不到「{name}」：\n{}", tui.lines().join("\n"));
}

fn until_logged(tui: &mut Tui, log: &Path, want: &str) {
    let end = Instant::now() + support::WAIT;
    while !std::fs::read_to_string(log)
        .unwrap_or_default()
        .contains(want)
    {
        assert!(
            Instant::now() < end,
            "没跑「{want}」：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn the_startup_switch_installs_and_removes_the_service_at_once() {
    let home = Home::new(Script::new([]));
    let dir = home.root().join("fake-bin");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let program = fake_core(&dir);
    let log = dir.join("calls.log");
    let bin = program.to_string_lossy().to_string();
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["config"], &[("MIYU_CORE_BIN", &bin)]);
    tui.wait_for("显示");
    // 开配置页就问一次装没装。
    until_logged(&mut tui, &log, "service status --format json");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    move_to(&mut tui, "开机启动核心");
    assert!(tui.shows("启动"), "{}", tui.lines().join("\n"));
    tui.key(b"\r");
    until_logged(&mut tui, &log, "service enable --format json");
    tui.wait_for("已设为开机启动：/fake/miyu.service");
    tui.key(b"\r");
    until_logged(&mut tui, &log, "service disable --format json");
    tui.wait_for("已取消开机启动");
    assert!(
        !home.settings().contains("tui.service") && !home.settings().contains("service"),
        "不写进设置：{}",
        home.settings()
    );
}

#[test]
fn without_a_known_core_program_there_is_no_switch() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("显示");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    for _ in 0..80 {
        tui.key(b"j");
    }
    tui.pump(Duration::from_millis(300));
    assert!(!tui.shows("开机启动核心"), "{}", tui.lines().join("\n"));
}
