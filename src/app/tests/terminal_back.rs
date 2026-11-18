//! 挂起、让出终端给编辑器回来以后，拖滚动条的像素坐标照常重新打开（审计 2026-11-18 #067）。

use super::app;

/// 抓着滚动条按 `Ctrl+Z`（或点开文件让出终端给编辑器），走的时候 `main.rs` 的 `leave` 关了终端的像素坐标，这边原来还记着
/// 开着：回来以后不再写打开，终端照格子报、这边照像素换，拖起来乱跳。回来以后记成关着，再拖照常重新打开。
#[test]
fn coming_back_to_the_terminal_turns_pixels_back_on_for_the_next_drag() {
    let (mut app, _sent) = app();
    app.screen.pixel.cell = Some((10, 20));
    let synced = |app: &mut crate::app::App| {
        let mut out = Vec::new();
        app.screen.pixel.sync(&mut out).expect("写得进");
        String::from_utf8(out).expect("是字")
    };
    // 抓着滚动条：打开。
    app.screen.pixel.want = true;
    assert_eq!(synced(&mut app), "\x1b[?1016h");
    // 挂起回来：终端那边已经关了，不再写关。
    app.terminal_back();
    assert_eq!(synced(&mut app), "", "终端那边已经关了，不再写");
    assert_eq!(app.screen.pixel.cell, Some((10, 20)), "一格多大照旧");
    // 接着拖（`mouse` 照抓没抓着滚动条再设 `want`）：照常重新打开。
    app.screen.pixel.want = true;
    assert_eq!(synced(&mut app), "\x1b[?1016h", "再拖照常打开");
}
