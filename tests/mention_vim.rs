//! 按键模式是 Vim 时的 `@` 文件列表（发版前 check 16，2026-11-09 项目主人：「列表里能用 j/k 选」，补：`h` `l` 同 `←` `→`，
//! `/` 筛、`Enter` 确认、`Esc` 退出筛选；蓝图 `tui.md`「`@` 文件列表」）。

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::Script;
use support::{Home, Tui};

const ESC: &[u8] = b"\x1b";

fn vim_home() -> Home {
    let home = Home::with_settings(Script::new([]), "[tui]\nkeys = \"vim\"\n");
    for name in ["a1.rs", "a2.rs", "a3.rs"] {
        std::fs::write(home.work.join(name), b"").expect("写测试文件");
    }
    home
}

/// 等输入框里的字变成 `want`。
fn wait_input(tui: &mut Tui, want: &str) {
    let end = Instant::now() + support::WAIT;
    while tui.input() != want {
        assert!(
            Instant::now() < end,
            "输入框不是「{want}」：\n{}",
            tui.lines().join("\n")
        );
        tui.pump(Duration::from_millis(50));
    }
}

/// 单按一下 `Esc`：等一会儿，免得和后面的键连成一串转义序列。
fn esc(tui: &mut Tui) {
    tui.key(ESC);
    tui.pump(Duration::from_millis(200));
}

#[test]
fn esc_picks_in_the_list_with_j_and_k_and_the_typed_word_stays() {
    let home = vim_home();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.type_text("@a");
    tui.wait_for("a3.rs");
    tui.wait_for("Esc 在列表里选");
    esc(&mut tui);
    tui.wait_for("j/k 选择");
    assert!(
        tui.shows("a1.rs"),
        "Esc 不关列表：\n{}",
        tui.lines().join("\n")
    );
    // 列出来的先后：照屏上的。
    let lines = tui.lines();
    let at = |name: &str| {
        lines
            .iter()
            .position(|l| l.contains(name) && !l.contains('❯'))
    };
    let mut order = ["a1.rs", "a2.rs", "a3.rs"];
    order.sort_by_key(|n| at(n));
    // h、l、退格都不进输入框（h、l 挪光标，词跟着变、选中回第一条，所以先按）；再 j、j、k：停在第二条。
    for key in [b"h", b"l", b"\x7f", b"j", b"j", b"k"] {
        tui.key(key);
        tui.pump(Duration::from_millis(100));
    }
    assert_eq!(tui.input(), "@a", "字不进输入框");
    tui.key(b"\r");
    let end = Instant::now() + support::WAIT;
    while !tui.input().contains(order[1]) {
        assert!(
            Instant::now() < end,
            "收成块的是第二条 {}：{}",
            order[1],
            tui.input()
        );
        tui.pump(Duration::from_millis(50));
    }
}

#[test]
fn a_slash_filters_enter_keeps_it_and_esc_takes_it_back() {
    let home = vim_home();
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("开放权限");
    tui.type_text("@a");
    tui.wait_for("a3.rs");
    esc(&mut tui);
    tui.wait_for("j/k 选择");
    tui.type_text("/");
    tui.wait_for("Esc 取消筛选");
    tui.type_text("3");
    wait_input(&mut tui, "@a3");
    tui.key(b"\r");
    tui.wait_for("j/k 选择");
    assert_eq!(tui.input(), "@a3", "Enter 确认筛的，不收成块");
    // 再筛一次又不要了：Esc 换回按 `/` 以前的词，回去选。
    tui.type_text("/");
    tui.wait_for("Esc 取消筛选");
    tui.type_text("x");
    wait_input(&mut tui, "@a3x");
    esc(&mut tui);
    wait_input(&mut tui, "@a3");
    tui.wait_for("j/k 选择");
    tui.key(b"\r");
    let end = Instant::now() + support::WAIT;
    while !tui.input().contains("a3.rs") {
        assert!(Instant::now() < end, "收成块：{}", tui.input());
        tui.pump(Duration::from_millis(50));
    }
}
