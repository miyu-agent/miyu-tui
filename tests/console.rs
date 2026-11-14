//! 没有颜色可靠的地方（蓝图 `tui.md`「主题」第 6 条）：check 13（2026-11-14 主人验收）。
//! ①英文界面里并列的几样不用「、」连（Linux 控制台的字体没有这个字）；②选中的那一行行首写箭头，上下挪看得出来
//! （项目主人：「加一个箭头不就好了吗」）：`NO_COLOR`（默认主题的 `❯`）、Linux 控制台（内置的纯 ASCII，`>`）各走一遍。

mod support;

use miyu_session::testkit::Script;
use support::{Home, Tui};

/// 行首（框线后面）写着箭头 `pointer` 的那一行，箭头后面的字（去掉两头空白）是 `choices` 里的哪一个。
fn picked(tui: &Tui, pointer: char, choices: &[&str]) -> Option<String> {
    tui.lines().iter().find_map(|l| {
        // 框里的行打头是框线（`│`、`|`），箭头在它后面
        let rest = l
            .trim_start()
            .trim_start_matches(['│', '|', ' '])
            .strip_prefix(pointer)?
            .trim();
        choices
            .iter()
            .find(|c| rest.starts_with(**c))
            .map(|c| (*c).to_string())
    })
}

/// 配置页：主菜单、一页里的行、选值的选择窗，选中的都有箭头，挪一下箭头跟着走。
fn walk(env: &[(&str, &str)], pointer: char) {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui_with("en_US.UTF-8", env);
    tui.wait_for("Full access");
    tui.say("/config");
    tui.wait_for("Display, New sessions");
    assert!(
        !tui.lines().iter().any(|l| l.contains('、')),
        "英文界面不用「、」连：\n{}",
        tui.lines().join("\n")
    );
    let menu = ["General", "Appearance"];
    assert_eq!(
        picked(&tui, pointer, &menu).as_deref(),
        Some("General"),
        "主菜单：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"j");
    assert_eq!(
        picked(&tui, pointer, &menu).as_deref(),
        Some("Appearance"),
        "主菜单挪了一项：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"k");
    tui.key(b"\r");
    tui.wait_for("Interface language");
    let rows = ["Interface language", "On start, open"];
    assert_eq!(
        picked(&tui, pointer, &rows).as_deref(),
        Some("Interface language"),
        "页里的行：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\r");
    tui.wait_for("Follow the system");
    let choices = ["Follow the system", "中文", "English", "日本語"];
    assert_eq!(
        picked(&tui, pointer, &choices).as_deref(),
        Some("Follow the system"),
        "选择窗选中的那一行行首有箭头：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"j");
    assert_eq!(
        picked(&tui, pointer, &choices).as_deref(),
        Some("中文"),
        "挪了一行，箭头跟着走：\n{}",
        tui.lines().join("\n")
    );
}

#[test]
fn without_colour_the_pick_has_an_arrow() {
    walk(&[("NO_COLOR", "1")], '❯');
}

#[test]
fn on_the_linux_console_the_pick_has_an_arrow_and_commas_are_plain() {
    walk(&[("TERM", "linux"), ("COLORTERM", "")], '>');
}

/// `/effort` 这种行首写「现在用的」记号的列表：没有颜色时光标停在现在用的那一项上也写箭头（原来只写记号，看不出光标在哪）。
#[test]
fn without_colour_the_effort_list_shows_the_arrow_on_the_current_level_too() {
    use miyu_session::testkit::Play;
    let settings = "[models]\nchat = \"deepseek/deepseek-v4\"\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nwindow = 128000\nreasoning = [\"off\", \"low\", \"high\"]\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui_with("zh_CN.UTF-8", &[("NO_COLOR", "1")]);
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for_done();
    tui.say("/effort");
    tui.wait_for("跟随设置");
    let levels = ["跟随设置", "off", "low", "high"];
    assert_eq!(
        picked(&tui, '❯', &levels).as_deref(),
        Some("跟随设置"),
        "光标在现在用的那一项上：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b[B");
    assert_eq!(
        picked(&tui, '❯', &levels).as_deref(),
        Some("off"),
        "挪了一行：\n{}",
        tui.lines().join("\n")
    );
}
