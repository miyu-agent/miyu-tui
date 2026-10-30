//! 伪终端里的端到端测试：配置页「软件包」接真的核心（蓝图 `tui.md`「配置页」第 43a、43c 条）。

mod support;

use std::path::Path;
use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 往下挪到选中的那一行（行首 `┃`）是 `to`：每按一下看一眼，慢的机器上按键早到也不会多挪、少挪。
fn reach(tui: &mut Tui, to: &str) {
    for _ in 0..40 {
        if tui
            .lines()
            .iter()
            .any(|l| l.contains('┃') && l.contains(to))
        {
            return;
        }
        tui.key(b"\x1b[B");
    }
    panic!("挪不到「{to}」：\n{}", tui.lines().join("\n"));
}

/// 挪到 `to`、回车。
fn enter(tui: &mut Tui, to: &str) {
    reach(tui, to);
    tui.key(b"\r");
}

/// 等到 `ok` 成立。
fn until(tui: &mut Tui, what: &str, ok: impl Fn(&Tui) -> bool) {
    let end = Instant::now() + support::WAIT;
    while !ok(tui) {
        assert!(
            Instant::now() < end,
            "等不到{what}：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

/// 进「软件包」的「已安装」：主菜单挪到「软件包」进去（停在「下载软件」），换到右边一页。
fn installed(tui: &mut Tui) {
    tui.wait_for("运行日志");
    enter(tui, "软件包");
    tui.wait_for("下载软件");
    tui.key(b".");
    tui.wait_for("全部升级");
}

/// 在数据根里放一个只带一个技能的包 `weather`（核心起来时读、补记进本地库）。
fn weather(root: &Path) {
    let dir = root.join("packages/weather");
    let skill = dir.join("skills/notes");
    std::fs::create_dir_all(&skill).expect("建得了目录");
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nversion = \"1.2.0\"\nname = { en = \"Weather\" }\n",
    )
    .expect("写得进去");
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: notes\ndescription: Take notes in a fixed format.\n---\nWrite notes.\n",
    )
    .expect("写得进去");
}

/// 选中的那一行里有 `name` 的。
fn row(tui: &Tui, name: &str) -> String {
    tui.lines()
        .into_iter()
        .find(|l| l.contains('┃') && l.contains(name))
        .unwrap_or_default()
}

/// 联网搜索的 key、地址那一组归联网搜索（核心 F-14，2026-10-20 项目主人验收报）：从「已安装」进联网搜索那一页，有
/// 「API Key 和地址」这一组；`Esc` 回「已安装」、停在它上面（第 43c 条）。
#[test]
fn the_web_search_keys_live_on_the_web_search_page() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    enter(&mut tui, "联网搜索");
    tui.wait_for("搜索服务供应商");
    tui.wait_for("API Key 和地址");
    tui.key(b"\x1b");
    tui.wait_for("全部升级");
    until(&mut tui, "停回联网搜索", |t| {
        !row(t, "联网搜索").is_empty()
    });
}

/// `d` 先列出要卸的、问一句，`y` 才卸：包目录没了，列表里也没了（第 43c 条）。
#[test]
fn removing_a_package_asks_first_and_takes_it_off_the_list() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", weather);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    reach(&mut tui, "Weather");
    tui.key(b"d");
    tui.wait_for("要卸载 Weather 1.2.0");
    tui.wait_for("继续？");
    assert!(
        home.root().join("packages/weather").exists(),
        "问的时候还没卸"
    );
    tui.key(b"y");
    tui.wait_for("已卸载 Weather");
    assert!(!home.root().join("packages/weather").exists(), "卸了");
    until(&mut tui, "列表里没了 Weather", |t| {
        !t.lines().iter().any(|l| l.contains("Weather  "))
    });
}

/// 空格开关：出厂的内置包关了就是卸掉（核心定），写「关」；再按一下装回来（第 43c 条）。
#[test]
fn space_turns_a_shipped_package_off_and_on() {
    let home = Home::new(Script::new([Play::Says("好。")]));
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    reach(&mut tui, "联网搜索");
    tui.key(b" ");
    tui.wait_for("联网搜索 已关闭");
    until(&mut tui, "写「关」", |t| {
        row(t, "联网搜索").trim_end().ends_with('关')
    });
    tui.key(b" ");
    tui.wait_for("联网搜索 已打开");
    until(&mut tui, "不写「关」", |t| {
        !row(t, "联网搜索").trim_end().ends_with('关')
    });
}
