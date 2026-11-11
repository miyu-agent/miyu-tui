//! 伪终端里的端到端测试：「受保护的命令」那一组（核心 HZ-1，蓝图 `tui.md`「配置页」）。真界面连一份真核心：这一组在
//! 「隐私与安全」页、排在私密目录前面，组名下面写着核心给的说明，五个开关照清单画、拨了当场存进系统配置。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn the_protected_commands_group_comes_before_private_directories_and_its_switches_save() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("显示");
    tui.open_menu("隐私与安全");
    tui.wait_for("受保护的命令");
    let lines = tui.lines();
    let at = |word: &str| {
        lines
            .iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("页里没有「{word}」：\n{}", lines.join("\n")))
    };
    assert!(
        at("受保护的命令") < at("私密目录"),
        "排在私密目录前面：\n{}",
        lines.join("\n")
    );
    // 组名下面是核心给的说明（暗色，折行写全），再是五个开关。
    assert!(
        at("受保护的命令") < at("完全放开时也会先问你")
            && at("完全放开时也会先问你") < at("破坏性 git 操作"),
        "{}",
        lines.join("\n")
    );
    for name in [
        "破坏性 git 操作",
        "递归删除",
        "关机和重启",
        "下载后直接执行",
        "容器使用宿主设备",
    ] {
        at(name);
    }
    // 光标挪到「关机和重启」上，回车拨掉：当场存进系统配置。
    for _ in 0..40 {
        let on_it = tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains("关机和重启"));
        if on_it {
            break;
        }
        tui.key(b"j");
    }
    tui.key(b"\r");
    let end = Instant::now() + support::WAIT;
    while !home.settings().contains("power = false") {
        assert!(
            Instant::now() < end,
            "设置里没有 power = false：\n{}\n屏幕：\n{}",
            home.settings(),
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}
