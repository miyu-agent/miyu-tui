//! 伪终端里的端到端测试：`/jump` 列出这个会话里说过的话，选一句跳过去，那一句顶到视口最上面（蓝图 `tui.md`「跳到说过的话」，
//! 2026-10-16 项目主人要的）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 一段回答：每段一行，`tag` 打头好认。比一屏短：长过视口的回答会停在开头（「正文」第 1 条），最后一段露不出来，流得慢的
/// 机器上等不到它（2026-10-16 macOS CI）；三轮加起来照样把第二问挤出视口。
fn long(tag: &str) -> &'static str {
    let text: Vec<String> = (1..=10).map(|i| format!("{tag} 第 {i} 段。")).collect();
    Box::leak(text.join("\n\n").into_boxed_str())
}

#[test]
fn jump_lists_what_was_said_and_puts_the_picked_one_at_the_top() {
    let home = Home::new(Script::new([
        Play::Says(long("甲")),
        Play::Says(long("乙")),
        Play::Says(long("丙")),
    ]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    for (said, last) in [
        ("第一问 alpha", "甲 第 10 段。"),
        ("第二问 beta", "乙 第 10 段。"),
        ("第三问 gamma", "丙 第 10 段。"),
    ] {
        tui.say(said);
        tui.wait_for(last);
    }
    tui.pump(Duration::from_millis(500));
    assert!(!tui.shows("第二问 beta"), "第二问早滚出视口了");
    tui.say("/jump");
    tui.wait_for("跳到  3 条");
    for said in ["第一问 alpha", "第二问 beta", "第三问 gamma"] {
        assert!(
            tui.shows(said),
            "列表里有「{said}」：\n{}",
            tui.lines().join("\n")
        );
    }
    tui.type_text("beta");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    // 一画出来就取颜色（竖线只亮 1 秒：原来固定等 0.5 秒再取，macOS 的 CI 慢时取到的已经是亮过以后的颜色）。
    let start = std::time::Instant::now();
    let at = loop {
        tui.pump(Duration::from_millis(20));
        let lines = tui.lines();
        let found = lines.iter().position(|l| l.contains("第二问 beta"));
        if let Some(at) = found.filter(|&at| at <= 2 && !tui.shows("跳到")) {
            break at;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "没跳过去、没顶到视口最上面（竖线那一行之下）、列表没关：\n{}",
            lines.join("\n")
        );
    };
    let lines = tui.lines();
    // 竖线亮一下（第 3 条）：刚跳过去和过了一秒多颜色不一样。
    let col = lines[at].chars().take_while(|&c| c != '┃').count();
    let (row, col) = (u16::try_from(at).unwrap(), u16::try_from(col).unwrap());
    assert_eq!(tui.cell(row, col), "┃");
    let lit = tui.foreground(row, col);
    let start = std::time::Instant::now();
    while tui.foreground(row, col) == lit && start.elapsed() < Duration::from_secs(5) {
        tui.pump(Duration::from_millis(100));
    }
    assert_ne!(tui.foreground(row, col), lit, "亮过以后画回原来的颜色");
}

#[test]
fn jump_reads_the_earlier_pages_of_an_old_session_without_scrolling_up() {
    // 比一页（核心默认 20 轮）多两轮；重起以后进最近的那个会话，只读了最新一页（`tests/pages.rs`）。
    const TURNS: usize = 22;
    let plays: Vec<Play> = (1..=TURNS)
        .map(|i| Play::Says(Box::leak(format!("第 {i} 号的回答。").into_boxed_str())))
        .collect();
    let home = Home::with_settings(Script::new(plays), "[ui]\nstartup = \"recent\"\n");
    let mut first = home.tui("zh_CN.UTF-8");
    first.wait_for("开放权限");
    for i in 1..=TURNS {
        first.say(&format!("第 {i} 句"));
        first.wait_for(&format!("第 {i} 号的回答。"));
        first.pump(Duration::from_millis(100));
    }
    drop(first);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("第 22 号的回答。");
    tui.pump(Duration::from_millis(300));
    tui.say("/jump");
    // 更早的那一页不等滚到顶就读上来：22 句都在。
    tui.wait_for("跳到  22 条");
    tui.type_text("第 1 句");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    tui.pump(Duration::from_millis(500));
    let lines = tui.lines();
    let at = lines
        .iter()
        .position(|l| l.contains("┃ 第 1 句"))
        .unwrap_or_else(|| panic!("没跳到第 1 句：\n{}", lines.join("\n")));
    assert!(at <= 2, "第 1 句顶到最上面：\n{}", lines.join("\n"));
}

/// 排期第 15 条（2026-11-02 项目主人）：侧边栏「跳转消息」列这个会话里你说的话，新的在下（发版前 check 第 18 条，2026-11-09
/// 项目主人，原来新的在上）；点一句跳过去，同 `/jump` 选它。这一段在「最近会话」上面（发版前 check 第 22 条，同一天项目主人）。
#[test]
fn the_sidebar_lists_what_was_said_and_a_click_jumps_there() {
    let home = Home::new(Script::new([
        Play::Says(long("甲")),
        Play::Says(long("乙")),
        Play::Says(long("丙")),
    ]));
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    for (said, last) in [
        ("第一问 alpha", "甲 第 10 段。"),
        ("第二问 beta", "乙 第 10 段。"),
        ("第三问 gamma", "丙 第 10 段。"),
    ] {
        tui.say(said);
        tui.wait_for(last);
    }
    tui.wait_for("跳转消息");
    tui.pump(Duration::from_millis(300));
    // 侧边栏那几列里的（窗口 140 列宽，侧边栏在右边）：新的在下。照显示宽度认：左边正文里有中文的一行，字数比列数少。
    let lines = tui.lines();
    let side = |word: &str, from: usize| {
        lines
            .iter()
            .skip(from)
            .position(|l| {
                l.find(word)
                    .is_some_and(|at| unicode_width::UnicodeWidthStr::width(&l[..at]) > 100)
            })
            .map(|at| at + from)
    };
    // 「最近会话」里会话的标题也是第一问：从「跳转消息」那一行往下找，它在「最近会话」上面。
    let title = side("跳转消息", 0);
    let from = title.unwrap_or_else(|| panic!("有跳转消息那一段：\n{}", lines.join("\n")));
    assert!(
        side("最近会话", 0).is_some_and(|recent| from < recent),
        "跳转消息在最近会话上面：\n{}",
        lines.join("\n")
    );
    let (gamma, beta, alpha) = (
        side("第三问 gamma", from),
        side("第二问 beta", from),
        side("第一问 alpha", from),
    );
    assert!(
        title < alpha && alpha < beta && beta < gamma,
        "新的在下：\n{}",
        lines.join("\n")
    );
    let main_has = |lines: &[String]| {
        lines.iter().position(|l| {
            l.find("第二问 beta")
                .is_some_and(|at| unicode_width::UnicodeWidthStr::width(&l[..at]) < 100)
        })
    };
    assert!(main_has(&lines).is_none(), "第二问早滚出视口了");
    // 排期第 44 条（2026-11-04 项目主人：「箭头。不要用原点。」）：当前这一句打头写箭头，跟着最新的是最后一句。
    let marked =
        |lines: &[String], word: &str| lines.iter().any(|l| l.contains(&format!("❯ {word}")));
    assert!(
        marked(&lines, "第三问 gamma") && !marked(&lines, "第二问 beta"),
        "跟着最新的标最后一句：\n{}",
        lines.join("\n")
    );
    // 点侧边栏里「第二问 beta」那一行：正文里那一句顶到视口最上面。
    let row = beta.expect("列着");
    let col = unicode_width::UnicodeWidthStr::width(
        &lines[row][..lines[row].find("第二问").expect("在")],
    ) + 1;
    tui.key(format!("\x1b[<0;{col};{0}M\x1b[<0;{col};{0}m", row + 1).as_bytes());
    let start = std::time::Instant::now();
    loop {
        tui.pump(Duration::from_millis(20));
        if main_has(&tui.lines()).is_some_and(|at| at <= 2) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "没跳过去：\n{}",
            tui.lines().join("\n")
        );
    }
    // 跳过去了：视口最上面是第二问那一段，箭头挪到它。
    let start = std::time::Instant::now();
    while !marked(&tui.lines(), "第二问 beta") {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "箭头没挪到跳到的那一句：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(20));
    }
    assert!(!marked(&tui.lines(), "第三问 gamma"), "只标一句");
}
