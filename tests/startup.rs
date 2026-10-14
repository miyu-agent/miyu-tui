//! 起来时读配置（蓝图 `tui.md`「连核心」第 9 条）：好几处要读的合成一条 `config.get`；核心不认其中哪几项（终端包的清单
//! 比终端旧、没重装）整条拒的，碰到那几项的照原来各发时一样当被拒，别的去掉那几项再问一次，照样读得到。

mod support;

use std::time::Duration;

use miyu_session::testkit::Script;
use support::Home;

/// 去掉清单里 `[settings.<name>]` 那一张表（装着旧清单的样子）。
fn without_setting(manifest: &str, name: &str) -> String {
    let head = format!("[settings.{name}]");
    let start = manifest.find(&head).expect("清单里有这一项");
    let end = manifest[start + head.len()..]
        .find("\n[")
        .map_or(manifest.len(), |at| start + head.len() + at + 1);
    format!("{}{}", &manifest[..start], &manifest[end..])
}

#[test]
fn an_older_manifest_still_gives_the_settings_it_has() {
    // 清单里还没有「主色」（10-14 加的）：主题那一处读不到，吉祥物的开关照样读到、照样关。
    let home = Home::with_manifest(
        Script::new([]),
        "[tui]\nmascot_home = false\n",
        |manifest| without_setting(&manifest, "theme_seed"),
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("@@"), "首页的吉祥物照设置关着：\n{screen}");
}

#[test]
fn a_current_manifest_reads_every_setting() {
    let home = Home::with_settings(Script::new([]), "[tui]\nmascot_home = false\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    let screen = tui.lines().join("\n");
    assert!(!screen.contains("@@"), "{screen}");
    // 对照：没关的画着吉祥物。
    let shown = Home::new(Script::new([]));
    let mut tui = shown.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    assert!(tui.shows("@@"), "{}", tui.lines().join("\n"));
}
