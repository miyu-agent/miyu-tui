//! 伪终端里的端到端测试：软件包那一页分组接真的核心（蓝图 `tui.md`「配置页」第 43a 条子条目，核心 SG-1）：一组一块、组名后面写
//! 「默认」「已改」；引擎选哪家，只画哪家那一组。

mod support;

use std::path::Path;
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 在数据根里放一个带分组设置的包 `speak`：引擎选 mimo 画 MiMo 那一组，选 minimax 画 MiniMax 那一组。
fn speak(root: &Path) {
    let dir = support::data_of(root).packages().join("speak");
    let skill = dir.join("skills/notes");
    std::fs::create_dir_all(&skill).expect("建得了目录");
    std::fs::write(
        dir.join("package.toml"),
        r#"[package]
version = "1.0.0"
name = { en = "Speak", zh = "播报" }

[settings.engine]
type = "option"
choices = ["mimo", "minimax"]
default = "mimo"
name = { en = "Engine", zh = "引擎" }

[settings.mimo]
name = { en = "MiMo", zh = "MiMo" }
shown_when = { engine = "mimo" }

[settings.mimo.voice]
type = "text"
name = { en = "MiMo voice", zh = "MiMo 音色" }

[settings.minimax]
name = { en = "MiniMax", zh = "MiniMax" }
shown_when = { engine = "minimax" }

[settings.minimax.voice]
type = "text"
name = { en = "MiniMax voice", zh = "MiniMax 音色" }
"#,
    )
    .expect("写得进去");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: notes\ndescription: Take notes in a fixed format.\n---\nWrite notes.\n",
    )
    .expect("写得进去");
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

#[test]
fn a_package_page_draws_one_block_per_section_and_only_the_chosen_engines() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", speak);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    tui.wait_for("运行日志");
    reach(&mut tui, "软件包");
    tui.key(b"\r");
    tui.wait_for("下载软件");
    tui.key(b".");
    tui.wait_for("全部升级");
    reach(&mut tui, "播报");
    tui.key(b"\r");
    tui.wait_for("引擎");
    until(&mut tui, "MiMo 那一组写着默认", |t| {
        t.lines().iter().any(|l| l.trim() == "MiMo  默认")
    });
    assert!(
        !tui.lines().iter().any(|l| l.contains("MiniMax")),
        "引擎是 mimo，MiniMax 那一组不画：\n{}",
        tui.lines().join("\n")
    );
    // 引擎换成 minimax：MiniMax 那一组出来，MiMo 那一组藏起来。
    reach(&mut tui, "引擎");
    tui.key(b"\r");
    tui.wait_for("minimax");
    // 选择窗停在正在用的 mimo 上，往下一个就是 minimax。
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    until(&mut tui, "MiniMax 那一组出来", |t| {
        t.lines().iter().any(|l| l.trim() == "MiniMax  默认")
    });
    assert!(
        !tui.lines().iter().any(|l| l.contains("MiMo 音色")),
        "MiMo 那一组藏起来：\n{}",
        tui.lines().join("\n")
    );
}
