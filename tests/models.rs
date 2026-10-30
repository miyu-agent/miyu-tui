//! 伪终端里的端到端测试：换模型、思考强度（蓝图 `tui.md`「配置与模型」）。真界面连一份照剧本回话的核心。

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::Home;

const DOWN: &[u8] = b"\x1b[B";

/// 底栏那一行。
fn footer(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .rev()
        .find(|l| l.contains("工作区"))
        .unwrap_or_default()
}

/// 等底栏里出现 `want`。
fn wait_footer(tui: &mut support::Tui, want: &str) {
    let end = std::time::Instant::now() + support::WAIT;
    while !footer(tui).contains(want) {
        assert!(
            std::time::Instant::now() < end,
            "底栏没有「{want}」：{}",
            footer(tui)
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 两个模型：v4 默认强度 low（默认的聊天模型），v5 默认强度 high；一个池。供应商、模型照假核心报的写。
const TWO: &str = "[models]\nchat = \"deepseek/deepseek-v4\"\n\n[pools.duo]\nmodels = [\"deepseek/deepseek-v4\", \"deepseek/deepseek-v5\"]\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nreasoning = [\"off\", \"low\", \"high\"]\neffort = \"low\"\n\n[providers.deepseek.models.deepseek-v5]\nreasoning = [\"off\", \"high\"]\neffort = \"high\"\n";

#[test]
fn a_model_picked_by_hand_shows_its_effort_at_once_and_becomes_the_new_session_default() {
    // 2026-10-02 项目主人报：换了模型，底栏的思考强度还是原来那个的；开新会话又回到原来的模型。定：手动换到哪个，
    // 哪个就是新会话的默认。
    let home = Home::with_settings(Script::new([Play::Says("好。")]), TWO);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    wait_footer(&mut tui, "low");
    tui.say("在吗");
    tui.wait_for("▣  ");
    tui.say("/model");
    tui.wait_for("deepseek-v5");
    tui.type_text("v5");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    wait_footer(&mut tui, "high");
    let end = std::time::Instant::now() + support::WAIT;
    while !home.settings().contains("chat = \"deepseek/deepseek-v5\"") {
        assert!(
            std::time::Instant::now() < end,
            "没记成新会话的默认：{}",
            home.settings()
        );
        tui.pump(Duration::from_millis(100));
    }
    tui.say("/new");
    tui.pump(Duration::from_millis(500));
    wait_footer(&mut tui, "deepseek-v5");
    wait_footer(&mut tui, "high");
}

#[test]
fn switching_to_a_pool_leaves_no_stale_effort_in_the_footer() {
    // 2026-10-02 项目主人报：换成池以后底栏还写着原来那个模型的强度。
    let home = Home::with_settings(Script::new([Play::Says("好。")]), TWO);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    wait_footer(&mut tui, "low");
    tui.say("/model");
    tui.wait_for("@duo");
    tui.type_text("duo");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    wait_footer(&mut tui, "@duo");
    tui.pump(Duration::from_millis(500));
    assert!(!footer(&tui).contains("low"), "{}", footer(&tui));
}

#[test]
fn choosing_a_model_shows_no_notice() {
    // 2026-10-01 项目主人：改模型、思考强度不需要通知。底栏当场写成选的那个。
    let settings = "[models]\nchat = \"dev/alpha\"\n\n[providers.dev]\ndriver = \"openai-chat\"\nbase_url = \"http://dev.invalid/v1\"\n\n[providers.dev.models.alpha]\nwindow = 128000\n\n[providers.dev.models.beta]\nwindow = 64000\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("▣  ");
    tui.say("/model");
    tui.wait_for("dev · 64k");
    // 按键模式出厂是方向键：打字就搜，`Enter` 一下就选（2026-10-21 项目主人：「新用户默认不开 vimkey」）。
    tui.wait_for("↑/↓ 选择");
    tui.type_text("beta");
    tui.pump(Duration::from_millis(300));
    tui.key(b"\r");
    tui.wait_for("beta dev");
    tui.pump(Duration::from_millis(300));
    assert!(!tui.shows("下一轮"), "{}", tui.lines().join("\n"));
}

#[test]
fn effort_is_chosen_for_this_session_only() {
    // 2026-10-30 项目主人定（核心 SM-1）：/effort 只管当前会话，不写系统配置；新开的会话照配置。第一行「跟随设置」。
    let settings = "[models]\nchat = \"deepseek/deepseek-v4\"\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nwindow = 128000\nreasoning = [\"off\", \"low\", \"high\"]\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("▣  ");
    tui.say("/effort");
    tui.wait_for("跟随设置");
    assert!(tui.shows("high"), "{}", tui.lines().join("\n"));
    // 第 0 行跟随设置，下面 off、low、high。
    for _ in 0..3 {
        tui.key(DOWN);
    }
    tui.key(b"\r");
    wait_footer(&mut tui, "high");
    assert!(
        !home.settings().contains("effort"),
        "不写系统配置：{}",
        home.settings()
    );
    // 新开的会话照配置（没配的不写哪一档），不带上一个会话定的。
    tui.say("/new");
    let end = std::time::Instant::now() + support::WAIT;
    while footer(&tui).contains("high") {
        assert!(
            std::time::Instant::now() < end,
            "新会话还写着：{}",
            footer(&tui)
        );
        tui.pump(Duration::from_millis(100));
    }
}

#[test]
fn effort_chosen_before_a_session_shows_at_once_and_goes_with_the_new_session() {
    // 还没开会话时 /effort：记着，开会话时带上；底栏当场写它（2026-10-02 项目主人报：换了要等下一轮才变）。
    let settings = "[models]\nchat = \"deepseek/deepseek-v4\"\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nwindow = 128000\nreasoning = [\"off\", \"low\", \"high\"]\neffort = \"low\"\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    wait_footer(&mut tui, "low");
    tui.say("/effort");
    tui.wait_for("跟随设置");
    // 第 0 行跟随设置，下面 off、low、high。
    for _ in 0..3 {
        tui.key(DOWN);
    }
    tui.key(b"\r");
    wait_footer(&mut tui, "high");
    tui.say("在吗");
    tui.wait_for("好。");
    wait_footer(&mut tui, "high");
    assert!(
        !home.settings().contains("effort = \"high\""),
        "不写系统配置：{}",
        home.settings()
    );
}

#[test]
fn a_pool_does_not_let_effort_change() {
    // 2026-10-02 项目主人定：用的是模型池不让改思考强度，/effort 只提示一句。
    let settings = "[models]\nchat = \"@duo\"\n\n[pools.duo]\nmodels = [\"deepseek/deepseek-v4\"]\n\n[providers.deepseek]\ndriver = \"openai-chat\"\nbase_url = \"http://deepseek.invalid/v1\"\n\n[providers.deepseek.models.deepseek-v4]\nreasoning = [\"off\", \"high\"]\n";
    let home = Home::with_settings(Script::new([]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/effort");
    tui.wait_for("当前使用的是模型池");
    assert!(!tui.shows("跟随设置"), "{}", tui.lines().join("\n"));
}

#[test]
fn a_new_session_does_not_flash_an_empty_footer() {
    // 2026-10-02 项目主人报：开新会话，底栏的思考强度闪一下。原来换上空的正文时底栏先空着，等核心交回默认的模型再写。
    let home = Home::with_settings(Script::new([Play::Says("好。")]), TWO);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("▣  ");
    // 假核心开了会话以后不报思考强度（不照配置算）：开一下 /effort 的框（开框要一次 `model.list`），底栏照配置的那一档
    // （`TWO` 里是 low）写上；开新会话的每一帧都还写着它，不先空一下。
    tui.say("/effort");
    tui.wait_for("跟随设置");
    tui.key(b"\x1b");
    wait_footer(&mut tui, "low");
    let start = tui.record_from_here();
    tui.say("/new");
    tui.pump(Duration::from_millis(800));
    let frames = support::frames(start, &tui.recorded());
    assert!(!frames.is_empty());
    for (i, frame) in frames.iter().enumerate() {
        let footer = frame.iter().rev().find(|l| l.contains("工作区"));
        if let Some(footer) = footer {
            assert!(footer.contains("low"), "第 {i} 帧底栏闪空了：{footer}");
        }
    }
}

#[test]
fn with_vim_keys_a_slash_searches_and_enter_keeps_the_filter() {
    // 2026-10-21 项目主人：「搜索用 vim 的方式，左斜杠键」「和 vim 一样。然后下次再 / 就清空搜索结果后再次进入搜索模式」。
    let settings = "[models]\nchat = \"dev/alpha\"\n\n[providers.dev]\ndriver = \"openai-chat\"\nbase_url = \"http://dev.invalid/v1\"\n\n[providers.dev.models.alpha]\nwindow = 128000\n\n[providers.dev.models.beta]\nwindow = 64000\n\n[tui]\nkeys = \"vim\"\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("/model");
    tui.wait_for("dev · 64k");
    tui.wait_for("/ 搜索 · j/k 选择");
    tui.type_text("bet");
    assert!(!tui.shows("搜索：bet"), "没按 / 打的字不进搜索");
    tui.type_text("/bet");
    tui.wait_for("搜索：bet");
    tui.wait_for("Ctrl+J/K 选择 · Enter 完成");
    tui.key(b"\r");
    tui.wait_for("/ 搜索 · j/k 选择");
    assert!(tui.shows("搜索：bet"), "Enter 收起，筛的留着");
    assert!(!tui.shows("dev · 128k"), "{}", tui.lines().join("\n"));
    tui.type_text("/");
    tui.wait_for("dev · 128k");
    assert!(!tui.shows("搜索：bet"), "再按 / 清空了重打");
    tui.type_text("al");
    tui.key(b"\r");
    tui.key(b"\x1b");
    tui.wait_for("dev · 64k");
    assert!(tui.shows("dev · 128k"), "Esc 先去掉筛的，框还开着");
    tui.key(b"\x1b");
    tui.pump(Duration::from_millis(300));
    assert!(!tui.shows("dev · 64k"), "再按 Esc 关框");
}
