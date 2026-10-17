//! 通知的图标（蓝图 `tui.md`「系统通知」第 3 条，2026-10-17 项目主人定）：这个会话的人格有头像的用头像，没有的用 Miyu 的
//! 图标（`resources/notify/miyu.png`，网页同一张）。头像缩成最长边 128 像素的 PNG 存进缓存目录，同一张只缩一次；
//! Miyu 的图标编进程序，用时写进缓存目录（`notify-send -i` 要的是文件）。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::ImageFormat;
use image::imageops::FilterType;

/// Miyu 的图标。
const MIYU: &[u8] = include_bytes!("../../resources/notify/miyu.png");

/// 缩到最长边几个像素。
const SIDE: u32 = 128;

/// 这一条通知用哪张：头像缩好的那份，没有头像、缩不成的用 Miyu 的；`dir` 是放它们的目录，写不进去的是 `None`。
pub fn pick(avatar: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    avatar.and_then(|a| small(a, dir)).or_else(|| miyu(dir))
}

/// 把 Miyu 的图标写进 `dir`（已经在、大小对的不重写）。
fn miyu(dir: &Path) -> Option<PathBuf> {
    let file = dir.join("miyu.png");
    let fresh = fs::metadata(&file).is_ok_and(|m| m.len() == MIYU.len() as u64);
    if !fresh {
        fs::create_dir_all(dir).ok()?;
        fs::write(&file, MIYU).ok()?;
    }
    Some(file)
}

/// 头像缩好的那份：名字照原文件的路径、大小、改的时刻算，已经在的直接用。
fn small(avatar: &Path, dir: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(avatar).ok()?;
    let mut h = DefaultHasher::new();
    (avatar, meta.len(), meta.modified().ok()).hash(&mut h);
    let file = dir.join(format!("avatar-{:016x}.png", h.finish()));
    if file.is_file() {
        return Some(file);
    }
    let picture = image::ImageReader::open(avatar)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let picture = picture.resize(SIDE, SIDE, FilterType::Triangle);
    fs::create_dir_all(dir).ok()?;
    picture.save_with_format(&file, ImageFormat::Png).ok()?;
    Some(file)
}

#[cfg(test)]
mod tests {
    use super::{SIDE, pick};

    #[test]
    fn an_avatar_is_shrunk_once_and_without_one_miyu_is_used() {
        let dir = std::env::temp_dir().join(format!("miyu-notify-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let miyu = pick(None, &dir).expect("Miyu 的图标");
        assert!(miyu.ends_with("miyu.png"));
        let big = dir.join("big.png");
        image::RgbImage::new(600, 300).save(&big).unwrap();
        let small = pick(Some(&big), &dir).expect("缩好的头像");
        assert_ne!(small, miyu);
        let (w, h) = image::image_dimensions(&small).unwrap();
        assert_eq!((w, h), (SIDE, SIDE / 2), "最长边 128，照比例");
        assert_eq!(pick(Some(&big), &dir), Some(small), "同一张只缩一次");
        let broken = dir.join("broken.png");
        std::fs::write(&broken, b"not a picture").unwrap();
        assert_eq!(
            pick(Some(&broken), &dir),
            Some(miyu),
            "读不出来的用 Miyu 的"
        );
        drop(std::fs::remove_dir_all(&dir));
    }
}
