//! 伪终端里的端到端测试：人格框、预设框里按 `e` 编辑，改了开着的会话正在用的那一个先问（蓝图 `tui.md`「新会话：人格、
//! 工作区」第 2b 条，2026-10-20 项目主人定）。真界面连一份照剧本回话的核心。

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 出厂预设「全部功能」在你那一层的文件（改出厂的就是建同名覆盖）。
fn file(home: &Home) -> PathBuf {
    home.root().join("home/alice/presets/full.toml")
}

/// 文件里的字；没有的是空的。
fn text(home: &Home) -> String {
    std::fs::read_to_string(file(home)).unwrap_or_default()
}

/// 等 `check` 成立，最多 [`support::WAIT`]；等不到的把屏幕打出来。
fn until(tui: &mut Tui, what: &str, check: impl Fn(&Tui) -> bool) {
    let end = Instant::now() + support::WAIT;
    while !check(tui) {
        assert!(
            Instant::now() < end,
            "等不到{what}，屏幕是：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 开了一个会话（预设照测具默认的全部功能），说过一句。
fn talked(home: &Home) -> Tui {
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui
}

/// 预设的详情窗里切掉第一项功能，等它写进文件，`Esc` 关窗。
fn toggle_and_close(tui: &mut Tui, home: &Home) {
    tui.wait_for("功能");
    let before = text(home);
    tui.key(b"j");
    tui.key(b" ");
    until(tui, "切掉的那一项写进文件", |_| {
        text(home) != before
    });
    tui.key(b"\x1b");
}

#[test]
fn editing_the_preset_in_use_from_the_box_asks_to_compact() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    // 有人格才有人格框（没有的核心不认人格，不问）。
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", None);
    let mut tui = talked(&home);
    // 无人格那一行不能编辑。
    tui.say("/persona");
    tui.wait_for("切换前会先压缩会话");
    // 光标停在正在用的那一行，行首是 `●`（选中了也照旧，第 2a 条）。
    assert!(tui.shows("● 无人格"), "{}", tui.lines().join("\n"));
    tui.key(b"e");
    tui.wait_for("这一项不能编辑");
    tui.key(b"\x1b");
    // 预设框按 `e`：直接开详情窗，提示行写着「e 编辑」。
    tui.say("/preset");
    tui.wait_for("e 编辑");
    assert!(tui.shows("● 全部功能"), "{}", tui.lines().join("\n"));
    tui.key(b"e");
    toggle_and_close(&mut tui, &home);
    // 改的是正在用的：先问；「否」回框，光标还在全部功能上。
    tui.wait_for("当前使用的预设发生变化");
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("切换前会先压缩会话");
    assert!(tui.shows("● 全部功能"), "{}", tui.lines().join("\n"));
    // 再改一次、答「是」：框关上，做一次前台压缩。
    tui.key(b"e");
    toggle_and_close(&mut tui, &home);
    tui.wait_for("当前使用的预设发生变化");
    tui.key(b"\r");
    until(&mut tui, "压缩", |t| {
        t.shows("上下文已压缩") || t.shows("没有可压缩的内容")
    });
    assert!(!tui.shows("切换前会先压缩会话"), "框关了");
}

#[test]
fn closing_the_detail_without_saving_goes_straight_back_to_the_box() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = talked(&home);
    tui.say("/preset");
    tui.wait_for("e 编辑");
    tui.key(b"e");
    tui.wait_for("功能");
    tui.key(b"\x1b");
    tui.wait_for("切换前会先压缩会话");
    assert!(!tui.shows("当前使用的预设发生变化"), "没改不问");
    assert!(!file(&home).exists(), "没改不写");
}

#[test]
fn saving_the_preset_in_use_from_config_asks_before_saving() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = talked(&home);
    tui.say("/config");
    tui.wait_for("决定开关哪些功能");
    tui.open_menu("预设");
    tui.wait_for("全部功能");
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("名称  全部功能");
    tui.key(b"j");
    tui.key(b" ");
    // 先问，不存；「否」不存，回到详情窗。
    tui.wait_for("当前使用的预设发生变化");
    assert!(!file(&home).exists(), "答之前不存");
    tui.key(b"j");
    tui.key(b"\r");
    until(&mut tui, "回到详情窗", |t| {
        !t.shows("当前使用的预设发生变化")
    });
    tui.pump(Duration::from_millis(300));
    assert!(!file(&home).exists(), "答「否」不存");
    // 「是」：存，留在配置页。
    tui.key(b" ");
    tui.wait_for("当前使用的预设发生变化");
    tui.key(b"\r");
    until(&mut tui, "存进文件", |_| file(&home).exists());
    assert!(!tui.shows("上下文已压缩"), "配置页里不压缩");
}
