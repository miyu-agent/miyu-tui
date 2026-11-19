//! 伪终端里的端到端测试：「受保护的命令」那一组（核心 HZ-1，蓝图 `tui.md`「配置页」）。真界面连一份真核心：这一组在
//! 「隐私与安全」页，组名下面写着核心给的说明，五个开关照清单画、拨了当场存进系统配置；私密目录那一组随核心 PV-RM 去掉了。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::Home;

#[test]
fn the_protected_commands_group_shows_its_note_and_its_switches_save() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("显示");
    tui.open_menu("隐私与安全");
    // 说明和五个开关在组名下面：等到最后一个开关也画出来再读。
    let lines = tui.wait_for_all(&["受保护的命令", "完全放开时也会先问你", "容器使用宿主设备"]);
    let at = |word: &str| {
        lines
            .iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("页里没有「{word}」：\n{}", lines.join("\n")))
    };
    assert!(
        !lines.iter().any(|l| l.contains("私密目录")),
        "私密目录那一组没了（核心 PV-RM）：\n{}",
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
