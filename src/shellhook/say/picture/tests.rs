//! shell 里画她展示的图（排期第 33 条）：kitty 才画，排成记号那一行加垫着的空行；活动区写标题，定下来换成 kitty 图形协议。

use std::path::PathBuf;

use ratatui::text::Line;

use super::Pictures;

/// 测试用的临时目录，测试完（连断言失败）删掉：`/tmp` 是内存盘，原来每跑一遍留一个（2026-11-13 审计）。
struct Gone(PathBuf);

impl Drop for Gone {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

/// 画一张 `w`×`h` 的纯色 PNG 存在这一张自己的临时目录里（文件名不带扩展名：读回来存下的就是这样）；目录交给 [`Gone`] 删。
fn png(name: &str, w: u32, h: u32) -> (PathBuf, Gone) {
    let dir = std::env::temp_dir().join(format!("miyu-picture-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut bytes = Vec::new();
    image::RgbImage::from_pixel(w, h, image::Rgb([255, 60, 120]))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    std::fs::write(&path, bytes).unwrap();
    (path, Gone(dir))
}

fn kitty() -> Pictures {
    Pictures::detect(
        |name| (name == "TERM").then(|| "xterm-kitty".into()),
        Some((8, 16)),
    )
}

fn text(line: &Line) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[test]
fn only_kitty_draws() {
    let (path, _gone) = png("plain", 64, 32);
    let mut none = Pictures::detect(|_| None, Some((8, 16)));
    assert!(
        none.lines(("b1", &path), (2, 80, 20), "[图片] 猫")
            .is_none()
    );
    let mut herdr = Pictures::detect(|name| (name == "KITTY_WINDOW_ID").then(|| "3".into()), None);
    assert!(
        herdr
            .lines(("b1", &path), (2, 80, 20), "[图片] 猫")
            .is_some(),
        "herdr 里带着外面 kitty 的"
    );
}

#[test]
fn a_picture_reserves_its_rows_shows_its_title_live_and_draws_when_settled() {
    // 64×32 像素，一格 8×16：8 列 2 行。
    let (path, _gone) = png("cat", 64, 32);
    let mut pictures = kitty();
    let lines = pictures
        .lines(("cat", &path), (2, 80, 20), "[图片] 猫")
        .expect("画得了");
    assert_eq!(lines.len(), 2, "记号那一行加垫着的一行");
    let live = pictures.live(&lines);
    assert_eq!(text(&live[0]), "  [图片] 猫", "活动区写标题");
    let mut strings = vec![String::from("x"), String::new()];
    pictures.settle(&lines, &mut strings);
    assert!(
        strings[0].starts_with("  \u{1b}_Ga=T,f=100,q=2,C=1,c=8,r=2,m=0;"),
        "{:?}",
        &strings[0][..40]
    );
    assert!(strings[0].ends_with("\u{1b}\\"));
    assert_eq!(strings[1], "", "垫着的那一行照旧");
    // 不是记号的行不动。
    let mut other = vec![String::from("字")];
    pictures.settle(&[Line::raw("字")], &mut other);
    assert_eq!(other, ["字"]);
}

#[test]
fn tall_pictures_are_capped_and_big_ones_go_in_chunks() {
    // 很高的：最多 `max_rows` 行，列跟着缩。
    let (tall, _gone) = png("tall", 40, 800);
    let mut pictures = kitty();
    let lines = pictures.lines(("tall", &tall), (2, 80, 5), "t").unwrap();
    assert_eq!(lines.len(), 5);
    // 大的：base64 超过 4096 字节分几块，前面的 `m=1`，最后一块 `m=0`。
    let noisy = tall.with_file_name("noisy");
    let mut bytes = Vec::new();
    image::RgbImage::from_fn(320, 160, |x, y| {
        image::Rgb([(x * 7) as u8, (y * 13) as u8, ((x ^ y) * 3) as u8])
    })
    .write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    std::fs::write(&noisy, bytes).unwrap();
    let lines = pictures.lines(("noisy", &noisy), (0, 80, 20), "n").unwrap();
    let mut strings = vec![String::new(); lines.len()];
    pictures.settle(&lines, &mut strings);
    let parts = strings[0].matches("\u{1b}_G").count();
    assert!(parts > 1, "分块：{parts}");
    assert!(strings[0].contains(",m=1;") && strings[0].contains("\u{1b}_Gm=0;"));
}
