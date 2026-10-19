//! 通知的图标（蓝图 `tui.md`「系统通知」第 3 条，2026-10-17 项目主人定）：这个会话的人格有头像的用头像，没有的用终端的吉祥物
//! 画成的图（2026-10-19 项目主人：原来用一张二次元女孩的图 `miyu.png`，看着像「某个头像」，`mascot_icon.rs`）。头像缩成
//! 最长边 128 像素的 PNG 存进缓存目录，同一张只缩一次（`notify-send -i` 要的是文件）。

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use image::ImageFormat;
use image::imageops::FilterType;

/// 缩到最长边几个像素。
const SIDE: u32 = 128;

/// 这一条通知用哪张：头像缩好的那份，没有头像、缩不成的用吉祥物画成的（`mascot` 是现在的吉祥物，没有的不带
/// 图标）；`dir` 是放它们的目录，写不进去的是 `None`。
pub fn pick(avatar: Option<&Path>, mascot: Option<&Mascot>, dir: &Path) -> Option<PathBuf> {
    avatar
        .and_then(|a| small(a, dir))
        .or_else(|| mascot.and_then(|look| super::mascot_icon::file(look, dir)))
}

/// 现在的吉祥物（换了吉祥物包的照包的样子画）。
pub type Mascot = crate::mascot::Look;

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
    use super::{Mascot, SIDE, pick};

    #[test]
    fn an_avatar_is_shrunk_once_and_without_one_the_mascot_is_drawn() {
        let dir = std::env::temp_dir().join(format!("miyu-notify-icon-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        let mascot: Mascot = crate::config::Config::builtin().unwrap().mascot;
        let mascot = Some(&mascot);
        let miyu = pick(None, mascot, &dir).expect("吉祥物画成的图");
        assert!(
            miyu.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("mascot-")),
            "{miyu:?}"
        );
        assert_eq!(pick(None, None, &dir), None, "没有吉祥物的不带图标");
        let big = dir.join("big.png");
        image::RgbImage::new(600, 300).save(&big).unwrap();
        let small = pick(Some(&big), mascot, &dir).expect("缩好的头像");
        assert_ne!(small, miyu);
        let (w, h) = image::image_dimensions(&small).unwrap();
        assert_eq!((w, h), (SIDE, SIDE / 2), "最长边 128，照比例");
        assert_eq!(
            pick(Some(&big), mascot, &dir),
            Some(small),
            "同一张只缩一次"
        );
        let broken = dir.join("broken.png");
        std::fs::write(&broken, b"not a picture").unwrap();
        assert_eq!(
            pick(Some(&broken), mascot, &dir),
            Some(miyu),
            "读不出来的用吉祥物画成的"
        );
        drop(std::fs::remove_dir_all(&dir));
    }
}
