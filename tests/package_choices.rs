//! 伪终端里的端到端测试：选项由包在运行时给的文字项接真的核心（蓝图 `tui.md`「配置页」第 43a 条子条目，核心 SG-2 的
//! `choices_from`）：`Enter` 问包的程序要选项、开选择窗，选了写进配置；包答不出选项的退回文字框。

mod support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

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
        "先编测试用的扩展：cargo test 会编，只跑这一个文件时加 --examples（{}）",
        built.display()
    );
    built
}

/// 装包 `say`（开着）：一项文字「音色」，选项问它的程序的 `voices` 方法；程序照 `answer` 这个文件答。
fn install(dir: &Path, answer: &str) {
    let package = support::data_of(dir).packages().join("say");
    std::fs::create_dir_all(package.join("bin")).expect("建得了包的目录");
    let program = package
        .join("bin")
        .join(format!("say-ext{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(extension(), &program).expect("拷得了扩展的程序");
    let choices = dir.join("say-choices.json");
    std::fs::write(&choices, answer).expect("写得进去");
    let manifest = format!(
        "[package]\nprotocol = [1, 1]\nname = {{ en = \"Say\", zh = \"朗读\" }}\n\n[command]\nname = \"say\"\nprogram = \"say-ext\"\nabout = {{ en = \"S\" }}\n\n[process]\nargs = [{:?}]\nstart = \"always\"\n\n[settings.voice]\ntype = \"text\"\nname = {{ en = \"Voice\", zh = \"音色\" }}\nchoices_from = \"voices\"\n",
        format!("choices:{}", choices.display()),
    );
    std::fs::write(package.join("package.toml"), manifest).expect("写得进去");
    let switches = support::data_of(dir).extensions_file();
    std::fs::create_dir_all(switches.parent().expect("有上一层")).expect("建得了");
    std::fs::write(switches, r#"{"version":1,"on":{"say":true},"approved":{}}"#).expect("写得进去");
}

/// 往下挪到选中的那一行（行首 `┃`）是 `to`。
fn reach(tui: &mut Tui, to: &str) {
    for _ in 0..40 {
        if tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains(to))
        {
            return;
        }
        tui.key(b"\x1b[B");
    }
    panic!("挪不到「{to}」：\n{}", tui.lines().join("\n"));
}

/// 等到 `ok` 成立。
fn until(tui: &mut Tui, what: &str, ok: impl Fn(&Tui) -> bool) {
    let end = Instant::now() + support::WAIT;
    while !ok(tui) {
        assert!(
            Instant::now() < end,
            "等不到{what}：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

/// 进「配置 › 软件包 › 已安装 › 朗读」，停在「音色」上。
fn open_voice(tui: &mut Tui) {
    tui.wait_for("运行日志");
    reach(tui, "软件包");
    tui.key(b"\r");
    tui.wait_for("下载软件");
    tui.key(b".");
    tui.wait_for("全部升级");
    reach(tui, "朗读");
    tui.key(b"\r");
    tui.wait_for("音色");
    reach(tui, "音色");
}

#[test]
fn a_text_item_with_choices_from_asks_the_package_and_saves_the_pick() {
    let answer = r#"{"choices": [{"value": "soft", "name": "温柔", "description": "女声"}, {"value": "deep"}]}"#;
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", |dir| {
        install(dir, answer);
    });
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    open_voice(&mut tui);
    // 包的程序起来要一会儿：没起来时退回的是文字框，关掉再按，直到选择窗出来。
    let end = Instant::now() + support::WAIT;
    loop {
        tui.key(b"\r");
        tui.pump(Duration::from_millis(400));
        if tui.shows("温柔") {
            break;
        }
        assert!(
            Instant::now() < end,
            "等不到选择窗：\n{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        tui.pump(Duration::from_millis(200));
    }
    assert!(
        tui.shows("女声") && tui.shows("deep"),
        "说明、没写名字的照值写：\n{}",
        tui.lines().join("\n")
    );
    // 选第二行 `deep`：窗关了，行上写存着的值。
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    until(&mut tui, "行上写 deep", |t| {
        t.lines()
            .iter()
            .any(|l| l.contains("音色") && l.contains("deep"))
            && !t.shows("温柔")
    });
}

#[test]
fn a_package_that_answers_no_choices_gets_the_text_box() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", |dir| {
        install(dir, r#"{"choices": []}"#);
    });
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    open_voice(&mut tui);
    tui.key(b"\r");
    // 编辑窗：打的字进去，回车存，行上写它。
    tui.pump(Duration::from_millis(600));
    tui.key("手写的".as_bytes());
    tui.key(b"\r");
    until(&mut tui, "行上写手写的", |t| {
        t.lines()
            .iter()
            .any(|l| l.contains("音色") && l.contains("手写的"))
    });
}
