//! 她给人看的文件（核心 D-5 `show`，蓝图 `tui.md`「展示」）：伪终端真 TUI、真核心，打开它的程序换成假的，不开真的浏览器。
#![cfg(all(unix, not(target_os = "macos")))]

mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 只放了假程序的目录：`xdg-mime` 说默认程序是 `mime`（空的是没有），别的把名字和参数记进 `log`。界面的 `PATH` 只给它，
/// 测试的机器上真有浏览器也不会被开。
fn fakes(home: &Home, programs: &[&str], mime: &str) -> (PathBuf, PathBuf) {
    let dir = home.root().join("fake-bin");
    std::fs::create_dir_all(&dir).expect("假程序目录");
    let log = home.root().join("opened.log");
    std::fs::write(&log, "").expect("日志");
    for name in programs {
        let body = if *name == "xdg-mime" {
            format!("#!/bin/sh\nprintf '%s' '{mime}'\n")
        } else {
            format!("#!/bin/sh\necho '{name}' \"$@\" >> '{}'\n", log.display())
        };
        let file = dir.join(name);
        std::fs::write(&file, body).expect("假程序");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("可执行");
    }
    (dir, log)
}

fn show(path: &str) -> Play {
    let args = serde_json::json!({"path": path}).to_string();
    Play::Calls(vec![("show".into(), args)])
}

/// 等日志里有字，最多五秒。
fn logged(log: &Path) -> String {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let text = std::fs::read_to_string(log).unwrap_or_default();
        if !text.is_empty() || Instant::now() > end {
            return text;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 点屏幕上写着 `text` 的那一行。
fn click(tui: &mut Tui, text: &str) {
    let lines = tui.lines();
    let y = lines
        .iter()
        .position(|l| l.contains(text))
        .expect("那一行在屏幕上");
    let x = lines[y].chars().take_while(|c| c.is_whitespace()).count();
    tui.key(format!("\x1b[<0;{};{}M", x + 2, y + 1).as_bytes());
    tui.key(format!("\x1b[<0;{};{}m", x + 2, y + 1).as_bytes());
}

#[test]
fn a_page_opens_at_once_and_its_card_opens_it_again() {
    let home = Home::with_tools(
        Script::new([show("index.html"), Play::Says("页面给你看了。")]),
        "",
    );
    std::fs::write(home.work.join("index.html"), "<h1>你好</h1>\n<p>一段</p>\n").expect("页面");
    let (bin, log) = fakes(&home, &["xdg-mime", "xdg-open"], "firefox.desktop");
    let path = bin.display().to_string();
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("PATH", &path), ("BROWSER", "")]);
    tui.wait_for("工作区");
    tui.say("给我看看页面");
    tui.wait_for("页面给你看了。");
    tui.wait_for("HTML · 30 B · 2 行");
    assert!(tui.shows("点击打开"), "{}", tui.lines().join("\n"));
    // 当场开了一次：交给默认程序的是带文件名的副本（第 6、8 条）。
    let opened = logged(&log);
    assert!(
        opened.starts_with("xdg-open ")
            && opened.trim_end().ends_with("/index.html")
            && opened.contains("miyu-shown"),
        "{opened}"
    );
    let copy = opened
        .trim_end()
        .trim_start_matches("xdg-open ")
        .to_string();
    assert_eq!(
        std::fs::read_to_string(&copy).expect("副本"),
        "<h1>你好</h1>\n<p>一段</p>\n"
    );
    // 点卡片再开一次。
    std::fs::write(&log, "").expect("清日志");
    click(&mut tui, "index.html");
    assert_eq!(logged(&log).trim_end(), format!("xdg-open {copy}"));
}

#[test]
fn markdown_is_a_card_and_with_nothing_to_open_it_the_card_says_where_the_copy_is() {
    let home = Home::with_tools(
        Script::new([show("notes.md"), Play::Says("笔记给你看了。")]),
        "",
    );
    std::fs::write(home.work.join("notes.md"), "# 笔记\n第一行\n").expect("笔记");
    let (bin, log) = fakes(&home, &["xdg-mime"], "");
    let path = bin.display().to_string();
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("PATH", &path), ("BROWSER", "")]);
    tui.wait_for("工作区");
    tui.say("给我看看笔记");
    tui.wait_for("笔记给你看了。");
    tui.wait_for("Markdown · ");
    tui.pump(Duration::from_millis(500));
    assert_eq!(
        std::fs::read_to_string(&log).expect("日志"),
        "",
        "Markdown 不当场开"
    );
    click(&mut tui, "notes.md");
    tui.wait_for("没有能打开它的程序");
    assert!(tui.shows("notes.md"), "{}", tui.lines().join("\n"));
}

#[test]
fn showing_the_same_path_again_folds_the_earlier_card() {
    let home = Home::with_tools(
        Script::new([
            show("notes.md"),
            Play::Says("第一版。"),
            show("notes.md"),
            Play::Says("第二版。"),
        ]),
        "",
    );
    std::fs::write(home.work.join("notes.md"), "一行\n").expect("笔记");
    let (bin, _) = fakes(&home, &["xdg-mime"], "");
    let path = bin.display().to_string();
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("PATH", &path), ("BROWSER", "")]);
    tui.wait_for("工作区");
    tui.say("看看");
    tui.wait_for("第一版。");
    std::fs::write(home.work.join("notes.md"), "一行\n两行\n").expect("改笔记");
    tui.say("再看看");
    tui.wait_for("第二版。");
    tui.wait_for("已更新");
    assert_eq!(
        tui.lines()
            .iter()
            .filter(|l| l.contains("点击打开"))
            .count(),
        1,
        "只有最新的那张是卡片：\n{}",
        tui.lines().join("\n")
    );
}
