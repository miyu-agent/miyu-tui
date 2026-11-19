//! 引导的主题那一步（从 `oobe.rs` 拆出来，那边到了 500 行）：等这一步能挪了再按键、宽屏走到这一步，和只在这一步上的两条测试。

use std::time::Duration;

use miyu_session::testkit::Script;

use super::CONFIGURED;
use super::support::{self, Home, Tui};

/// 光标 `❯` 停在 `word` 那一格上（宽屏时左边画着形象，`❯` 不在行首）。
pub(super) fn picked(lines: &[String], word: &str) -> bool {
    lines.iter().any(|l| {
        l.split('❯')
            .skip(1)
            .any(|rest| rest.trim_start().starts_with(word))
    })
}

/// 等主题那一步能挪了，交回那时的各行：进这一步先读配置，读回来以前按的键都被丢掉、哪一格都不画光标（`src/oobe/theme.rs` 的
/// `busy`；2026-11-19 本机门禁慢的时候丢了一下 `j`，照按四下数停在了「形象」、开了它的列表）。等光标画在「明暗」上再按。
pub(super) fn theme_ready(tui: &mut Tui) -> Vec<String> {
    tui.wait_until("主题那一步能挪（光标在「明暗」上）", |l| {
        picked(l, "明暗")
    })
}

/// 宽屏（窄于这个宽度的引导不画左边那一块）从头走到主题那一步，建一个没头像的人格。
pub(super) fn to_theme_wide(home: &Home) -> Tui {
    let mut tui = home.tui_wide_with("zh_CN.UTF-8", 140, &[]);
    for (wait, press) in [
        ("按 Enter 开始", b"\r"),
        ("界面语言", b"\r"),
        ("你喜欢的操作方式", b"\r"),
        ("方块或乱码", b"\r"),
        ("已配置", b"\r"),
        ("希望金额以什么货币显示", b"\r"),
        ("创建人格", b"\r"),
    ] {
        tui.wait_for(wait);
        tui.key(press);
    }
    tui.type_text("小助手");
    tui.key(b"\r");
    for _ in 0..7 {
        tui.key(b"\x1b[B");
    }
    tui.key(b"\r");
    theme_ready(&mut tui);
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
