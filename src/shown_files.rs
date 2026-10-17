//! 展示的文件在本机的副本（蓝图 `tui.md`「展示」第 6 条）：照 `blob.get` 读回来的那一份没有扩展名，拷一份带原文件名的
//! 到临时目录，画图（`.svg` 照扩展名认）、用程序开都用它。同一份只拷一次。

use std::path::{Path, PathBuf};

/// 卡片、图挂的地址：`miyu-shown:<条目编号>`，点了照展示的退路开，不当网址开（`app/shown.rs`）。
pub const SCHEME: &str = "miyu-shown:";

/// mermaid 大图挂的地址：`miyu-zoom:<大图的路径>`，点了照同一个退路开（蓝图「展示」第 7 条）。
pub const ZOOM: &str = "miyu-zoom:";

/// `blob` 的副本该在哪：`<临时目录>/miyu-shown/<blob 前 12 位>/<文件名>`。文件名只取最后一段，空的、`.`、`..` 换成
/// `file`。
pub fn place(blob: &str, name: &str) -> PathBuf {
    // 哈希去掉 `sha256:` 这类前缀。
    let key: String = blob
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect();
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let base = if base.is_empty() || base == "." || base == ".." {
        "file"
    } else {
        base
    };
    std::env::temp_dir()
        .join("miyu-shown")
        .join(if key.is_empty() { "blob" } else { &key })
        .join(base)
}

/// 照读回来的 `from` 拷出副本，交回它的位置；已经在的不再拷，拷不成的是 `None`。
pub fn copy(blob: &str, name: &str, from: &Path) -> Option<PathBuf> {
    let to = place(blob, name);
    if to.is_file() {
        return Some(to);
    }
    std::fs::create_dir_all(to.parent()?).ok()?;
    std::fs::copy(from, &to).ok()?;
    Some(to)
}

#[cfg(test)]
mod tests {
    use super::{copy, place};

    #[test]
    fn the_copy_keeps_the_file_name_and_is_made_once() {
        let at = place("sha256:0123456789abcdef", "../页面.html");
        assert!(
            at.ends_with("miyu-shown/0123456789ab/页面.html"),
            "{}",
            at.display()
        );
        assert!(place("sha256:ab", "").ends_with("file"));
        let dir = std::env::temp_dir().join(format!("miyu-shown-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let from = dir.join("blob");
        std::fs::write(&from, "<p>hi</p>").unwrap();
        let blob = format!("sha256:{:012x}", std::process::id());
        let made = copy(&blob, "index.html", &from).unwrap();
        assert_eq!(std::fs::read_to_string(&made).unwrap(), "<p>hi</p>");
        std::fs::write(&from, "changed").unwrap();
        let again = copy(&blob, "index.html", &from).unwrap();
        assert_eq!(
            std::fs::read_to_string(&again).unwrap(),
            "<p>hi</p>",
            "同一份只拷一次"
        );
        drop(std::fs::remove_dir_all(made.parent().unwrap()));
        drop(std::fs::remove_dir_all(&dir));
    }
}
