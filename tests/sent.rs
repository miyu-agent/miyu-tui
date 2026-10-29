//! 她发出去的图（蓝图 `tui.md`「她发的图」，核心 X-19）：真核心拉起测试用的扩展（`examples/widget_extension.rs`），她调它的
//! `sticker`，扩展答的 `sent` 记进 `tool.result`；伪终端画不了图，正文里那一步后面是「[表情包] 一句描述」，时间线照常收起，
//! 收起那一行写那件工具的英文显示名（核心 X-24，界面语言是自动）。

mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 测试用的扩展编在哪（同 `tests/widgets.rs`）。
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
        "先编测试用的扩展：加 --examples（{}）",
        built.display()
    );
    built
}

/// 装扩展 `memes`（开着，清单写了图标 `face-slightly-smiling`）：登记工具 `sticker`，发的图是先放进 alice 名下的一个 blob。
fn install(dir: &Path) {
    let blob = miyu_store::blob::Blobs::new(dir.join("blobs"))
        .put(b"\x89PNG not really a picture")
        .expect("存得进");
    let package = dir.join("packages/memes");
    std::fs::create_dir_all(package.join("bin")).expect("建得了包的目录");
    let program = package
        .join("bin")
        .join(format!("memes-ext{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(extension(), &program).expect("拷得了扩展的程序");
    let manifest = format!(
        "[package]\nprotocol = [1, 1]\nname = {{ en = \"Memes\" }}\nicon = \"face-slightly-smiling\"\n\n[command]\nname = \"memes\"\nprogram = \"memes-ext\"\nabout = {{ en = \"M\" }}\n\n[process]\nargs = [{:?}]\nstart = \"always\"\ncapabilities = [\"tools\"]\n",
        format!("sticker:{}", blob.as_str()),
    );
    std::fs::write(package.join("package.toml"), manifest).expect("写得进去");
    // 登记工具的扩展照实声明 `tools`、批过才拉起（核心 X-30）；批过的能力和开关记在一起（`approved`，核心 SU-2 中起；
    // 原来在组表 `system/groups.json`）。
    std::fs::write(
        dir.join("extensions.json"),
        r#"{"version":1,"on":{"memes":true},"approved":{"memes":["tools"]}}"#,
    )
    .expect("写得进去");
}

/// 起终端，等扩展登记成了（核心写下登记缓存）才交回：不然她调到的时候还没有这件工具。
fn started(home: &Home) -> support::Tui {
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    let cache = home.root().join("state/providers/memes.json");
    let end = Instant::now() + support::WAIT;
    while !cache.exists() {
        assert!(Instant::now() < end, "扩展一直没登记工具");
        tui.pump(Duration::from_millis(50));
    }
    tui
}

#[test]
fn a_sticker_she_sends_is_written_after_its_step_when_pictures_cannot_be_shown() {
    let script = Script::new([
        Play::Calls(vec![("sticker".into(), "{}".into())]),
        Play::Says("好。"),
    ]);
    let home = Home::prepared(script, "", install);
    let mut tui = started(&home);
    tui.say("发个表情");
    tui.wait_for("好。");
    tui.wait_for("[表情包] 招手的猫");
    let lines = tui.lines();
    let at = |text: &str| {
        lines
            .iter()
            .position(|l| l.contains(text))
            .unwrap_or_else(|| panic!("屏上没有「{text}」：\n{}", lines.join("\n")))
    };
    // 只为发这一张的那一段，收起那一行写那件工具的显示名（核心 X-24）；界面语言是自动，收起那一行整行英文，显示名取
    // 调用那一刻记下的英文那一份（2026-10-16 项目主人验 T-260）。
    assert!(
        at("Send sticker · ") < at("[表情包] 招手的猫") && at("[表情包] 招手的猫") < at("好。"),
        "那一步收进时间线，图排在它后面、回答前面：\n{}",
        lines.join("\n")
    );
    assert!(
        !lines.iter().any(|l| l.contains("Used 1 tool")),
        "不再数用了几件工具"
    );
}

#[test]
fn the_step_of_a_package_tool_carries_its_package_icon() {
    // 2026-10-16 项目主人验 T-260：表情包那一步还是扳手。包清单写了 Lucide 名的，那一步画对照表里的字形（笑脸）。
    let script = Script::new([
        Play::Calls(vec![("sticker".into(), "{}".into())]),
        Play::Says("好。"),
    ]);
    let home = Home::prepared(script, "[tui]\ntimeline_fold = false\n", install);
    let mut tui = started(&home);
    tui.say("发个表情");
    tui.wait_for("好。");
    tui.wait_for("\u{f118} 发送表情包");
}
