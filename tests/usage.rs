//! 伪终端里的端到端测试：花了多少钱（蓝图 `tui.md`「配置与模型」第 5 条，核心 8-15）。剧本照价格算好金额写进
//! `model.called` 的 `cost`，侧边栏、框下面那一行照它写；`/usage` 向核心要 `usage.query`。

mod support;

use std::time::Duration;

use miyu_models::catalog::{Price, Rates};
use miyu_models::price::Tariff;
use miyu_session::testkit::{Play, Script};
use support::Home;

/// 输入 1、输出 2、读缓存 0.5 美元每百万：剧本一次报 60 没命中、40 命中、10 输出，花 0.0001。
fn usd() -> Tariff {
    Tariff {
        price: Price::of(
            Rates {
                input: Some(1.0),
                output: Some(2.0),
                cache_read: Some(0.5),
                cache_write: None,
            },
            "USD",
        ),
        multiplier: 1.0,
        source: "config:config.toml:7".to_string(),
    }
}

#[test]
fn the_sidebar_and_the_footer_show_what_was_spent_and_slash_usage_lists_it() {
    let home = Home::new(Script::new([Play::Says("好。")]).priced(usd()));
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.wait_for("花费 $0.0001");
    tui.wait_for("· $0.0001");
    tui.say("/usage");
    // 总览：四个数、热度图（2026-10-07 项目主人：照 Codex 做细）。
    tui.wait_for("单日最高");
    tui.wait_for("连续使用");
    tui.wait_for("1 天");
    tui.wait_for("最长的会话");
    tui.wait_for("共 $0.0001");
    // 热度图的图例、格子：照等的写，不当场看（2026-10-12 macOS 的 CI 上框刚打开那一帧只画了上半截，当场看偶发红）。
    tui.wait_for("少");
    tui.wait_for("多");
    // 换页不改框的高度（2026-10-07 项目主人）：下边框在格子下面，等到它也画出来再量。
    tui.wait_for_all(&["■", "╰─ Tab"]);
    let height = |tui: &support::Tui| {
        let lines = tui.lines();
        let top = lines.iter().position(|l| l.contains("╭─ 用量"));
        let bottom = lines.iter().position(|l| l.contains("╰─ Tab"));
        top.zip(bottom).map(|(t, b)| b - t)
    };
    let first = height(&tui);
    // 最近 30 天：柱状图加按天的表。
    tui.key(b"\t");
    tui.wait_for("今天");
    tui.wait_for("1 次");
    tui.wait_for("█");
    // 按模型：占比条、用途。
    tui.key(b"\t");
    tui.wait_for("100%");
    // 那个模型一行的数在占比右边：等到它也画出来再看。
    tui.wait_for_all(&["主对话 100%", "1 次   110   $0.0001"]);
    let row = tui
        .lines()
        .into_iter()
        .find(|l| l.contains("100%"))
        .unwrap_or_default();
    assert!(row.contains("1 次   110   $0.0001"), "{row}");
    assert_eq!(height(&tui), first, "按模型那一页矮，框不变矮");
    // 按会话：没标题的照预览（第一句话）。
    tui.key(b"\t");
    tui.wait_for("在吗");
    assert_eq!(height(&tui), first);
    // 换回总览：`Shift+Tab` 三下。
    for _ in 0..3 {
        tui.key(b"\x1b[Z");
    }
    tui.wait_for("单日最高");
    tui.key(b"\x1b");
}

#[test]
fn requests_without_a_price_are_counted_not_guessed() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.wait_for("缓存命中");
    tui.pump(Duration::from_millis(300));
    let screen = tui.lines().join("\n");
    assert!(
        !screen.contains("花费"),
        "一次都算不出的不写花费：\n{screen}"
    );
    // 2026-10-10 项目主人：「没有费用记录的请求不需要出现在侧边栏中」。
    assert!(!screen.contains("没有价格"), "{screen}");
    // 输入框背景里的提示可能写着「$ 点名技能」（随机挑的），不算。
    let outside_input = tui
        .lines()
        .into_iter()
        .filter(|l| !l.contains('❯'))
        .collect::<Vec<_>>();
    assert!(
        !outside_input.iter().any(|l| l.contains('$')),
        "框下面那一行也不写：\n{screen}"
    );
}

/// `/usage` 里 `←` `→`、`h` `l` 换页（2026-11-17 验收第 7 条：只有 `Tab` 换得了）；按键模式是 Vim 的也一样。
#[test]
fn arrows_and_h_l_turn_the_usage_pages() {
    for (settings, hint) in [
        ("", "Tab/←/→ 换页"),
        ("[tui]\nkeys = \"vim\"\n", "Tab/h/l 换页"),
    ] {
        let home = Home::with_settings(Script::new([Play::Says("好。")]).priced(usd()), settings);
        let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
        tui.wait_for("开放权限");
        tui.say("在吗");
        tui.wait_for("好。");
        tui.say("/usage");
        tui.wait_for("单日最高");
        tui.wait_for(hint);
        tui.key(b"\x1b[C");
        tui.wait_for("今天");
        tui.key(b"\x1b[C");
        tui.wait_for("主对话 100%");
        tui.key(b"\x1b[D");
        tui.wait_for("今天");
        tui.key(b"l");
        tui.wait_for("主对话 100%");
        tui.key(b"h");
        tui.wait_for("今天");
        tui.key(b"\x1b");
    }
}
