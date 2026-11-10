//! 通知的图标（蓝图 `tui.md`「系统通知」第 3 条，2026-10-17 项目主人定）：这个会话的人格有头像的用头像，没有的不带图标
//! （2026-11-10 项目主人：「无人格没有头像」；原来画吉祥物）。头像缩成最长边 128 像素的 PNG 存进缓存目录，同一张只缩一次（`notify-send -i` 要的是文件）。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::ImageFormat;
use image::imageops::FilterType;

/// 缩到最长边几个像素。
const SIDE: u32 = 128;

/// 这一条通知用哪张：头像缩好的那份；`dir` 是放它的目录，读不出来、写不进去的是 `None`（不带图标）。
pub fn pick(avatar: &Path, dir: &Path) -> Option<PathBuf> {
    small(avatar, dir)
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
    fn an_avatar_is_shrunk_once_and_a_broken_one_gives_no_icon() {
        let dir = std::env::temp_dir().join(format!("miyu-notify-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("big.png");
        image::RgbImage::new(600, 300).save(&big).unwrap();
        let small = pick(&big, &dir).expect("缩好的头像");
        let (w, h) = image::image_dimensions(&small).unwrap();
        assert_eq!((w, h), (SIDE, SIDE / 2), "最长边 128，照比例");
        assert_eq!(pick(&big, &dir), Some(small), "同一张只缩一次");
        let broken = dir.join("broken.png");
        std::fs::write(&broken, b"not a picture").unwrap();
        assert_eq!(pick(&broken, &dir), None, "读不出来的不带图标");
        drop(std::fs::remove_dir_all(&dir));
    }
}
