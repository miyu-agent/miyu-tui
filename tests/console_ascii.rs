//! Linux 控制台里只画 ASCII（蓝图 `tui.md`「主题」第 6 条，check 13 ①，2026-11-14 主人验收：「我切换语言为英文，也会有
//! 很多无法显示的字符」）：`TERM=linux` 的英文界面，写了 `nerd` 图标的也算，走一遍首页、命令列表、对话（工具、思考、
//! Markdown）、`/model`、`/effort`、`/sessions`、`/context`、配置页的主菜单、一页、选择窗，每一格都是 ASCII。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 写了 `nerd` 图标（主人的验收目录就是这样写的）、一个带思考档位的模型。
const SETTINGS: &str = "[tui]\nicons = \"nerd\"\n\n[models]\nchat = \"deepseek/deepseek-v4\"\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nwindow = 128000\nreasoning = [\"off\", \"low\", \"high\"]\n";

/// 她先跑一条命令，再想一下、答一段带列表、引用、表格、代码的 Markdown。
fn script() -> Script {
    Script::new([
        Play::Calls(vec![(
            "shell".into(),
            r#"{"command": "echo hi", "title": "Say hi"}"#.into(),
        )]),
        Play::Thinks {
            thinking: "Let me check.",
            text: "Done **here**.\n\n- one\n- two\n\n> quoted\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```",
        },
    ])
}

/// 屏幕上每一格都是 ASCII；不是的写出是哪个字、在哪一行。
fn all_ascii(tui: &Tui, what: &str) {
    let odd: Vec<String> = tui
        .lines()
        .iter()
        .filter(|l| !l.is_ascii())
        .map(|l| {
            let chars: String = l.chars().filter(|c| !c.is_ascii()).collect();
            format!("{chars:?} in {:?}", l.trim())
        })
        .collect();
    assert!(
        odd.is_empty(),
        "{what} 有不是 ASCII 的字：\n{}",
        odd.join("\n")
    );
}

/// 开一个框、等它画出 `shows`、查一遍、`Esc` 关上。
fn boxed(tui: &mut Tui, command: &str, shows: &str) {
    tui.say(command);
    tui.wait_for(shows);
    tui.pump(Duration::from_millis(300));
    all_ascii(tui, command);
    tui.key(b"j");
    all_ascii(tui, command);
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(300));
}

#[test]
fn the_linux_console_draws_only_ascii_even_with_nerd_icons_written() {
    let home = Home::with_tools(script(), SETTINGS);
    let mut tui = home.tui_wide_with("en_US.UTF-8", 160, &[("TERM", "linux"), ("COLORTERM", "")]);
    tui.wait_for("Full access");
    tui.pump(Duration::from_millis(500));
    all_ascii(&tui, "首页");
    tui.say("hi");
    tui.wait_for("Done here");
    tui.pump(Duration::from_millis(800));
    all_ascii(&tui, "对话");
    tui.type_text("/");
    tui.wait_for("Commands");
    all_ascii(&tui, "命令列表");
    tui.key(b"\x1b");
    tui.key(b"\x7f");
    boxed(&mut tui, "/model", "Models");
    boxed(&mut tui, "/effort", "low");
    boxed(&mut tui, "/sessions", "Sessions");
    boxed(&mut tui, "/context", "Context");
    tui.say("/config");
    tui.wait_for("Display, New sessions");
    all_ascii(&tui, "配置页主菜单");
    tui.open_menu("General");
    tui.wait_for("Interface language");
    all_ascii(&tui, "配置页一页");
    // 选「起来进哪个」那一行的选择窗（界面语言的选项是各语言自己的写法，中文、日文本来就是那两种字）
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("The most recent session");
    all_ascii(&tui, "选择窗");
    tui.key(b"\x1b");
    tui.key(b"\x1b");
    tui.open_menu("Providers");
    tui.pump(Duration::from_millis(800));
    all_ascii(&tui, "供应商和模型");
}
