//! 伪终端里的端到端测试：新会话先选人格（蓝图 `tui.md`「新会话：人格、工作区」，2026-10-07 项目主人定：每次开新会话
//! 都先弹框选）。真界面连一份照剧本回话的核心，测试家目录里放几个人格。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;

/// 不照测具默认的无人格：弹人格框。
const ASK: (&str, &str) = ("MIYU_TUI_PERSONA", "");
/// 不照测具默认的全部功能：弹预设框。
const ASK_PRESET: (&str, &str) = ("MIYU_TUI_PRESET", "");
const UP: &[u8] = b"\x1b[A";

/// 输入框里那一行。
fn input(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('❯'))
        .unwrap_or_default()
}

#[test]
fn every_new_session_asks_for_a_persona_first_and_the_chosen_one_goes_into_the_session() {
    let script = Script::new([Play::Says("好。"), Play::Says("再见。")]);
    let home = Home::new(script.clone());
    home.persona(
        "pirate",
        "[persona]\nname = { zh = \"海盗\", en = \"Pirate\" }\n",
        Some("你是海盗，暗号 X1Y2。"),
    );
    home.persona("broken", "[persona]\nbad = 1\n", None);
    let mut tui = home.tui_with("zh_CN.UTF-8", &[ASK, ASK_PRESET]);
    tui.wait_for("请选择人格");
    tui.wait_for("海盗");
    assert!(tui.shows("broken"), "写错的也列着（选不了）");
    let broken = tui
        .lines()
        .into_iter()
        .find(|l| l.contains("broken"))
        .unwrap_or_default();
    // 原因照界面语言说（核心 P-3 再补，2026-10-08 起列表的 `problem` 照连接的语言给）。
    assert!(
        broken.contains("不认识的键"),
        "写错的后面写核心给的原因：{broken}"
    );
    // 框开着时打的字不进输入框：选了才能打字。
    tui.type_text("abc");
    assert!(!input(&tui).contains("abc"), "{}", input(&tui));
    // 第一行「无人格」（2026-10-08 项目主人：人格允许为空），默认人格没设，光标停在它上面；往下挪到海盗（写错的跳过）。
    assert!(tui.shows("无人格"), "{}", tui.lines().join("\n"));
    assert!(picked(&tui).contains("无人格"), "{}", picked(&tui));
    // 框开着不能打字：直接 j、k 上下（2026-10-11 项目主人）。
    for _ in 0..8 {
        if picked(&tui).contains("海盗") {
            break;
        }
        tui.key(b"j");
    }
    assert!(picked(&tui).contains("海盗"), "{}", tui.lines().join("\n"));
    tui.key(b"\r");
    // 人格选好了接着选预设（2026-10-08 项目主人定先人格、再预设）：照编号排 dev、full（出厂叫「常规开发」「全部功能」，核心 P-4 下；dev 原来叫「基础功能」，核心 2bc0e251 改名），光标停在默认的全部功能上。
    tui.wait_for("请选择预设");
    tui.wait_for("全部功能");
    tui.key(b"k");
    tui.key(b"\r");
    // 选好了框关上、能打字（首页那一行不写人格、预设，2026-10-08 项目主人）；选的照发出去的请求认。
    closed(&mut tui);
    // 还没说话再打开：选过的那个写 `●`（2026-10-20 项目主人报：空会话选了再打开没有圆点），没有切换前压缩那一行。
    tui.say("/persona");
    tui.wait_for("● 海盗");
    assert!(
        !tui.shows("切换前会先压缩会话"),
        "{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b");
    closed(&mut tui);
    tui.say("在吗");
    tui.wait_for("好。");
    let sent = format!("{:?}", script.requests().last());
    assert!(sent.contains("X1Y2"), "开会话带上了选的人格：{sent}");
    // 开了会话能中途换（2026-10-19 项目主人放开，核心 P-13）：同一个框，现在用的行首写 `●`；Esc 关、不换。会话还短、没东西可压缩
    // 的不写「切换前会先压缩」（核心 W-12 的 `compactable`，长会话的见下面那一条）。
    tui.say("/persona");
    tui.wait_for("● 海盗");
    assert!(
        !tui.shows("切换前会先压缩会话"),
        "{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b");
    closed(&mut tui);
    tui.say("/preset");
    tui.wait_for("请选择预设");
    tui.key(b"\x1b");
    closed(&mut tui);
    // `/new` 以后又先问。人格框是第一步，`Esc` 不动；预设框 `Esc` 回人格框重选（2026-10-23 项目主人：「esc 不应该是选择
    // 默认，而应该是返回」）。
    tui.say("/new");
    tui.wait_for("请选择人格");
    tui.key(b"\x1b");
    tui.pump(std::time::Duration::from_millis(300));
    assert!(
        tui.shows("请选择人格"),
        "人格框 Esc 不关：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\r");
    tui.wait_for("请选择预设");
    tui.key(b"\x1b");
    tui.wait_for("请选择人格");
    tui.key(b"\r");
    tui.wait_for("请选择预设");
    tui.key(b"\r");
    closed(&mut tui);
}

/// 框里选中的那一行（行首 `❯`，在输入框上面，先找到的就是它）。
fn picked(tui: &support::Tui) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('❯'))
        .unwrap_or_default()
}

/// 人格框、预设框都关上了：框的标题没了，输入框能打字。
fn closed(tui: &mut support::Tui) {
    let end = std::time::Instant::now() + support::WAIT;
    while tui.shows("请选择") {
        assert!(
            std::time::Instant::now() < end,
            "框没关：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
}

#[test]
fn a_session_needs_a_preset_and_a_broken_default_keeps_the_box_open() {
    // 核心 Y12：开会话一定要有预设；测具设了人格、没有人格框可回，`Esc` 不关、弹一句，选了才能打字。
    let home = Home::with_settings(
        Script::new([Play::Says("好。")]),
        "[preset]\ndefault = \"nope\"\n",
    );
    let mut tui = home.tui_with("zh_CN.UTF-8", &[ASK_PRESET]);
    tui.wait_for("请选择预设");
    tui.key(b"\x1b");
    tui.wait_for("请先选择预设");
    assert!(tui.shows("请选择预设"), "框还开着");
    tui.key(b"\r");
    closed(&mut tui);
    tui.say("在吗");
    tui.wait_for("好。");
}

#[test]
fn an_open_session_shows_its_persona_in_the_sidebar() {
    // 核心 P-1 下起订阅的回应带 `persona`：侧边栏会话名下面写一行（蓝图「新会话：人格、工作区」第 6 条）。
    let home = Home::new(Script::new([Play::Says("好。")]));
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", Some("你是海盗。"));
    let mut tui = home.tui_wide_with("zh_CN.UTF-8", 140, &[("MIYU_TUI_PERSONA", "pirate")]);
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.wait_for("人格 海盗");
    tui.wait_for("预设 全部功能");
}

#[test]
fn a_session_without_a_persona_shows_no_persona_line() {
    // 无人格的会话订阅回应里没有 `persona`（核心 P-4 上）：侧边栏不写人格那一行。
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui_wide("zh_CN.UTF-8", 140);
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.wait_for("预设 全部功能");
    assert!(!tui.shows("人格 "), "{}", tui.lines().join("\n"));
}

#[test]
fn choosing_no_persona_opens_a_session_without_one_even_when_a_default_is_set() {
    // 2026-10-08 项目主人：「软件默认不自带任何人格，人格允许为空」；框里第一行「无人格」，选了开会话时发
    // `"persona": null`，不照默认人格（核心 P-4 上）。
    let script = Script::new([Play::Says("好。")]);
    let home = Home::with_settings(script.clone(), "[persona]\ndefault = \"pirate\"\n");
    home.persona(
        "pirate",
        "[persona]\nname = \"海盗\"\n",
        Some("你是海盗，暗号 X1Y2。"),
    );
    let mut tui = home.tui_with("zh_CN.UTF-8", &[ASK]);
    tui.wait_for("请选择人格");
    tui.wait_for("不使用人格");
    assert!(
        !tui.shows("记忆不生效"),
        "无人格也有记忆了（2026-10-19 项目主人）"
    );
    // 光标停在默认的海盗上：挪到最上面的「无人格」。
    for _ in 0..5 {
        tui.key(UP);
    }
    tui.key(b"\r");
    closed(&mut tui);
    tui.say("在吗");
    tui.wait_for("好。");
    let sent = format!("{:?}", script.requests().last());
    assert!(!sent.contains("X1Y2"), "无人格不带默认人格的人设：{sent}");
}

#[test]
fn a_persona_switched_midway_compacts_first_then_speaks_as_the_new_one() {
    // 2026-10-19 项目主人放开中途换（核心 P-13）：先压缩，再照新的人格接着聊；正文记一行「已换成」。
    // 会话很短：核心压缩时不用请求模型，只剩两段回话。
    let script = Script::new([Play::Says("好。"), Play::Says("啊哈。")]);
    let home = Home::new(script.clone());
    home.persona(
        "pirate",
        "[persona]\nname = { zh = \"海盗\", en = \"Pirate\" }\n",
        Some("你是海盗，暗号 X1Y2。"),
    );
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.say("/persona");
    tui.wait_for("请选择人格");
    for _ in 0..8 {
        if picked(&tui).contains("海盗") {
            break;
        }
        tui.key(b"j");
        tui.pump(std::time::Duration::from_millis(50));
    }
    assert!(picked(&tui).contains("海盗"), "{}", tui.lines().join("\n"));
    tui.key(b"\r");
    tui.wait_for("人格已换成「海盗」");
    tui.say("你是谁");
    tui.wait_for("啊哈。");
    let sent = format!("{:?}", script.requests().last());
    assert!(sent.contains("X1Y2"), "换了以后照海盗说：{sent}");
}

#[test]
fn the_compaction_note_shows_only_when_there_is_something_to_compact() {
    // 2026-10-20 项目主人：「如果没什么可压缩的，切换就不应该提示要压缩」。照核心 W-12 的 `compactable`：订阅回应给的，之后照推来的
    // `compactable.changed`。短会话没东西可压；贴一大段进去（同核心的测试，7 万字）就有了。
    let script = Script::new([Play::Says("好。"), Play::Says("收到。")]);
    let home = Home::new(script);
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", None);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    tui.say("/persona");
    tui.wait_for("请选择人格");
    tui.pump(std::time::Duration::from_millis(200));
    assert!(
        !tui.shows("切换前会先压缩会话"),
        "短会话不写：\n{}",
        tui.lines().join("\n")
    );
    tui.key(b"\x1b");
    closed(&mut tui);
    let long = "x".repeat(70_000);
    tui.send(format!("\x1b[200~{long}\x1b[201~").as_bytes());
    tui.pump(std::time::Duration::from_millis(300));
    tui.send(b"\r");
    tui.wait_for("收到。");
    let end = std::time::Instant::now() + support::WAIT;
    loop {
        tui.say("/persona");
        tui.wait_for("请选择人格");
        tui.pump(std::time::Duration::from_millis(200));
        if tui.shows("切换前会先压缩会话") {
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "长会话要写：\n{}",
            tui.lines().join("\n")
        );
        tui.key(b"\x1b");
        closed(&mut tui);
    }
}

#[test]
fn the_new_session_pick_takes_the_place_of_the_input_box_and_typing_goes_nowhere() {
    // 2026-10-21 项目主人：「不是以列表的形式出，而是直接替换掉输入框。因为我刚才发现，如果开启会话加载慢了，还是会导致
    // 输入框有很短的时间可以输入内容」。
    let home = Home::new(Script::new([]));
    // 一个人格都没装的核心不问人格（列表是空的），装一个才有人格框。
    home.persona("pirate", "[persona]\nname = { zh = \"海盗\" }\n", None);
    let mut tui = home.tui_with("zh_CN.UTF-8", &[ASK, ASK_PRESET]);
    // 一起来就打字：列表读回来以前（「正在加载…」）、以后都不进输入框。
    tui.type_text("abc");
    tui.wait_for("请选择人格");
    tui.type_text("xyz");
    // 只有一个框：人格框就是输入框那一块，上面不另开框。
    let frames: Vec<String> = tui
        .lines()
        .into_iter()
        .filter(|l| l.trim_start().starts_with('╭'))
        .collect();
    assert_eq!(frames.len(), 1, "{}", tui.lines().join("\n"));
    assert!(frames[0].contains("请选择人格"), "{frames:?}");
    tui.key(b"\r");
    tui.wait_for("请选择预设");
    tui.key(b"\r");
    closed(&mut tui);
    tui.pump(std::time::Duration::from_millis(200));
    let line = input(&tui);
    assert!(
        !line.contains("abc") && !line.contains("xyz"),
        "选完出来的输入框是空的：{line}"
    );
}
