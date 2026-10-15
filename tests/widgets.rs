//! 界面块（蓝图 `tui.md`「界面块」，核心 X-15）：真核心拉起一个测试用的扩展（`examples/widget_extension.rs`），它照核心说的
//! 有人在看推内容。首页、底栏、侧边栏照屏上有的位置画出来；点列表里的一条经 `package.call` 调到它的方法、它换了内容跟着换；
//! 悬停底栏那一格说是哪个软件放的。

mod support;

use std::path::{Path, PathBuf};
use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};
use unicode_width::UnicodeWidthStr;

/// 测试用的扩展编在哪：`cargo test` 编 examples，放在测试程序上两级的 `examples/` 里。
fn extension() -> PathBuf {
    let exe = std::env::current_exe().expect("找得到测试程序");
    let dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("测试程序在 target/<档>/deps 里");
    let built = dir
        .join("examples")
        .join(format!("widget_extension{}", std::env::consts::EXE_SUFFIX));
    assert!(
        built.exists(),
        "先编测试用的扩展：cargo test 会编，只跑这一个文件时加 --examples（{}）",
        built.display()
    );
    built
}

/// 每一块推什么。
const CONTENT: &str = r#"{
  "intro": {"content": [
    {"type": "text", "text": "早上好，今天晴"},
    {"type": "buttons", "items": [{"label": "看本周", "action": {"open": "week"}}]}
  ]},
  "temp": {"content": [{"type": "text", "text": "晴 23°C"}]},
  "today": {"content": [
    {"type": "list", "items": [{"title": "刷新一下", "state": "todo", "action": {"call": "refresh", "params": {"widget": "today"}}}]},
    {"type": "text", "text": "湿度 40%"}
  ]},
  "week": {"content": [{"type": "heading", "text": "这一周"}, {"type": "list", "items": DAYS}]}
}"#;

/// 装扩展 `wx`（批了 `widgets`、开着）：首页一块、底栏一块、侧边栏一块（跟着会话）。程序记下收到的 `widget.watch`、
/// `method.call`。
fn install(dir: &Path) {
    let package = dir.join("home/alice/packages/wx");
    std::fs::create_dir_all(package.join("bin")).expect("建得了包的目录");
    let program = package
        .join("bin")
        .join(format!("wx-ext{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(extension(), &program).expect("拷得了扩展的程序");
    let content = dir.join("wx-content.json");
    // 整页里三十天，一天一条，点了调 `refresh`：一屏放不下，要滚。
    let days: Vec<String> = (1..=30)
        .map(|n| format!(r#"{{"title": "第 {n} 天", "action": {{"call": "refresh", "params": {{"widget": "week"}}}}}}"#))
        .collect();
    let text = CONTENT.replace("DAYS", &format!("[{}]", days.join(", ")));
    std::fs::write(&content, text).expect("写得进去");
    let record = dir.join("wx-calls.log");
    let manifest = format!(
        "[package]\nprotocol = [1, 1]\nname = {{ en = \"Weather\" }}\n\n[command]\nname = \"wx\"\nprogram = \"wx-ext\"\nabout = {{ en = \"W\" }}\n\n[process]\nargs = [{:?}, {:?}]\nstart = \"always\"\ncapabilities = [\"widgets\"]\n\n[[widget]]\nid = \"intro\"\nplace = \"home\"\n\n[[widget]]\nid = \"temp\"\nplace = \"status\"\n\n[[widget]]\nid = \"today\"\nplace = \"sidebar\"\ntitle = {{ en = \"Today\", zh = \"天气\" }}\nscope = \"session\"\n\n[[widget]]\nid = \"week\"\nplace = \"page\"\ntitle = {{ en = \"This week\", zh = \"本周\" }}\ncommand = \"week\"\n",
        format!("content:{}", content.display()),
        format!("record:{}", record.display()),
    );
    std::fs::write(package.join("package.toml"), manifest).expect("写得进去");
    std::fs::write(
        dir.join("system/extensions.json"),
        r#"{"version":1,"on":{"wx":true},"approved":{"wx":["widgets"]}}"#,
    )
    .expect("写得进去");
}

/// `text` 在屏上第几行、从第几列起（宽字算两列）。
fn locate(tui: &Tui, text: &str) -> (usize, usize) {
    tui.lines()
        .iter()
        .enumerate()
        .find_map(|(y, line)| line.find(text).map(|at| (line[..at].width(), y)))
        .unwrap_or_else(|| panic!("屏上没有「{text}」：\n{}", tui.lines().join("\n")))
}

#[test]
fn an_extension_puts_blocks_on_the_home_page_the_status_line_and_the_sidebar() {
    let home = Home::prepared(Script::new([Play::Says("你好呀。")]), "", install);
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    // 首页：工作目录下面，标题是软件名（没写标题）。
    tui.wait_for("早上好，今天晴");
    let (_, intro) = locate(&tui, "早上好，今天晴");
    let (_, title) = locate(&tui, "Weather");
    assert_eq!(
        title + 1,
        intro,
        "标题在内容上面一行：\n{}",
        tui.lines().join("\n")
    );
    // 底栏：一格字。
    tui.wait_for("晴 23°C");
    // 悬停那一格：提示是哪个软件放的。
    let before = tui.lines().join("\n").matches("Weather").count();
    let (x, y) = locate(&tui, "晴 23°C");
    tui.key(format!("\x1b[<35;{};{}M", x + 2, y + 1).as_bytes());
    tui.pump(Duration::from_millis(400));
    let after = tui.lines().join("\n").matches("Weather").count();
    assert_eq!(
        after,
        before + 1,
        "提示软件名：\n{}",
        tui.lines().join("\n")
    );
    // 进了对话：侧边栏接着画跟着会话的那一块，标题后面接软件名。
    tui.say("你好");
    tui.wait_for("你好呀。");
    tui.wait_for("天气 · Weather");
    tui.wait_for("湿度 40%");
    assert!(!tui.shows("早上好，今天晴"), "首页的块不在对话里画");
    // 点列表里那一条：调到扩展的方法，它换了内容，跟着换。
    let (x, y) = locate(&tui, "刷新一下");
    tui.key(format!("\x1b[<0;{};{}M", x + 1, y + 1).as_bytes());
    tui.key(format!("\x1b[<0;{};{}m", x + 1, y + 1).as_bytes());
    tui.wait_for("refreshed");
    let calls = std::fs::read_to_string(home.root().join("wx-calls.log")).unwrap_or_default();
    assert!(calls.contains(r#""method":"refresh""#), "{calls}");
}

#[test]
fn a_narrow_window_has_no_sidebar_blocks_but_keeps_the_status_cell() {
    let home = Home::prepared(Script::new([Play::Says("你好呀。")]), "", install);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("晴 23°C");
    tui.say("你好");
    tui.wait_for("你好呀。");
    tui.pump(Duration::from_millis(600));
    assert!(tui.shows("晴 23°C"), "{}", tui.lines().join("\n"));
    assert!(
        !tui.shows("湿度 40%"),
        "窄屏没有侧边栏：\n{}",
        tui.lines().join("\n")
    );
    // 屏上没有侧边栏，核心不叫只放侧边栏的那一块干活（「界面块」第 8 条）。
    let seen = std::fs::read_to_string(home.root().join("wx-calls.log")).unwrap_or_default();
    assert!(
        seen.contains(r#""widget":"temp""#),
        "底栏那一块有人看：{seen}"
    );
    assert!(
        !seen.contains(r#""widget":"today""#),
        "侧边栏那一块没人看：{seen}"
    );
}

#[test]
fn blocks_come_back_after_the_core_restarts() {
    let home = Home::prepared(Script::new([]), "", install);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("晴 23°C");
    // 核心停了：扩展跟着停，块清掉。
    home.stop_core();
    let end = std::time::Instant::now() + Duration::from_secs(10);
    while tui.shows("晴 23°C") {
        assert!(
            std::time::Instant::now() < end,
            "断了线块还在：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
    // 再起来：连上以后照原来的说法重新订阅，扩展又推过来。
    home.restart_core();
    tui.wait_for("晴 23°C");
}

#[test]
fn a_page_opens_from_a_button_or_its_command_scrolls_and_esc_goes_back() {
    let home = Home::prepared(Script::new([]), "", install);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("早上好，今天晴");
    tui.type_text("草稿");
    // 首页那一块里的按钮：开同一个包的整页，面包屑写软件名和标题。
    let (x, y) = locate(&tui, "看本周");
    tui.key(format!("\x1b[<0;{};{}M", x + 1, y + 1).as_bytes());
    tui.key(format!("\x1b[<0;{};{}m", x + 1, y + 1).as_bytes());
    tui.wait_for("Weather — 本周");
    tui.wait_for("第 1 天");
    assert!(
        !tui.shows("第 30 天"),
        "一屏放不下：\n{}",
        tui.lines().join("\n")
    );
    // 翻页看到后面的；Enter 做选着的那一条（调到扩展的方法，它换了内容）。
    tui.key(b"\x1b[6~");
    tui.wait_for("第 30 天");
    tui.key(b"\r");
    tui.wait_for("refreshed");
    // Esc 回到首页，打了一半的字还在。
    tui.key(b"\x1b");
    tui.wait_for("早上好，今天晴");
    assert_eq!(tui.input(), "草稿", "{}", tui.lines().join("\n"));
    // 打整页带的命令：当场开那一页。
    tui.key(b"\x7f\x7f");
    tui.say("/week");
    tui.wait_for("Weather — 本周");
    // 只是开一页：不为它开会话（不交给 `command.run`，「界面块」第 10 条）。
    tui.pump(Duration::from_millis(300));
    let sessions = std::fs::read_dir(home.root().join("home/alice/sessions"))
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(sessions, 0, "开整页不开会话");
    let seen = std::fs::read_to_string(home.root().join("wx-calls.log")).unwrap_or_default();
    assert!(
        seen.contains(r#""widget":"week""#),
        "整页开着有人看：{seen}"
    );
}
