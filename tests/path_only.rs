//! 伪终端里的端到端测试：超过上传上限的附件只把路径交给她（蓝图 `tui.md`「输入框」第 12 条，核心 AT-1；2026-11-11 项目
//! 主人定：上限不动，超限的附件照常显示、实际只给路径，不再整句发不出去）。剪贴板用假的 `wl-paste`，只在 Linux 上跑。

mod support;

#[cfg(target_os = "linux")]
use miyu_session::testkit::{Play, Script};
#[cfg(target_os = "linux")]
use support::Home;

/// 一家、一个模型，收图。
#[cfg(target_os = "linux")]
const SEEING: &str = "[providers.fake]\ndriver = \"openai-chat\"\nbase_url = \"http://127.0.0.1:9/v1\"\n\n[providers.fake.models.m]\ninputs = [\"text\", \"image\"]\n\n[models]\nchat = \"fake/m\"\n";

/// 超过 20 MiB 的字节数。
#[cfg(target_os = "linux")]
const BIG: u64 = 21 * 1024 * 1024;

#[test]
#[cfg(target_os = "linux")]
fn a_file_over_the_upload_limit_goes_out_as_its_path() {
    use std::io::Write;
    let home = Home::with_settings(Script::new([Play::Says("收到路径了。")]), SEEING);
    // 一张 21 MiB 的「图」：只写开头几个字节，后面是空洞，不真占 21 MiB。
    let path = home.work.join("晚霞.png");
    let mut file = std::fs::File::create(&path).expect("建得了");
    file.write_all(b"\x89PNG\r\n\x1a\n").expect("写得进去");
    file.set_len(BIG).expect("撑得大");
    drop(file);
    let path = path.display().to_string();
    let mut tui = support::clipboard::with_clipboard(&home, &path);
    tui.wait_for("Tab 切换权限级别");
    tui.pump(std::time::Duration::from_millis(600));
    tui.key(b"\x16");
    tui.wait_for("[图片 1]");
    tui.say(" 看看这张");
    // 原来这里是整句不发、正文写核心的原话（太大）；现在照常发出去，她答了。
    tui.wait_for("收到路径了。");
    tui.wait_for("[图片 1] 仅路径");
    let message = support::clipboard::logged_message(home.root(), "看看这张").expect("记着这一句");
    let blocks = message["body"]["blocks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let item = blocks
        .iter()
        .find(|b| b["type"] == "path")
        .unwrap_or_else(|| panic!("记成只有路径的一块：{message}"));
    assert_eq!(item["path"], path);
    assert_eq!(item["name"], "晚霞.png");
    assert_eq!(item["size"], BIG);
    assert!(
        blocks.iter().all(|b| b.get("blob").is_none()),
        "没有传内容：{message}"
    );
}
