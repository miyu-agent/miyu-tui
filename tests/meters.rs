//! 量尺（蓝图 `tui.md`「量尺」；设计 23 第二节的预算，2026-10-15 项目主人定进 CI、超预算就拦、不打倍数）：空闲时唤醒几次、
//! 空闲的内存、打开长会话的内存峰值、热启动到首页输入框多久。只在 Linux 上量（`/proc`），要照发出去的样子量：CI 单独一步
//! 用 release 编、一个一个跑（`cargo test --release --test meters -- --ignored --test-threads=1`）。

#![cfg(target_os = "linux")]

mod support;

use std::time::Duration;

use miyu_session::testkit::{Play, Script};
use support::{Home, Tui};

/// 一 MiB。
const MIB: u64 = 1024 * 1024;
/// 终端界面自己载着一个大会话、空闲时，PSS 最多多少（设计 23：「终端界面自己，载着一个大会话 20 MB」）。
const PSS_BUDGET: u64 = 20 * MIB;
/// 打开长会话时的峰值比稳下来以后最多多多少（2026-10-15 项目主人定）。
const PEAK_OVER_STEADY: u64 = 10 * MIB;
/// 热启动（核心已经在跑）到首页输入框出来，p95 最多多久（设计 23 F5：人整段等着的，上限 50 ms）。
const FIRST_FRAME_P95: Duration = Duration::from_millis(50);
/// 吉祥物醒着的那一阵、哈欠、转回正前方都过去要多久（`mascot.json` 的 `idle.awake_ms` 10 秒，加上哈欠和转头）。
const SETTLE: Duration = Duration::from_secs(13);
/// 停住以后量多久。
const IDLE: Duration = Duration::from_secs(5);
/// 长会话几轮、每句带多少字（同核心那边的对比：50 轮，每句约 1.5 KB 资料，回答一句话）。
const TURNS: usize = 50;
const NOTE_BYTES: usize = 1500;

/// `/proc/<进程>/<文件>` 里某一行的 kB 数，换成字节。
fn kb(pid: u32, file: &str, field: &str) -> u64 {
    let text = std::fs::read_to_string(format!("/proc/{pid}/{file}")).expect("读得到 /proc");
    let line = text
        .lines()
        .find(|l| l.starts_with(field))
        .unwrap_or_else(|| panic!("{file} 里没有 {field}"));
    let n: u64 = line
        .split_whitespace()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .expect("是数");
    n * 1024
}

fn pss(pid: u32) -> u64 {
    kb(pid, "smaps_rollup", "Pss:")
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / MIB as f64
}

#[test]
#[ignore = "量尺：CI 单独一步用 release 编、一个一个跑"]
fn idle_on_the_home_screen_wakes_nothing_and_stays_small() {
    let home = Home::new(Script::new([]));
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(SETTLE);
    let pid = tui.pid().expect("有进程号");
    let before = tui.wakeups().expect("Linux 上数得到");
    tui.record();
    tui.pump(IDLE);
    let written = tui.recorded().len();
    let woke = tui.wakeups().expect("Linux 上数得到") - before;
    let pss = pss(pid);
    println!(
        "空闲 {IDLE:?}：唤醒 {woke} 次，写 {written} 字节，PSS {:.1} MiB",
        mib(pss)
    );
    assert_eq!(woke, 0, "空闲时唤醒");
    assert_eq!(written, 0, "空闲时往终端写");
    assert!(pss <= PSS_BUDGET, "空闲 PSS {:.1} MiB 超过预算", mib(pss));
}

/// 说 `TURNS` 轮，每句带 `NOTE_BYTES` 字的资料（照粘贴发）；交回会话编号。
fn long_session(home: &Home) -> String {
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    let note: String = "资料".repeat(NOTE_BYTES / 6);
    for i in 1..=TURNS {
        tui.send(format!("\x1b[200~第 {i} 条：{note}\x1b[201~").as_bytes());
        tui.pump(Duration::from_millis(30));
        tui.send(b"\r");
        tui.wait_for(&format!("第 {i} 条记下了。"));
    }
    drop(tui);
    let sessions = home.root().join("home/alice/sessions");
    let mut ids: Vec<String> = std::fs::read_dir(&sessions)
        .expect("有会话目录")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(ids.len(), 1, "只开了一个会话：{ids:?}");
    ids.remove(0)
}

/// 一张假的桌面壁纸：waypaper 的设置（`$XDG_CONFIG_HOME/waypaper/config.ini`，有没有 Wayland 都认）指一张 4K 的图（解开
/// 32 MiB），别的壁纸程序找不到（`XDG_RUNTIME_DIR` 指空目录、不认桌面）。交回给界面的环境变量（看桌面的开关打开）。
fn desktop(home: &Home) -> Vec<(&'static str, String)> {
    let dir = home.work.join("desktop");
    std::fs::create_dir_all(dir.join("run")).expect("建得了");
    std::fs::create_dir_all(dir.join(".config/waypaper")).expect("建得了");
    let picture = dir.join("wall.png");
    image::RgbImage::from_fn(3840, 2160, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 96])
    })
    .save(&picture)
    .expect("存得了图");
    let ini = format!("[Settings]\nwallpaper = {}\n", picture.display());
    std::fs::write(dir.join(".config/waypaper/config.ini"), ini).expect("写得了");
    let at = |sub: &str| dir.join(sub).display().to_string();
    vec![
        ("HOME", at("")),
        ("XDG_CONFIG_HOME", at(".config")),
        ("XDG_RUNTIME_DIR", at("run")),
        ("XDG_CURRENT_DESKTOP", String::new()),
        ("MIYU_TUI_DESKTOP_WALLPAPER", "1".into()),
    ]
}

/// `--resume` 打开会话 `id`，正文出来、稳下来以后交回（峰值、稳下来的 RSS、PSS）。
fn open(home: &Home, id: &str, env: &[(&str, String)]) -> (u64, u64, u64) {
    let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut tui = home.tui_args_with("zh_CN.UTF-8", &["--resume", id], &env);
    tui.wait_for(&format!("第 {TURNS} 条记下了。"));
    tui.pump(Duration::from_secs(3));
    let pid = tui.pid().expect("有进程号");
    (
        kb(pid, "status", "VmHWM:"),
        kb(pid, "status", "VmRSS:"),
        pss(pid),
    )
}

/// 峰值不比稳下来多太多，PSS 不超预算。
fn assert_flat(what: &str, (peak, steady, pss): (u64, u64, u64)) {
    println!(
        "{what}：峰值 {:.1} MiB，稳下来 {:.1} MiB，PSS {:.1} MiB",
        mib(peak),
        mib(steady),
        mib(pss)
    );
    assert!(
        peak.saturating_sub(steady) <= PEAK_OVER_STEADY,
        "{what}：峰值比稳下来多了 {:.1} MiB",
        mib(peak.saturating_sub(steady))
    );
    assert!(
        pss <= PSS_BUDGET,
        "{what}：PSS {:.1} MiB 超过预算",
        mib(pss)
    );
}

#[test]
#[ignore = "量尺：CI 单独一步用 release 编、一个一个跑"]
fn opening_a_long_session_has_no_memory_spike() {
    let plays: Vec<Play> = (1..=TURNS)
        .map(|i| Play::Says(Box::leak(format!("第 {i} 条记下了。").into_boxed_str())))
        .collect();
    let home = Home::new(Script::new(plays));
    let id = long_session(&home);
    // 桌面上有一张 4K 的壁纸：没跟随壁纸的不解它（2026-10-15 查出原来起来就解，峰值冲到 40 MiB）。
    let env = desktop(&home);
    assert_flat("打开长会话", open(&home, &id, &env));
    // 跟随壁纸：头一次解一次、记进缓存；以后同一张不再解。
    let settings = home.root().join("home/alice/settings.toml");
    let text = std::fs::read_to_string(&settings).expect("有个人设置");
    std::fs::write(&settings, format!("{text}\n[tui]\ntheme = \"wallpaper\"\n")).expect("写得了");
    // 核心起来时读个人设置：停掉再起来，读新写的。
    home.stop_core();
    home.restart_core();
    let (peak, steady, _) = open(&home, &id, &env);
    println!(
        "跟随壁纸、头一次（解图）：峰值 {:.1} MiB，稳下来 {:.1} MiB",
        mib(peak),
        mib(steady)
    );
    assert!(
        home.root().join("cache/miyu/tui/wallpaper.json").is_file(),
        "解过的记进了缓存"
    );
    assert_flat("跟随壁纸、记过的", open(&home, &id, &env));
}

#[test]
#[ignore = "量尺：CI 单独一步用 release 编、一个一个跑"]
fn a_warm_start_shows_the_input_box_quickly() {
    let home = Home::new(Script::new([]));
    // 先起一次：核心那头头一回连上要做的（读模型目录这些）不算进来，量的是核心已经在跑的热启动。
    first(&home);
    let mut took: Vec<Duration> = (0..20).map(|_| first(&home)).collect();
    took.sort();
    let p95 = took[took.len() * 95 / 100 - 1];
    println!(
        "热启动到首页输入框：中位数 {:?}，p95 {p95:?}",
        took[took.len() / 2]
    );
    assert!(p95 <= FIRST_FRAME_P95, "p95 {p95:?} 超过预算：{took:?}");
}

/// 起一个界面，交回到首页输入框出来用了多久。
fn first(home: &Home) -> Duration {
    let mut tui: Tui = home.tui("zh_CN.UTF-8");
    let at = tui.wait_for_at("Tab 切换权限级别");
    at.duration_since(tui.started)
}
