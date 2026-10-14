//! 伪终端里的端到端测试：Tab、Shift+Tab 切权限级别（蓝图 `tui.md`「权限级别」第 2 条）。真界面连一份照剧本回话的核心。
//! 2026-10-14 项目主人报「Tab 切不了权限级别了」：首页选完人格、预设以后按 Tab，底栏一直是「工作区」。`Tab`、`Shift+Tab`
//! 都往下一档轮（工作区 → 开放权限 → 只读 → 工作区）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 不照测具默认的无人格、全部功能：弹人格框、预设框（项目主人的走法）。
const ASK: [(&str, &str); 2] = [("MIYU_TUI_PERSONA", ""), ("MIYU_TUI_PRESET", "")];
const BACK_TAB: &[u8] = b"\x1b[Z";

/// 底栏左下角那一行（有级别的那一行）。
fn footer(tui: &Tui) -> String {
    tui.lines()
        .into_iter()
        .rev()
        .find(|l| ["工作区", "开放权限", "只读"].iter().any(|w| l.contains(w)))
        .unwrap_or_default()
}

/// 按一下 `key`，等底栏换成 `level`。
fn press_until(tui: &mut Tui, key: &[u8], level: &str) {
    tui.key(key);
    wait_level(tui, level);
}

/// 等底栏写着 `level`。
fn wait_level(tui: &mut Tui, level: &str) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !footer(tui).contains(level) {
        assert!(
            std::time::Instant::now() < end,
            "底栏没换成「{level}」：{}\n{}",
            footer(tui),
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(100));
    }
}

#[test]
fn tab_cycles_the_level_on_the_home_screen_after_picking_a_persona_and_preset() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    home.persona("pirate", "[persona]\nname = \"海盗\"\n", None);
    let mut tui = home.tui_with("zh_CN.UTF-8", &ASK);
    tui.wait_for("请选择人格");
    tui.key(b"\r");
    tui.wait_for("请选择预设");
    tui.key(b"\r");
    // 预设框开着时输入框的占位字也在：等框关了再按（框开着时 Tab 归框）。
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while tui.shows("请选择预设") || tui.shows("请选择人格") {
        assert!(
            std::time::Instant::now() < end,
            "{}",
            tui.lines().join("\n")
        );
        tui.pump(std::time::Duration::from_millis(50));
    }
    tui.wait_for("Tab 切换权限级别");
    assert!(footer(&tui).contains("工作区"), "{}", footer(&tui));
    press_until(&mut tui, b"\t", "开放权限");
    press_until(&mut tui, b"\t", "只读");
    press_until(&mut tui, BACK_TAB, "工作区");
}

#[test]
fn tab_cycles_the_level_in_an_opened_session() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("工作区");
    tui.say("在吗");
    tui.wait_for("好。");
    press_until(&mut tui, b"\t", "开放权限");
    press_until(&mut tui, BACK_TAB, "只读");
}

#[test]
fn a_full_default_shows_on_the_home_screen_and_tab_goes_on_from_the_sessions_real_level() {
    // 2026-10-14 项目主人报「会话里 Tab 切不了」（核心查到根因）：引导的沙盒那一步选了完全放开，个人设置
    // `permission.level = "full"`，新会话开成开放权限；界面却首页、会话里都画工作区，按 Tab 照工作区算下一档发 full，核心
    // 那边没变、不推，永远卡在工作区。
    let settings = "[permission]\nlevel = \"full\"\n";
    let home = Home::with_settings(Script::new([Play::Says("好。")]), settings);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    wait_level(&mut tui, "开放权限");
    tui.say("在吗");
    tui.wait_for("好。");
    wait_level(&mut tui, "开放权限");
    press_until(&mut tui, b"\t", "只读");
    press_until(&mut tui, b"\t", "工作区");
}

/// 测试家目录里唯一的那个会话的编号。
fn only_session(home: &Home) -> String {
    let dir = home.root().join("home/alice/sessions");
    let ids: Vec<String> = std::fs::read_dir(dir)
        .expect("会话目录")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_eq!(ids.len(), 1, "{ids:?}");
    ids[0].clone()
}

#[test]
fn a_resumed_session_comes_back_at_the_level_it_was_left_at() {
    // 2026-10-14 项目主人：herdr 退出后再开，照 `--resume` 恢复的会话，权限级别和退出前一样（核心重启过的也一样）。
    for restart in [false, true] {
        let home = Home::new(Script::new([Play::Says("好。")]));
        let mut tui = home.tui("zh_CN.UTF-8");
        tui.wait_for("工作区");
        tui.say("在吗");
        tui.wait_for("好。");
        press_until(&mut tui, b"\t", "开放权限");
        drop(tui);
        if restart {
            home.stop_core();
            home.restart_core();
        }
        let id = only_session(&home);
        let mut resumed = home.tui_args("zh_CN.UTF-8", &["--resume", &id]);
        resumed.wait_for("好。");
        wait_level(&mut resumed, "开放权限");
    }
}
