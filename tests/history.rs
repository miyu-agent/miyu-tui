//! 伪终端里的端到端测试：输入历史重启以后还在、所有会话一起（蓝图 `tui.md`「输入历史列表」第 8 条，2026-10-07 项目主人定）。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

const UP: &[u8] = b"\x1b[A";

/// 输入框里那一行。
fn input(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('❯'))
        .unwrap_or_default()
}

/// 按 ↑ 直到输入框里是 `text`。输入历史等核心交回软件包列表（状态目录在哪）才读进来，连上那一刻按的 ↑ 可能赶在它前面：
/// 没翻出来的收掉再按，最多等几秒（2026-10-11：本地十次红一次）。
fn up_until(tui: &mut support::Tui, text: &str) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        tui.key(UP);
        tui.pump(Duration::from_millis(300));
        if input(tui).contains(text) {
            return;
        }
        assert!(
            std::time::Instant::now() < end,
            "↑ 翻不到「{text}」：{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        tui.pump(Duration::from_millis(200));
    }
}

#[test]
fn what_was_sent_comes_back_with_up_after_a_restart_even_in_a_new_session() {
    let home = Home::new(Script::new([Play::Says("好。"), Play::Says("嗯。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("重启以前说的一句");
    tui.wait_for("好。");
    drop(tui);
    // 重启：新的界面、新的会话（ui.startup 照默认开新的）。
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    up_until(&mut tui, "重启以前说的一句");
    // 记在终端这个软件包的状态目录里（核心 9-1 上，`package.list` 的 `state`）。
    let saved =
        std::fs::read_to_string(home.data().package_state("tui").join("history.jsonl")).unwrap();
    assert_eq!(saved.lines().count(), 1, "记在包的状态目录里：{saved}");
}

/// 排期第 17 条（2026-11-03 照 Claude Code 改版）：`Ctrl+R` 每条左边时间、右边一行预览，下面露选中那条的全文，最下面是搜索
/// 那一行；直接打字就是模糊搜索（框线上不写「搜索：」），`Enter` 填进输入框。
#[test]
fn ctrl_r_shows_times_a_preview_and_a_search_row_and_typing_filters() {
    let home = Home::new(Script::new([Play::Says("好。"), Play::Says("嗯。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.say("帮我跑一下门禁");
    tui.wait_for("好。");
    tui.say("改一下侧边栏的顺序");
    tui.wait_for("嗯。");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\x12");
    tui.wait_for("历史  2 条");
    tui.wait_for("筛选历史…");
    let lines = tui.lines();
    let row = |word: &str| {
        lines
            .iter()
            .position(|l| l.contains(word))
            .unwrap_or_else(|| panic!("没有「{word}」：\n{}", lines.join("\n")))
    };
    let list = row("刚才 ");
    assert!(
        lines[list].find("刚才").unwrap() < lines[list].find("门禁").unwrap(),
        "时间在左边：\n{}",
        lines.join("\n")
    );
    // 选中的是最新的那条：下面的预览又写了一遍它。
    let picked = lines
        .iter()
        .filter(|l| l.contains("改一下侧边栏的顺序") && l.contains('│'))
        .count();
    assert_eq!(picked, 2, "列表一行、预览一行：\n{}", lines.join("\n"));
    // 预览装在圆角框里（发版前 check 第 31 条），搜索那一行上面的横线接上左右的框线。
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.trim().starts_with('├') && l.trim().ends_with('┤'))
            .count(),
        1,
        "{}",
        lines.join("\n")
    );
    let inner = |corner: char| {
        lines
            .iter()
            .filter(|l| l.trim().starts_with('│') && l.contains(corner))
            .count()
    };
    assert_eq!((inner('╭'), inner('╰')), (1, 1), "{}", lines.join("\n"));
    assert!(
        row("筛选历史…") > row("改一下侧边栏的顺序"),
        "搜索那一行在最下面"
    );
    // 模糊搜索：「跑门」照先后对得上「帮我跑一下门禁」。
    tui.type_text("跑门");
    tui.wait_for("历史  1 条");
    assert!(
        !tui.shows("搜索："),
        "框线上不写搜索：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\r");
    tui.pump(Duration::from_millis(300));
    assert!(
        input(&tui).contains("帮我跑一下门禁") && !tui.shows("历史  1 条"),
        "填进输入框、列表关了：\n{}",
        tui.lines().join("\n")
    );
}

/// 发版前 check 第 52 条（2026-11-12 项目主人：「Ctrl+R 打开的历史的列表有点太矮了……预览框默认只有一行，导致会因为内容
/// 上下来回变化高度」）：框的高度打开时定死，上下选、搜索筛少了都不变；框够宽时预览在右边，竖线接上上边框（`┬`）和横线
/// （`┴`），窗口变窄照上下排。
#[test]
fn ctrl_r_keeps_its_height_and_puts_the_preview_on_the_right_when_wide() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    // 30 条历史（连进会话说的那句 31 条）：双数的三行、单数的一行（原来露着的几条长短不一，预览就一伸一缩）。
    let state = home.data().package_state("tui");
    std::fs::create_dir_all(&state).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let saved: String = (0..30u64)
        .rev()
        .map(|n| {
            let text = if n % 2 == 0 {
                format!("第 {n} 条\n第二行\n第三行")
            } else {
                format!("第 {n} 条")
            };
            let line = serde_json::json!({"at": now - n * 60_000, "text": text});
            format!("{line}\n")
        })
        .collect();
    std::fs::write(state.join("history.jsonl"), saved).unwrap();
    let mut tui = home.tui_wide("zh_CN.UTF-8", 180);
    tui.wait_for("开放权限");
    // 先说一句进会话（首页的输入框窄），历史多一条。
    tui.say("你好");
    tui.wait_for("好。");
    open_history(&mut tui, "历史  31 条");
    let wide = box_rows(&tui);
    eprintln!("===== 180 列 =====\n{}", tui.lines().join("\n"));
    let top = tui.lines()[wide.0].clone();
    assert!(
        top.contains('┬'),
        "宽框预览在右边：\n{}",
        tui.lines().join("\n")
    );
    assert!(
        tui.lines().iter().any(|l| l.contains('┴')),
        "{}",
        tui.lines().join("\n")
    );
    for _ in 0..5 {
        tui.key(UP);
        tui.pump(Duration::from_millis(150));
        assert_eq!(
            box_rows(&tui),
            wide,
            "上下选框不变：\n{}",
            tui.lines().join("\n")
        );
    }
    // 筛少了：框照旧。
    tui.type_text("29");
    tui.wait_for("历史  1 条");
    assert_eq!(
        box_rows(&tui),
        wide,
        "筛少了框不变：\n{}",
        tui.lines().join("\n")
    );
    tui.type_text("xyz");
    tui.wait_for("历史  0 条");
    assert_eq!(box_rows(&tui), wide, "{}", tui.lines().join("\n"));
    // 140 列：侧边栏占掉一块，框里不到 100 列，上下排。
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(200));
    tui.resize(40, 140);
    tui.pump(Duration::from_millis(500));
    open_history(&mut tui, "历史  31 条");
    eprintln!("===== 140 列 =====\n{}", tui.lines().join("\n"));
    // 窗口变窄：上下排，预览固定 6 行；框还是一打开就定死。
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(200));
    tui.resize(40, 80);
    tui.pump(Duration::from_millis(500));
    open_history(&mut tui, "历史  31 条");
    let narrow = box_rows(&tui);
    eprintln!("===== 80 列 =====\n{}", tui.lines().join("\n"));
    assert!(
        !tui.lines()[narrow.0].contains('┬'),
        "窄框上下排：\n{}",
        tui.lines().join("\n")
    );
    for _ in 0..5 {
        tui.key(UP);
        tui.pump(Duration::from_millis(150));
        assert_eq!(box_rows(&tui), narrow, "{}", tui.lines().join("\n"));
    }
    tui.type_text("29");
    tui.wait_for("历史  1 条");
    assert_eq!(box_rows(&tui), narrow, "{}", tui.lines().join("\n"));
}

/// 按 `Ctrl+R` 直到框的上边写着 `title`（输入历史等核心交回软件包列表才读进来）。
fn open_history(tui: &mut support::Tui, title: &str) {
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        tui.key(b"\x12");
        tui.pump(Duration::from_millis(300));
        if tui.shows(title) {
            return;
        }
        assert!(
            std::time::Instant::now() < end,
            "打不开「{title}」：\n{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        tui.pump(Duration::from_millis(200));
    }
}

/// 框的上边、下边在屏幕第几行（上边写着「历史」，下边写着按键提示）。
fn box_rows(tui: &support::Tui) -> (usize, usize) {
    let lines = tui.lines();
    let top = lines
        .iter()
        .position(|l| l.contains("历史  "))
        .expect("框开着");
    let bottom = lines
        .iter()
        .position(|l| l.contains("Esc 关闭"))
        .expect("下边写着按键提示");
    (top, bottom)
}
