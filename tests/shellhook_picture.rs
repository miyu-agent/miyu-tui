//! shell 里画她展示的图（排期第 33 条，图纸 `shellhook.md`「怎么印」第 3b 条）：伪终端里跑 `shellhook say`、真核心、假模型
//! 调一次 `show` 展示一张 PNG；终端是 kitty（`KITTY_WINDOW_ID`）时，定下来印进滚动区的那几行里有 kitty 图形协议的图。
#![cfg(unix)]

mod support;

use std::time::{Duration, Instant};

use miyu_session::testkit::{Play, Script};
use support::Home;

#[test]
fn a_shown_picture_is_drawn_with_the_kitty_protocol_in_the_shell() {
    let args = serde_json::json!({"path": "cat.png"}).to_string();
    let home = Home::with_tools(
        Script::new([
            Play::Calls(vec![("show".into(), args)]),
            Play::Says("给你看了。"),
        ]),
        "",
    );
    let mut png = Vec::new();
    image::RgbImage::from_pixel(64, 32, image::Rgb([255, 60, 120]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("编得成");
    std::fs::write(home.work.join("cat.png"), png).expect("写得进去");
    let input = home.root().join("said");
    std::fs::write(&input, "给我看看那张图").expect("写得进去");
    let input = input.display().to_string();
    let mut tui = home.tui_args_with(
        "zh_CN.UTF-8",
        &["shellhook", "say", "--input", &input],
        &[("KITTY_WINDOW_ID", "1")],
    );
    tui.record();
    tui.wait_for("给你看了。");
    let end = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    while Instant::now() < end {
        tui.pump(Duration::from_millis(100));
        bytes.extend(tui.recorded());
        tui.record();
        if bytes.windows(13).any(|w| w == b"\x1b_Ga=T,f=100,") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("\u{1b}_Ga=T,f=100,q=2,C=1,c="),
        "印进滚动区的那几行里有图：{}",
        tui.lines().join("\n")
    );
}
