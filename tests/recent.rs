//! 伪终端里的端到端测试：侧边栏「最近会话」（蓝图 `tui.md`「后台命令、子代理和侧边栏」第 7 条，2026-10-26 项目主人，反馈
//! 第 21 条）。照会话列表的先后，当前这个在原位、行首圆点；点一行切过去，切过去的不挪到最上面。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn the_sidebar_lists_recent_sessions_and_a_click_switches() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    // 没标题的照预览：新的（topic-two，也是当前这个）在上面，topic-one 在下面。从「最近会话」那一行往下找：上面「跳转消息」
    // 里也列着你说的 topic-two（发版前 check 第 22 条，跳转消息挪到了最近会话上面）。
    let side = |lines: &[String], word: &str| {
        let from = lines.iter().position(|l| l.contains("最近会话"))?;
        lines
            .iter()
            .skip(from)
            .position(|l| l.find(word).is_some_and(|at| at > 100))
            .map(|at| at + from)
    };
    let lines = tui.lines();
    let (title, two, one) = (
        lines.iter().position(|l| l.contains("最近会话")),
        side(&lines, "topic-two"),
        side(&lines, "topic-one"),
    );
    assert!(
        title < two && two < one,
        "新的在上面：\n{}",
        lines.join("\n")
    );
    // 记号挂在缩进那两格里，名字和别的几段的内容对齐（2026-10-27 项目主人：缩进有待优化）。
    let col = |row: usize, word: &str| {
        lines[row][..lines[row].find(word).expect("在")]
            .chars()
            .count()
    };
    let usage = lines
        .iter()
        .position(|l| l.find("共 ").is_some_and(|at| at > 100))
        .expect("有用量那一段");
    assert_eq!(
        col(one.expect("列着"), "topic-one"),
        col(usage, "共 "),
        "\n{}",
        lines.join("\n")
    );
    // 点 topic-one 那一行：切过去，正文换成它的。
    let row = one.expect("列着 topic-one");
    let col = lines[row].find("topic-one").expect("在") + 1;
    let at = format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1);
    tui.key(at.as_bytes());
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("answer-two") {
        assert!(
            std::time::Instant::now() < end,
            "没切过去：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    // 切过去的不顶到最上面（2026-10-27 项目主人：「正常按照时间顺序就好了。否则很难找」）：照旧 topic-two 在上，当前这个的
    // 箭头（2026-10-30 起，原来是圆点）挪到 topic-one 那一行。
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        let lines = tui.lines();
        let (two, one) = (side(&lines, "topic-two"), side(&lines, "topic-one"));
        let marked = one.is_some_and(|row| lines[row].contains("❱ topic-one"));
        if marked {
            assert!(two < one, "切过去的不挪：\n{}", lines.join("\n"));
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "箭头没挪到切过去的那一行：\n{}",
            lines.join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

#[test]
fn clicking_a_recent_session_twice_never_flashes_an_empty_session() {
    // 2026-10-27 项目主人报：侧边栏切会话有时一闪空会话的画面。点了还没换上来又点一下，第二下把停放着、还在补发的空正文
    // 提前换了上来：一帧首页，等补完才有字。
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    let lines = tui.lines();
    let row = lines
        .iter()
        .position(|l| l.find("topic-one").is_some_and(|at| at > 100))
        .expect("列着 topic-one");
    let col = lines[row].find("topic-one").expect("在") + 1;
    let click = format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1);
    let start = tui.record_from_here();
    tui.key(format!("{click}{click}").as_bytes());
    tui.wait_for("answer-one");
    tui.pump(std::time::Duration::from_millis(300));
    let frames = support::frames(start, &tui.recorded());
    assert!(!frames.is_empty());
    for frame in &frames {
        let body = |word: &str| {
            frame
                .iter()
                .any(|l| l.find(word).is_some_and(|at| at < 100))
        };
        assert!(
            body("answer-one") || body("answer-two"),
            "一帧空会话：\n{}",
            frame.join("\n")
        );
    }
}

/// 发版前 check 第 17 条（2026-11-09 项目主人）：框里打了字再从「最近会话」切到别的会话，字不跟过去；切回来还在。
#[test]
fn switching_sessions_leaves_the_typed_text_with_its_session() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    tui.type_text("draft-for-two");
    tui.pump(std::time::Duration::from_millis(200));
    assert!(tui.input().contains("draft-for-two"), "{}", tui.input());
    let click = |tui: &mut support::Tui, word: &str| {
        let lines = tui.lines();
        let row = lines
            .iter()
            .position(|l| l.find(word).is_some_and(|at| at > 100))
            .expect("列着");
        let col = lines[row].find(word).expect("在") + 1;
        let at = format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1);
        tui.key(at.as_bytes());
    };
    click(&mut tui, "topic-one");
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("answer-two") {
        assert!(std::time::Instant::now() < end, "没切过去");
        tui.pump(std::time::Duration::from_millis(50));
    }
    assert!(
        !tui.input().contains("draft-for-two"),
        "字不该跟过去：{}",
        tui.input()
    );
    click(&mut tui, "topic-two");
    tui.wait_for("answer-two");
    let end = std::time::Instant::now() + support::WAIT;
    while !tui.input().contains("draft-for-two") {
        assert!(
            std::time::Instant::now() < end,
            "切回来字该还在：{}",
            tui.input()
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

/// 发版前 check 第 17 条后续（2026-11-09 项目主人）：`Ctrl+C` 清掉的那一句也跟着会话走，别的会话按 `↑` 找不回它，切回来找得回。
#[test]
fn the_cleared_text_comes_back_only_in_its_own_session() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    tui.type_text("cleared-in-two");
    tui.pump(std::time::Duration::from_millis(200));
    tui.key(b"\x03");
    tui.pump(std::time::Duration::from_millis(200));
    assert!(!tui.input().contains("cleared-in-two"), "{}", tui.input());
    let click = |tui: &mut support::Tui, word: &str| {
        let lines = tui.lines();
        let row = lines
            .iter()
            .position(|l| l.find(word).is_some_and(|at| at > 100))
            .expect("列着");
        let col = lines[row].find(word).expect("在") + 1;
        let at = format!("\x1b[<0;{col};{}M\x1b[<0;{col};{}m", row + 1, row + 1);
        tui.key(at.as_bytes());
    };
    click(&mut tui, "topic-one");
    tui.wait_for("answer-one");
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("answer-two") {
        assert!(std::time::Instant::now() < end, "没切过去");
        tui.pump(std::time::Duration::from_millis(50));
    }
    tui.key(b"\x1b[A");
    tui.pump(std::time::Duration::from_millis(300));
    assert!(
        !tui.input().contains("cleared-in-two"),
        "别的会话按 ↑ 不该找回：{}",
        tui.input()
    );
    tui.key(b"\x1b[B");
    tui.pump(std::time::Duration::from_millis(200));
    click(&mut tui, "topic-two");
    tui.wait_for("answer-two");
    tui.pump(std::time::Duration::from_millis(300));
    tui.key(b"\x1b[A");
    let end = std::time::Instant::now() + support::WAIT;
    while !tui.input().contains("cleared-in-two") {
        assert!(
            std::time::Instant::now() < end,
            "切回来按 ↑ 该找回：{}",
            tui.input()
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

/// 发版前 check 第 23 条（2026-11-09 项目主人：侧边栏最近会话、跳转消息自己填数，0 是不显示）：清单里是 int，配置写成数，侧边栏照它列。
#[test]
fn the_sidebar_counts_are_plain_numbers_and_zero_hides_the_list() {
    let script = Script::new([Play::Says("answer-one"), Play::Says("answer-two")]);
    let home = Home::with_settings(script, "[tui]\nsidebar_sessions = 1\nsidebar_jumps = 0\n");
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("开放权限");
    tui.say("topic-one");
    tui.wait_for("answer-one");
    tui.say("/new");
    tui.say("topic-two");
    tui.wait_for("answer-two");
    tui.wait_for("最近会话");
    tui.pump(std::time::Duration::from_millis(300));
    let lines = tui.lines();
    let side = |word: &str| {
        lines
            .iter()
            .any(|l| l.find(word).is_some_and(|at| at > 100))
    };
    assert!(
        side("topic-two") && !side("topic-one") && !side("跳转消息"),
        "最近会话只列 1 个，跳转消息不画：\n{}",
        lines.join("\n")
    );
}
