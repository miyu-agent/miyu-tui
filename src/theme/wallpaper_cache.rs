//! 壁纸取过的色记在缓存里（蓝图 `tui.md`「主题」的「跟随壁纸」第 6 条）：解一张 4K 的图要三十多 MB，起来时解一次，打开长
//! 会话的内存尖峰就是它（2026-10-15 性能对比查出）。同一张图（路径、大小、改动时间都一样）下次直接用记着的，不再读、不再解；
//! 只记最近的一张，换了壁纸的照常解一次再记。

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use miyu_client::places::Env;
use serde_json::{Value, json};

/// 记在哪：机器的缓存目录下的 `tui/wallpaper.json`；找不到缓存目录的不记。
pub fn file() -> Option<PathBuf> {
    miyu_client::places::cache_root(&Env::current())
        .ok()
        .map(|root| root.join("tui").join("wallpaper.json"))
}

/// 认一张图：路径、大小、改动时间（纳秒）。
fn mark(path: &Path) -> Option<Value> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(json!({
        "path": path.to_string_lossy(),
        "len": meta.len(),
        "modified": modified.as_nanos().to_string(),
    }))
}

/// 缓存 `file` 里记着的这张图的候选主色；没记、记的不是这张（换过、改过）是 `None`。
pub fn get(file: &Path, path: &Path) -> Option<Vec<u32>> {
    let want = mark(path)?;
    let got: Value = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
    if got["image"] != want {
        return None;
    }
    got["seeds"]
        .as_array()?
        .iter()
        .map(|s| s.as_str().and_then(super::persona::hex))
        .collect()
}

/// 在缓存 `file` 里记下这张图的候选主色；写不成的不管（下次起来再解一次）。
pub fn put(file: &Path, path: &Path, seeds: &[u32]) {
    let Some(image) = mark(path) else {
        return;
    };
    let seeds: Vec<String> = seeds.iter().map(|s| format!("#{s:06x}")).collect();
    let body = json!({"image": image, "seeds": seeds}).to_string();
    let temp = file.with_extension("json.tmp");
    let written = file
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&temp, body))
        .and_then(|()| std::fs::rename(&temp, file));
    if written.is_err() {
        // 写不成：下次起来再解一次。
    }
}

#[cfg(test)]
mod tests {
    use super::{get, put};

    #[test]
    fn the_same_picture_reads_back_and_a_changed_one_does_not() {
        let dir = std::env::temp_dir().join(format!("miyu-tui-wallcache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (cache, picture) = (dir.join("tui/wallpaper.json"), dir.join("wall.png"));
        std::fs::write(&picture, b"one").unwrap();
        assert_eq!(get(&cache, &picture), None, "没记过");
        put(&cache, &picture, &[0x2f_ab34, 0x11_2233]);
        assert_eq!(get(&cache, &picture), Some(vec![0x2f_ab34, 0x11_2233]));
        // 换了内容（大小变了）：不认。
        std::fs::write(&picture, b"other picture").unwrap();
        assert_eq!(get(&cache, &picture), None, "换过的图不认");
        assert_eq!(get(&cache, &dir.join("gone.png")), None, "别的图不认");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
