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
    let dir = support::data_of(root).packages().join("weather");
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
        home.data().packages().join("weather").exists(),
        "问的时候还没卸"
    );
    // 排期第 42 条（2026-11-04 项目主人）：问的那几行画在弹窗里（圆角框，「继续？」左右是框线），列表照常不收起（别的包还看得见）。
    let lines = tui.lines();
    let ask = lines.iter().position(|l| l.contains("继续？")).expect("在");
    assert!(
        lines[ask].contains("│ 继续？") && lines[ask].trim_end().ends_with('│'),
        "弹窗：\n{}",
        lines.join("\n")
    );
    assert!(
        lines[..ask].iter().any(|l| l.contains("╭──")),
        "弹窗上边框：\n{}",
        lines.join("\n")
    );
    assert!(
        lines.iter().any(|l| l.contains("人格记忆")),
        "列表照常：\n{}",
        lines.join("\n")
    );
    tui.key(b"y");
    tui.wait_for("已卸载 Weather");
    assert!(!home.data().packages().join("weather").exists(), "卸了");
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

/// 在数据根里放一个只带设置的包 `drawtest`：一项滑块（1～50，一步 5，默认 20）。
fn drawtest(root: &Path) {
    let dir = support::data_of(root).packages().join("drawtest");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nversion = \"0.1.0\"\nname = { zh = \"画图测试\" }\n\n[settings.steps]\ntype = \"int\"\nslider = true\nmin = 1\nmax = 50\nstep = 5\ndefault = 21\nname = { zh = \"步数\" }\n",
    )
    .expect("写得进去");
}

/// 滑块（「配置页」第 33 条子规则，2026-10-31 项目主人定）：那一行 `→` 直接调，不按别的键，停手半秒自己存进系统配置。
#[test]
fn a_slider_saves_itself_half_a_second_after_the_last_arrow() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", drawtest);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    enter(&mut tui, "画图测试");
    tui.wait_for("步数");
    // 核心照包的开关在最上面多一项（「把这个包关掉」），滑块在下面：挪到它上面。
    reach(&mut tui, "步数");
    tui.wait_for("←/→ 调");
    tui.key(b"\x1b[C");
    until(&mut tui, "那一行写 26", |t| {
        row(t, "步数").trim_end().ends_with("26")
    });
    until(&mut tui, "停手以后存进系统配置", |_| {
        home.settings().contains("steps = 26")
    });
}

/// 在数据根里放一个带记录列表的包 `acct`：账号，每一条一个名字（当标题）、一个 Key（密钥）。
fn accounts(root: &Path) {
    let dir = support::data_of(root).packages().join("acct");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let manifest = "[package]\nversion = \"0.1.0\"\nname = { zh = \"账号测试\" }\n\n\
        [settings.accounts]\ntype = \"list\"\nelement = \"record\"\ntitle = \"name\"\nname = { zh = \"账号\" }\n\n\
        [settings.accounts.fields.name]\ntype = \"text\"\nname = { zh = \"名字\" }\n\n\
        [settings.accounts.fields.key]\ntype = \"secret\"\nname = { zh = \"Key\" }\n";
    std::fs::write(dir.join("package.toml"), manifest).expect("写得进去");
}

/// 记录列表（「配置页」第 47b 条，核心 PC-2）：加一条、填名字和密钥，收起列表窗时整份存；密钥先存进密钥库、配置里写引用，
/// 不写明文（真的核心）。
#[test]
fn a_record_with_a_secret_is_saved_as_a_reference() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", accounts);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    enter(&mut tui, "账号测试");
    tui.wait_for("账号");
    enter(&mut tui, "账号");
    tui.wait_for("还没有");
    tui.key(b"a");
    tui.wait_for("账号 › 第 1 项");
    tui.key(b"\r");
    for c in ["备", "用"] {
        tui.key(c.as_bytes());
    }
    tui.key(b"\r");
    tui.wait_for("账号 › 备用");
    tui.key(b"\x1b[B");
    tui.key(b"\r");
    tui.key(b"sk-test-1");
    tui.key(b"\r");
    until(&mut tui, "Key 写已设置", |t| {
        t.lines()
            .iter()
            .any(|l| l.contains("Key") && l.contains("已设置"))
    });
    tui.key(b"\x1b");
    tui.wait_for("J/K 移动");
    tui.key(b"\x1b");
    until(&mut tui, "存进系统配置", |_| {
        home.settings().contains("备用")
    });
    let settings = home.settings();
    assert!(settings.contains("secret"), "写成引用：\n{settings}");
    assert!(!settings.contains("sk-test-1"), "不写明文：\n{settings}");
    until(&mut tui, "页里那一行写标题", |t| {
        row(t, "账号").contains("备用")
    });
}

/// 排期第 39 条（2026-11-04 项目主人：详情页太高、和列表比例不好）：预览块只有内容那么高，最后一行内容下面空一行，再往下
/// 就不是预览块的底色了（原来一直铺到状态行上面）。
#[test]
fn the_preview_is_only_as_tall_as_what_it_says() {
    let home = Home::prepared(Script::new([Play::Says("好。")]), "", weather);
    let mut tui = home.tui_args("zh_CN.UTF-8", &["config"]);
    installed(&mut tui);
    reach(&mut tui, "Weather");
    tui.wait_for("技能 notes");
    tui.pump(Duration::from_millis(200));
    let lines = tui.lines();
    let first = lines
        .iter()
        .position(|l| l.contains("名字"))
        .expect("有预览");
    let last = lines
        .iter()
        .position(|l| l.contains("技能 notes"))
        .expect("有包含那一行");
    // 预览块的左边：字往左两格。
    let x = lines[first].chars().take_while(|c| *c == ' ').count() - 2;
    let (x, first, last) = (
        u16::try_from(x).unwrap(),
        u16::try_from(first).unwrap(),
        u16::try_from(last).unwrap(),
    );
    let panel = tui.background(first, x);
    assert_eq!(tui.background(last + 1, x), panel, "最后一行下面留一行白");
    assert_ne!(
        tui.background(last + 2, x),
        panel,
        "再往下不是预览块：\n{}",
        lines.join("\n")
    );
}
