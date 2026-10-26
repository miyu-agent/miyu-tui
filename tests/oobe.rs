//! 第一次打开的引导（蓝图 `tui.md`「第一次打开的引导」）：`--page oobe` 起来，一步步走到底，查核心那边写下的人格、默认
//! 人格、默认预设。测具的核心没有目录、不能真的试一家，接模型那一步走「已配好 · 用它」；选一家、试、存的那一段在单元
//! 测试里（`src/oobe/model/tests.rs`）。

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 配好了一家、默认模型是它：接模型那一步写「已配好」。
const CONFIGURED: &str = "[providers.fake]\ndriver = \"openai-chat\"\nbase_url = \"http://127.0.0.1:9/v1\"\n\n[models]\nchat = \"fake/m\"\n";

/// 等 `check` 成立，最多 [`support::WAIT`]；等不到的把屏幕打出来。
fn until(tui: &mut Tui, what: &str, check: impl Fn() -> bool) {
    let end = Instant::now() + support::WAIT;
    while !check() {
        assert!(
            Instant::now() < end,
            "等不到{what}，屏幕是：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(100));
    }
}

/// 起引导，走过欢迎、语言、操作方式、图标、接模型（用已配好的），停在建人格。
fn to_persona(home: &Home) -> Tui {
    let mut tui = home.tui_args("zh_CN.UTF-8", &["--page", "oobe"]);
    tui.wait_for("按 Enter 开始");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    tui.key(b"\r");
    tui.wait_for("你喜欢的操作方式");
    tui.key(b"\r");
    tui.wait_for("方块或乱码");
    tui.key(b"\r");
    tui.wait_for("已配置");
    tui.key(b"\r");
    tui.wait_for("创建人格");
    tui
}

fn persona_file(home: &Home, name: &str) -> PathBuf {
    home.root().join("home/alice/personas/persona-1").join(name)
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// 主题那一步（2026-10-13 项目主人定）：挪到「下一步」回车，照停着的写（明暗、配色、铺底色、吉祥物那一格回车是开列表；
/// 人格有头像的没有吉祥物那一格，多按的一下停在「下一步」）。
fn through_theme(tui: &mut support::Tui) {
    tui.wait_for("选择明暗和配色");
    for _ in 0..4 {
        tui.key(b"j");
    }
    tui.key(b"\r");
}

/// 「AI 怎么认识你」那一步（2026-10-13 项目主人定，都可以不填；2026-10-16 起多一格回复语言）：挪到「下一步」回车。
fn through_profile(tui: &mut support::Tui) {
    tui.wait_for("AI 怎么称呼你");
    tui.key(b"j");
    tui.key(b"j");
    tui.key(b"j");
    tui.key(b"\r");
}

/// 语义模型那一步（第 26b 条）：测试的数据根没装它的包、软件源指到连不上的地方，这一屏照出、不写大小；选「是」当场下，
/// 下不成写原话、给「重试」「跳过」，跳过到好了，不写设置（2026-10-19 项目主人定）。
fn through_semantic(tui: &mut support::Tui) {
    tui.wait_for("是否开启内置语义模型");
    assert!(!tui.shows("需下载"), "连不上软件源的不写大小");
    tui.key(b"\r");
    tui.wait_for("重试");
    assert!(tui.shows("跳过"), "{}", tui.lines().join("\n"));
    tui.key(b"j");
    tui.key(b"\r");
}

#[test]
fn a_home_that_never_walked_the_guide_opens_it_and_one_that_did_does_not() {
    // 核心 8-11 四补：没走过的（`ui.welcomed` 是假）连上就进引导，盖住首页；走过的照常进首页。
    let fresh = Home::unwelcomed(Script::new([]), CONFIGURED);
    let mut tui = fresh.tui("zh_CN.UTF-8");
    // 问到 `ui.welcomed` 以前不画首页（2026-10-10 项目主人报：进引导前闪一下空会话）。
    let start = tui.record_from_here();
    tui.wait_for("按 Enter 开始");
    let flashed = support::frames(start, &tui.recorded())
        .into_iter()
        .find(|f| f.iter().any(|l| l.contains("Tab 切换权限级别")));
    assert!(
        flashed.is_none(),
        "闪过首页：\n{}",
        flashed.unwrap_or_default().join("\n")
    );
    let walked = Home::with_settings(Script::new([]), CONFIGURED);
    let mut tui = walked.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(800));
    assert!(!tui.shows("按 Enter 开始"), "{}", tui.lines().join("\n"));
}

#[test]
fn walking_the_whole_guide_creates_the_persona_and_sets_the_defaults() {
    let home = Home::unwelcomed(Script::new([Play::Says("好。")]), CONFIGURED);
    let mut tui = to_persona(&home);
    // 名字（回车编辑、回车写好）、简单描述（同名字，2026-10-17）、具体人格提示词（回车开大编辑浮窗，Esc 收起）、示范对话
    // （回车开浮窗，a 加一轮）、人设提醒短语（第 7、21a 条）。
    tui.key(b"\r");
    tui.wait_for("Enter 完成");
    tui.type_text("小助手");
    tui.key(b"\r");
    tui.key(b"j");
    tui.key(b"\r");
    tui.type_text("会写诗");
    tui.key(b"\r");
    tui.wait_for("会写诗");
    tui.key(b"j");
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("Esc 完成");
    tui.type_text("你是小助手。");
    tui.key(b"\x1b");
    tui.wait_for("你是小助手。");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.wait_for("暂无示范对话");
    tui.key(b"a");
    tui.wait_for("你问的");
    tui.type_text("在吗");
    tui.key(b"\r");
    tui.type_text("在的");
    tui.key(b"\r");
    tui.wait_for("在的");
    tui.key(b"\x1b");
    tui.wait_for("1 轮");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.wait_for("Esc 完成");
    tui.type_text("说话简短。");
    tui.key(b"\x1b");
    tui.wait_for("说话简短。");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    // 建好、写成默认人格，到了主题（第 22a 条）：没头像的停在现在的配置；明暗那一格回车开列表（2026-10-14 项目主人：
    // 和配置页选值的同一个），挪到哪当场换成哪的底；选好深色，挪到「下一步」写下。
    tui.wait_for("选择明暗和配色");
    assert!(tui.shows("Tokyo Night"), "{}", tui.lines().join("\n"));
    // 两格之间空一行，同别的几步的表单（2026-10-14 项目主人报：挤在一起）。
    let lines = tui.lines();
    // 那一格的行：去掉光标的 `❯` 以后以名字打头（说明那一行里也有「明暗」「配色」）。
    let row = |word: &str| {
        lines.iter().position(|l| {
            let l = l.trim_start();
            let l = l.trim_start_matches('❯').trim_start();
            // 这一步的标题「配色方案」也是「配色」打头，不算（2026-10-21 从「主题」改名）。
            l.starts_with(word) && !l.starts_with("配色方案")
        })
    };
    let (mode, colors) = (row("明暗").unwrap_or(0), row("配色").unwrap_or(0));
    assert_eq!(colors, mode + 2, "{}", lines.join("\n"));
    assert!(lines[mode + 1].trim().is_empty(), "{}", lines.join("\n"));
    // 自动的开着也铺（2026-10-15 项目主人定）：测试终端不答底色，当深色，铺 tokyonight 深色的底。
    let night = vt100::Color::Rgb(0x1a, 0x1b, 0x26);
    assert_eq!(tui.background(0, 0), night, "自动的开着也铺");
    tui.key(b"\r");
    tui.wait_for("浅色");
    tui.key(b"j");
    tui.key(b"j");
    tui.pump(Duration::from_millis(300));
    assert_eq!(
        tui.background(0, 0),
        vt100::Color::Rgb(0xe1, 0xe2, 0xe7),
        "挪到浅色当场换成浅色的底"
    );
    tui.key(b"k");
    tui.pump(Duration::from_millis(300));
    assert_eq!(tui.background(0, 0), night, "挪回深色");
    tui.key(b"\r");
    // 没头像：明暗、配色下面还有铺底色、吉祥物两格，吉祥物停在显示，下面暗色一句（2026-10-15 项目主人定）。
    assert!(
        tui.shows("吉祥物") && tui.shows("显示") && tui.shows("开启会提高 CPU 占用，拖慢启动速度"),
        "{}",
        tui.lines().join("\n")
    );
    assert!(tui.shows("铺底色"), "{}", tui.lines().join("\n"));
    // 铺底色挪到关：当场不铺（2026-10-15 项目主人：主题设置那里的铺底色也缺了）。
    tui.key(b"j");
    tui.key(b"j");
    tui.key(b"\r");
    tui.wait_for("关");
    tui.key(b"j");
    tui.pump(Duration::from_millis(300));
    assert_eq!(tui.background(0, 0), vt100::Color::Default, "关了当场不铺");
    tui.key(b"\r");
    for _ in 0..2 {
        tui.key(b"j");
    }
    tui.key(b"\r");
    through_profile(&mut tui);
    tui.wait_for("选择预设");
    assert!(read(&persona_file(&home, "persona.toml")).contains("小助手"));
    assert!(
        read(&persona_file(&home, "persona.toml")).contains("会写诗"),
        "简单描述写进 summary"
    );
    assert!(read(&persona_file(&home, "prompts/persona.md")).contains("你是小助手。"));
    assert!(read(&persona_file(&home, "prompts/examples.md")).contains("在吗"));
    assert!(read(&persona_file(&home, "prompts/reminders.md")).contains("说话简短。"));
    until(&mut tui, "默认人格", || {
        home.settings().contains("persona-1")
    });
    // 选着现在的默认预设（全部功能），回车。
    tui.wait_for("全部功能");
    tui.key(b"\r");
    through_semantic(&mut tui);
    tui.wait_for("按 Enter 开始对话");
    assert!(
        tui.shows("小助手") && tui.shows("全部功能"),
        "{}",
        tui.lines().join("\n")
    );
    until(&mut tui, "默认预设", || {
        home.settings().contains("[preset]")
    });
    until(&mut tui, "主题写下深色", || {
        home.settings().contains("theme_mode = \"dark\"")
    });
    until(&mut tui, "铺底色写下关", || {
        home.settings().contains("theme_background = false")
    });
    until(
        &mut tui,
        "吉祥物照默认的显示写下首页、侧边栏两项",
        || {
            let settings = home.settings();
            settings.contains("mascot_home = true") && settings.contains("mascot_sidebar = true")
        },
    );
    // 回车走：记下走过了，回到平常的首页。
    tui.key(b"\r");
    tui.wait_for("Tab 切换权限级别");
    assert!(!tui.shows("按 Enter 开始对话"));
    until(&mut tui, "记下走过了", || {
        home.settings().contains("welcomed = true")
    });
    assert!(!tui.shows("没记下"), "{}", tui.lines().join("\n"));
}

#[test]
fn a_name_is_required_and_esc_walks_back_a_step() {
    let home = Home::with_settings(Script::new([]), CONFIGURED);
    let mut tui = to_persona(&home);
    // 什么都没填的是「跳过」（2026-10-19）：填了简单描述、名字空着的才要名字。
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.type_text("会写诗");
    tui.key(b"\r");
    // 简单描述下面：头像、具体人格提示词、示范对话、人设提醒短语，再下面是「下一步」。
    for _ in 0..5 {
        tui.key(b"\x1b[B");
    }
    tui.wait_for("下一步 →");
    tui.key(b"\r");
    tui.wait_for("请填写名字");
    assert!(
        !persona_file(&home, "persona.toml").exists(),
        "名字空着不建"
    );
    tui.key(b"\x1b");
    tui.wait_for("已配置");
    tui.key(b"\x1b");
    tui.wait_for("方块或乱码");
}

#[test]
fn choosing_english_switches_the_guide_and_is_saved() {
    let home = Home::with_settings(Script::new([]), CONFIGURED);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["--page", "oobe"]);
    tui.wait_for("按 Enter 开始");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    // 跟随系统、中文、English、日本語：往下两行。
    tui.key(b"\x1b[B");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.wait_for("How do you like to move around?");
    until(&mut tui, "写进个人设置", || {
        home.settings().contains("language = \"en\"")
    });
}

#[test]
fn choosing_vim_in_the_guide_is_saved_and_the_session_list_takes_j_and_k() {
    // 2026-10-21 项目主人：「新用户默认不开 vimkey。oobe 里可以加一个『你喜欢的操作方式』，方向键和 Vim 两个选项」，放在语言后面。
    let home = Home::with_settings(Script::new([]), CONFIGURED);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["--page", "oobe"]);
    tui.wait_for("按 Enter 开始");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    tui.key(b"\r");
    tui.wait_for("你喜欢的操作方式");
    tui.wait_for("↑↓ 上下，打字直接搜");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.wait_for("方块或乱码");
    until(&mut tui, "写进个人设置", || {
        home.settings().contains("keys = \"vim\"")
    });
    tui.wait_for("j/k 选择");
}

#[test]
fn a_custom_preset_is_created_named_and_made_the_default() {
    let home = Home::with_settings(Script::new([]), CONFIGURED);
    let mut tui = to_persona(&home);
    tui.key(b"\r");
    tui.type_text("小助手");
    tui.key(b"\r");
    // 名称、简单描述、头像、具体人格提示词、示范对话、人设提醒短语，下面是「下一步」。
    for _ in 0..6 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    through_theme(&mut tui);
    through_profile(&mut tui);
    tui.wait_for("选择预设");
    tui.wait_for("全部功能");
    // 自定义在最后：回车开浮窗（光标在名字），回车填名字，关掉第一项功能，到最后一行「创建」回车建好。
    assert!(tui.shows("自选启用的功能"));
    // 出厂预设几个随核心变（P-16 上多了「最少」），往下挪到底就是自定义。
    for _ in 0..8 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    tui.wait_for("输入名称");
    assert!(tui.shows("[*]"));
    tui.key(b"\r");
    tui.type_text("写代码");
    tui.key(b"\r");
    tui.key(b"\x1b[B");
    tui.key(b" ");
    tui.wait_for("[ ]");
    for _ in 0..40 {
        tui.key(b"j");
    }
    // 功能多的往下滚，停上去就露出来。
    tui.wait_for("创建 →");
    tui.key(b"\r");
    through_semantic(&mut tui);
    tui.wait_for("按 Enter 开始对话");
    let file = home.root().join("home/alice/presets/preset-1.toml");
    let text = read(&file);
    assert!(
        text.contains("写代码") && text.contains("= false"),
        "{text}"
    );
    until(&mut tui, "默认预设", || {
        home.settings().contains("preset-1")
    });
}

#[test]
fn the_first_session_after_the_guide_uses_what_it_just_chose_without_asking() {
    // 2026-10-09 项目主人定（照推荐）：刚在引导里选完人格、预设，紧接着的新会话不再弹两个框，直接用刚选的。
    let home = Home::unwelcomed(Script::new([Play::Says("好。")]), CONFIGURED);
    let picks = [("MIYU_TUI_PERSONA", ""), ("MIYU_TUI_PRESET", "")];
    let mut tui = home.tui_wide_with("zh_CN.UTF-8", 140, &picks);
    tui.wait_for("按 Enter 开始");
    tui.key(b"\r");
    tui.wait_for("界面语言");
    tui.key(b"\r");
    tui.wait_for("你喜欢的操作方式");
    tui.key(b"\r");
    tui.wait_for("方块或乱码");
    tui.key(b"\r");
    tui.wait_for("已配置");
    tui.key(b"\r");
    tui.wait_for("创建人格");
    tui.key(b"\r");
    tui.type_text("小助手");
    tui.key(b"\r");
    // 名称、简单描述、头像、具体人格提示词、示范对话、人设提醒短语，下面是「下一步」。
    for _ in 0..6 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    through_theme(&mut tui);
    through_profile(&mut tui);
    tui.wait_for("全部功能");
    // 选常规开发（不是出厂的默认）。
    tui.key(b"\x1b[A");
    tui.key(b"\r");
    through_semantic(&mut tui);
    tui.wait_for("按 Enter 开始对话");
    tui.key(b"\r");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(Duration::from_millis(800));
    assert!(
        !tui.shows("请选择人格") && !tui.shows("请选择预设"),
        "{}",
        tui.lines().join("\n")
    );
    // 说一句：会话照刚选的开，侧边栏写人格、预设。
    tui.type_text("你好");
    tui.key(b"\r");
    tui.wait_for("好。");
    tui.wait_for("人格 小助手");
    tui.wait_for("预设 常规开发");
}

/// 宽屏（窄于这个宽度的引导不画左边那一块）从头走到主题那一步，建一个没头像的人格。
fn to_theme_wide(home: &Home) -> Tui {
    let mut tui = home.tui_wide_with("zh_CN.UTF-8", 140, &[]);
    for (wait, press) in [
        ("按 Enter 开始", b"\r"),
        ("界面语言", b"\r"),
        ("你喜欢的操作方式", b"\r"),
        ("方块或乱码", b"\r"),
        ("已配置", b"\r"),
        ("创建人格", b"\r"),
    ] {
        tui.wait_for(wait);
        tui.key(press);
    }
    tui.type_text("小助手");
    tui.key(b"\r");
    for _ in 0..6 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    tui.wait_for("选择明暗和配色");
    tui
}

#[test]
fn the_lower_rows_open_a_whole_list_even_on_a_crowded_step() {
    // 主题那一步多了主色、铺底色两格（2026-10-15）以后，靠下那几格的选择窗在下面放不下：往上挪到放得下，不截掉选项。
    let home = Home::unwelcomed(Script::new([]), CONFIGURED);
    let mut tui = to_theme_wide(&home);
    // 明暗、配色、铺底色，第四格是吉祥物：开列表，两项都在。
    for _ in 0..3 {
        tui.key(b"j");
    }
    tui.key(b"\r");
    tui.wait_for("不显示");
}

#[test]
fn choosing_no_mascot_on_the_theme_step_walks_it_out_at_once() {
    // 2026-10-15 项目主人：选择不显示吉祥物之后当场就可以把吉祥物关掉。
    let home = Home::unwelcomed(Script::new([]), CONFIGURED);
    let mut tui = to_theme_wide(&home);
    tui.pump(Duration::from_millis(300));
    assert!(tui.shows("@"), "吉祥物在：\n{}", tui.lines().join("\n"));
    for _ in 0..3 {
        tui.key(b"j");
    }
    tui.key(b"\r");
    tui.wait_for("不显示");
    let start = tui.record_from_here();
    tui.key(b"j");
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while tui.shows("@") {
        assert!(
            std::time::Instant::now() < end,
            "吉祥物没走开：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
    // 是一帧帧往左走开的，不是一下没了：录下的帧里它最左边的 `@` 至少到过三个不同的列，往左走（走到边上剩下的那半截
    // 边缘会抖一列，不要求每一帧都更靠左）。
    let lefts: Vec<usize> = support::frames(start, &tui.recorded())
        .iter()
        .filter_map(|f| f.iter().filter_map(|l| l.find('@')).min())
        .collect();
    let mut seen = lefts.clone();
    seen.dedup();
    assert!(seen.len() >= 3, "走了几步：{lefts:?}");
    assert!(seen.first() > seen.last(), "往左走：{lefts:?}");
    // 挪回「显示」：走回来。
    tui.key(b"k");
    tui.wait_for("@");
}
