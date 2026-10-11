//! `<img>` 写的宽高定大小（蓝图 `tui.md`「图片、公式和 mermaid 图」第 8 条）。

use image::{Rgba, RgbaImage};

use super::draw;
use crate::config::{Config, FigureLook};
use crate::figures::cells::Cell;

#[test]
fn written_width_and_height_decide_the_cells() {
    let path = std::env::temp_dir().join(format!("miyu-img-{}.png", std::process::id()));
    RgbaImage::from_pixel(10, 10, Rgba([0, 128, 255, 255]))
        .save(&path)
        .unwrap();
    let url = path.display().to_string();
    let cell = Cell {
        width: 10,
        height: 20,
    };
    let cells = |size| {
        let (_, fit) = draw(&url, size, cell, 100, 100, &look()).unwrap();
        (fit.cols, fit.rows)
    };
    assert_eq!(cells((None, None)), (1, 1), "没写的照图本身");
    assert_eq!(cells((Some(40), Some(20))), (4, 1), "写了宽高照写的");
    assert_eq!(cells((Some(40), None)), (4, 2), "只写宽：高照原图比例");
    assert_eq!(cells((None, Some(40))), (4, 2), "只写高：宽照原图比例");
    let (_, fit) = draw(&url, (Some(400), None), cell, 10, 100, &look()).unwrap();
    assert_eq!(fit.cols, 10, "宽过正文的等比缩");
    std::fs::remove_file(&path).unwrap_or_default();
}

#[test]
fn a_local_svg_file_is_drawn_like_an_inline_svg() {
    // 2026-09-29 项目主人问 svg 不能渲染吗：原来 `.svg` 文件当成读不懂，只有那一行字。
    let path = std::env::temp_dir().join(format!("miyu-img-{}.svg", std::process::id()));
    std::fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="40"><rect width="20" height="40" fill="#e5a07a"/></svg>"##,
    )
    .unwrap();
    let url = path.display().to_string();
    let cell = Cell {
        width: 10,
        height: 20,
    };
    let (image, fit) = draw(&url, (None, None), cell, 100, 100, &look()).unwrap();
    assert_eq!((fit.cols, fit.rows), (2, 2), "照它自己的大小");
    assert!(image.pixels().any(|p| p.0[3] > 0), "画出了东西");
    let (_, fit) = draw(&url, (Some(40), None), cell, 100, 100, &look()).unwrap();
    assert_eq!(
        (fit.cols, fit.rows),
        (4, 4),
        "`<img>` 写的宽照第 8 条，高照比例"
    );
    std::fs::remove_file(&path).unwrap_or_default();
}

#[test]
fn ppm_and_bmp_files_are_read() {
    // 2026-09-30 项目主人：`grim` 截的 `.ppm` 画不出来。
    for ext in ["ppm", "bmp"] {
        let path = std::env::temp_dir().join(format!("miyu-img-{}.{ext}", std::process::id()));
        image::RgbImage::from_pixel(10, 10, image::Rgb([200, 100, 50]))
            .save(&path)
            .unwrap();
        let cell = Cell {
            width: 10,
            height: 20,
        };
        let drawn = draw(
            &path.display().to_string(),
            (None, None),
            cell,
            100,
            100,
            &look(),
        );
        assert!(drawn.is_ok(), "{ext}：{:?}", drawn.err());
        std::fs::remove_file(&path).unwrap_or_default();
    }
}

/// 出厂的图片配置。
fn look() -> FigureLook {
    Config::builtin().expect("出厂配置").figures
}

/// 不是普通文件的图片地址（只有 Unix 上建得出 FIFO）。
#[cfg(unix)]
mod not_a_file {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::look;
    use crate::figures::cells::Cell;
    use crate::figures::file::draw;

    #[test]
    fn a_picture_address_that_is_not_a_plain_file_is_refused_at_once() {
        // review A4（2026-10-11）：回答里的图片地址写 /dev/tty、一个 FIFO，出图线程原来直接打开：读终端吃掉按键，
        // 或者永远卡在 open 上，之后的图全停在「正在画图」。只读普通文件。
        let dir = std::env::temp_dir().join(format!("miyu-fifo-{}", std::process::id()));
        drop(std::fs::create_dir_all(&dir));
        let fifo = dir.join("pipe.png");
        drop(std::fs::remove_file(&fifo));
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        assert!(made.is_ok_and(|s| s.success()), "建得了 FIFO");
        let look = look();
        let url = fifo.display().to_string();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let cell = Cell {
                width: 10,
                height: 20,
            };
            let got = draw(&url, (None, None), cell, 40, 10, &look);
            // 等的那一头超时走了的，发不出去也不要紧。
            if tx.send(got.is_err()).is_err() {}
        });
        let refused = rx.recv_timeout(Duration::from_secs(3));
        drop(std::fs::remove_dir_all(&dir));
        assert_eq!(refused, Ok(true), "当场拒，不卡在打开上");
    }
}
