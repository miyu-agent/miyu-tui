//! 起来时（蓝图 `tui.md`「连核心」第 1、9 条）：好几处要读的配置合成一条 `config.get`，核心不认其中哪几项（终端包的清单
//! 比终端旧、没重装）整条拒的，碰到那几项的照原来各发时一样当被拒，别的去掉那几项再问一次；上次记下的样子，下次起来不等
//! 核心就照它画。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
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
    // 对照：开着的画着吉祥物（出厂关着，2026-10-26 起）。
    let shown = Home::with_settings(Script::new([]), "[tui]\nmascot_home = true\n");
    let mut tui = shown.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    assert!(tui.shows("@@"), "{}", tui.lines().join("\n"));
}

#[test]
fn a_home_seen_before_is_drawn_before_the_core_answers() {
    // 先画后连（「连核心」第 1 条）：上次连上时记下的样子，下次起来不等核心就照它画。
    let home = Home::with_settings(
        Script::new([Play::Says("好。")]),
        "[tui]\nmascot_home = false\ntheme_mode = \"dark\"\n",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    drop(tui);
    // 核心冻住，什么都不答：照记下的当场画首页（不等 `ui.welcomed` 的 3 秒），深色铺底、不画吉祥物。
    home.freeze_core();
    let started = Instant::now();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "没等核心：{:?}",
        started.elapsed()
    );
    assert_eq!(
        tui.background(0, 0),
        vt100::Color::Rgb(0x1a, 0x1b, 0x26),
        "照记下的深色铺底"
    );
    assert!(
        !tui.shows("@@"),
        "照记下的不画吉祥物：\n{}",
        tui.lines().join("\n")
    );
    // 连上以前就能打字、回车：排着，核心回来连上了再发，不丢。
    tui.say("在吗");
    tui.pump(Duration::from_millis(300));
    home.stop_core();
    home.restart_core();
    tui.wait_for("好。");
}

#[test]
fn what_the_core_says_wins_over_what_was_remembered() {
    // 记下的和核心说的不一样（别处改了配置）：先照记下的画，连上照核心的当场换，不闪空白。
    let home = Home::with_settings(Script::new([]), "[tui]\nmascot_home = false\n");
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(500));
    drop(tui);
    let settings = home.root().join("home/alice/settings.toml");
    let text = std::fs::read_to_string(&settings).expect("有个人设置");
    std::fs::write(
        &settings,
        text.replace("mascot_home = false", "mascot_home = true"),
    )
    .expect("写得了");
    home.stop_core();
    home.restart_core();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.wait_for("@@");
}

#[test]
fn entering_the_recent_session_never_flashes_the_empty_home() {
    // 2026-10-27 项目主人报：机器忙、起来进最近的会话时，先闪一下空会话的首页（输入框在正中）才跳过去。核心冻住就是
    // 「会话迟迟打不开」：这段时间照会话的样子画，输入框在最下面，正文空着。
    let home = Home::with_settings(
        Script::new([Play::Says("第一句的回答。")]),
        "[ui]\nstartup = \"recent\"\n",
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.say("你好");
    tui.wait_for("第一句的回答。");
    tui.pump(Duration::from_millis(500));
    drop(tui);
    // 再起来一趟进了最近的会话：记下「上一回进了已有的会话」。
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("第一句的回答。");
    tui.pump(Duration::from_millis(500));
    drop(tui);
    home.freeze_core();
    let mut tui = home.tui("zh_CN.UTF-8");
    let start = tui.record_from_here();
    tui.pump(Duration::from_millis(1500));
    let frames = support::frames(start, &tui.recorded());
    assert!(!frames.is_empty(), "核心冻住也照记下的先画");
    for frame in &frames {
        let top = frame.iter().position(|l| l.contains('╭'));
        assert!(
            top.is_some_and(|row| row + 5 >= frame.len()),
            "输入框在最下面，不画首页：\n{}",
            frame.join("\n")
        );
    }
    home.stop_core();
    home.restart_core();
    tui.wait_for("第一句的回答。");
}
