//! 附件照模型收（蓝图 `tui.md`「输入框」第 12 条「照模型收」）：模型收不了图片的，`Ctrl+V` 贴进来的图收成文件块、提示一句，
//! 发出去写路径；收得了的照旧是附件。终端送来的粘贴（`Ctrl+Shift+V`、拖文件）照原样是字（2026-10-10 项目主人）。
//! 剪贴板用工作目录里的假 `wl-paste`（放在 `PATH` 前面），不碰跑测试那台机器上的剪贴板；`wl-paste` 只有 Linux 上读，
//! 按 `Ctrl+V` 的那两条只在 Linux 上跑（macOS 读剪贴板走 `osascript`，Windows 另有一套）。

mod support;

use miyu_session::testkit::{Play, Script};
use support::Home;
use support::clipboard::picture;
#[cfg(target_os = "linux")]
use support::clipboard::with_clipboard;

/// 一家、一个模型，配置里写明它只收文字。
#[cfg(target_os = "linux")]
const BLIND: &str = "[providers.fake]\ndriver = \"openai-chat\"\nbase_url = \"http://127.0.0.1:9/v1\"\n\n[providers.fake.models.m]\ninputs = [\"text\"]\n\n[models]\nchat = \"fake/m\"\n";
/// 同一个模型，收图。
const SEEING: &str = "[providers.fake]\ndriver = \"openai-chat\"\nbase_url = \"http://127.0.0.1:9/v1\"\n\n[providers.fake.models.m]\ninputs = [\"text\", \"image\"]\n\n[models]\nchat = \"fake/m\"\n";

/// 把路径当一次粘贴送进去（终端拖文件进来、`Ctrl+Shift+V` 都是这样）。
fn drop_in(tui: &mut support::Tui, path: &str) {
    tui.key(format!("\x1b[200~{path}\x1b[201~").as_bytes());
}

#[test]
#[cfg(target_os = "linux")]
fn a_model_that_cannot_see_images_gets_the_path_instead() {
    let home = Home::with_settings(Script::new([Play::Says("好。")]), BLIND);
    let path = picture(&home);
    let mut tui = with_clipboard(&home, &path);
    tui.wait_for("Tab 切换权限级别");
    tui.pump(std::time::Duration::from_millis(600));
    tui.key(b"\x16");
    tui.wait_for("当前模型不支持图片");
    assert!(
        tui.shows("[晚霞.png]") && !tui.shows("[图片 1]"),
        "{}",
        tui.lines().join("\n")
    );
}

#[test]
#[cfg(target_os = "linux")]
fn a_model_that_sees_images_still_gets_an_attachment() {
    let home = Home::with_settings(Script::new([Play::Says("好。")]), SEEING);
    let path = picture(&home);
    let mut tui = with_clipboard(&home, &path);
    tui.wait_for("Tab 切换权限级别");
    tui.pump(std::time::Duration::from_millis(600));
    tui.key(b"\x16");
    tui.wait_for("[图片 1]");
    assert!(!tui.shows("当前模型不支持"));
}

#[test]
fn a_dropped_or_terminal_pasted_path_stays_text() {
    // 2026-10-10 项目主人：「ctrl+shift+V 粘贴文字，ctrl+V 粘贴占位符」。拖文件进来和 Ctrl+Shift+V 哪个终端都是一次粘贴，
    // 分不开，照原样是字。
    let home = Home::with_settings(Script::new([Play::Says("好。")]), SEEING);
    let path = picture(&home);
    let mut tui = home.tui("zh_CN.UTF-8");
    tui.wait_for("Tab 切换权限级别");
    tui.pump(std::time::Duration::from_millis(600));
    drop_in(&mut tui, &path);
    tui.wait_for("晚霞.png");
    assert!(!tui.shows("[图片 1]"), "{}", tui.lines().join("\n"));
}
