//! 解壁纸图取色放在另起的短命进程里（蓝图「主题」的「跟随壁纸」第 6 条子规则，2026-11-02）：程序带隐藏的
//! `--wallpaper-seeds`，从标准输入读图、一行一个 `#rrggbb` 印出候选就退，不进界面。

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn the_hidden_flag_reads_a_picture_and_prints_its_main_colors() {
    let mut png = Vec::new();
    image::RgbImage::from_pixel(64, 64, image::Rgb([40, 170, 60]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("编得出图");
    let mut child = Command::new(env!("CARGO_BIN_EXE_miyu-tui"))
        .arg("--wallpaper-seeds")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("起得来");
    child.stdin.take().unwrap().write_all(&png).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines.is_empty(), "印了候选");
    assert!(
        lines.iter().all(|l| l.len() == 7 && l.starts_with('#')),
        "一行一个 #rrggbb：{text}"
    );
}
