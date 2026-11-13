//! 展示的文件在哪（蓝图 `tui.md`「展示」第 6 条，核心 SW-1，2026-11-13：展示不限大小，内容不进核心）：终端和核心同一台机器，
//! 照 `shown.path` 直接读、直接开；读之前核对大小和修改时间，和展示时记的不一样写「文件已变」，没了写「文件已不在」（同核心
//! `session.shown` 的 `file_changed`、`file_gone`）。

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::core::Shown;

/// 卡片、图挂的地址：`miyu-shown:<条目编号>`，点了照展示的退路开，不当网址开（`app/shown.rs`）。
pub const SCHEME: &str = "miyu-shown:";

/// mermaid 大图挂的地址：`miyu-zoom:<大图的路径>`，点了照同一个退路开（蓝图「展示」第 7 条）。
pub const ZOOM: &str = "miyu-zoom:";

/// 展示的文件这一刻怎样。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// 还是展示时那一份：照这个路径读、开。
    Here(PathBuf),
    /// 大小或修改时间变了。
    Changed,
    /// 没了、不是普通文件了。
    Gone,
}

/// 修改时间差在这么多以内算没变：文件系统记的精度不一（有的只到秒）。
const SLACK: Duration = Duration::from_secs(1);

/// 看一眼展示的文件：照 `path` 取大小、修改时间，和展示时记的比。核心没给修改时间的只比大小。
pub fn find(file: &Shown) -> Found {
    let path = PathBuf::from(&file.path);
    let Ok(meta) = std::fs::metadata(&path) else {
        return Found::Gone;
    };
    if !meta.is_file() {
        return Found::Gone;
    }
    if meta.len() != file.size {
        return Found::Changed;
    }
    let shown: Option<SystemTime> = file
        .modified
        .as_deref()
        .and_then(|m| m.parse::<jiff::Timestamp>().ok())
        .map(SystemTime::from);
    let moved = match (shown, meta.modified().ok()) {
        (Some(shown), Some(now)) => {
            let gap = now.duration_since(shown).unwrap_or_else(|e| e.duration());
            gap > SLACK
        }
        _ => false,
    };
    if moved {
        Found::Changed
    } else {
        Found::Here(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{Found, find};
    use crate::core::Shown;

    fn shown(path: &std::path::Path, size: u64, modified: Option<String>) -> Shown {
        Shown {
            path: path.display().to_string(),
            modified,
            name: "a.html".into(),
            media_type: "text/html".into(),
            size,
            title: None,
            language: None,
        }
    }

    #[test]
    fn a_shown_file_is_read_where_it_is_and_checked_for_changes() {
        let dir = std::env::temp_dir().join(format!("miyu-shown-find-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.html");
        std::fs::write(&file, "<p>hi</p>").unwrap();
        let at = jiff::Timestamp::try_from(std::fs::metadata(&file).unwrap().modified().unwrap())
            .unwrap()
            .to_string();
        assert_eq!(
            find(&shown(&file, 9, Some(at.clone()))),
            Found::Here(file.clone())
        );
        assert_eq!(
            find(&shown(&file, 9, None)),
            Found::Here(file.clone()),
            "没给修改时间的只比大小"
        );
        assert_eq!(
            find(&shown(&file, 10, Some(at))),
            Found::Changed,
            "大小变了"
        );
        let long_ago = jiff::Timestamp::from_second(1_000_000_000)
            .unwrap()
            .to_string();
        assert_eq!(
            find(&shown(&file, 9, Some(long_ago))),
            Found::Changed,
            "修改时间变了"
        );
        assert_eq!(find(&shown(&dir.join("gone.html"), 9, None)), Found::Gone);
        assert_eq!(find(&shown(&dir, 9, None)), Found::Gone, "目录不算普通文件");
        drop(std::fs::remove_dir_all(&dir));
    }
}
