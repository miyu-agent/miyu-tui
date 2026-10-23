//! 回答里写的本机地址指到哪个文件（蓝图 `tui.md`「她的回答：Markdown」第 10、12 条）：
//! `file://` 去掉头，`~/` 换成家目录，别的相对路径照工作目录找。点链接和画图片用同一套认法。

use std::path::{Path, PathBuf};

/// 家目录里的路径写成 `~/…`（照 `miyu ask`）：时间线的对象、侧边栏的工作目录都照它写。
pub fn home_short(value: &str) -> String {
    match std::env::home_dir().and_then(|home| {
        Path::new(value)
            .strip_prefix(home)
            .ok()
            .map(|r| r.display().to_string())
    }) {
        Some(rest) if rest.is_empty() => "~".into(),
        Some(rest) => format!("~/{rest}"),
        None => value.to_string(),
    }
}

/// 环境变量里写的程序路径：开头的 `~/` 换成家目录（`home`），别的原样。fish 只在词开头展开 `~`，`env MIYU_CORE_BIN=~/…`
/// 交来的是字面的 `~/…`（2026-10-16 项目主人照验收清单起终端，报「找不到核心程序：~/…」）。
pub fn expand_home(value: std::ffi::OsString, home: Option<&Path>) -> PathBuf {
    match (value.to_str().and_then(|v| v.strip_prefix("~/")), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(value),
    }
}

/// 照给定的家目录、工作目录认。
pub fn path(url: &str, home: Option<&Path>, cwd: &Path) -> PathBuf {
    let url = url.strip_prefix("file://").unwrap_or(url);
    if let (Some(rest), Some(home)) = (url.strip_prefix("~/"), home) {
        return home.join(rest);
    }
    let path = Path::new(url);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// 照这个进程的家目录（`HOME`）、工作目录认；工作目录读不出来时交回 `None`。
pub fn resolve(url: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let cwd = std::env::current_dir().ok()?;
    Some(path(url, home.as_deref(), &cwd))
}

/// 字里写的本机路径，文件（或目录）真在的交回打开时用的绝对路径：步骤标题的对象、回答里的路径点了能开（2026-10-22 项目主人：
/// 「pi 的文件链接也是可以点击打开的」）。末尾的 `:行号`、`:行号:列号` 不算进路径。
pub fn existing(text: &str) -> Option<String> {
    let bare = strip_line(text);
    if bare.is_empty() || bare.contains("://") || bare.len() > 4096 {
        return None;
    }
    let file = resolve(bare)?;
    file.exists().then(|| file.display().to_string())
}

/// 去掉末尾的 `:12`、`:12:3`。
fn strip_line(text: &str) -> &str {
    let mut rest = text;
    for _ in 0..2 {
        match rest.rsplit_once(':') {
            Some((head, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
                rest = head
            }
            _ => break,
        }
    }
    rest
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::path;

    #[test]
    fn a_program_path_starting_with_a_tilde_is_under_home() {
        let home = std::path::Path::new("/home/me");
        let expand = |v: &str| super::expand_home(v.into(), Some(home));
        assert_eq!(expand("~/bin/miyu"), home.join("bin/miyu"));
        assert_eq!(expand("/opt/miyu"), std::path::PathBuf::from("/opt/miyu"));
        assert_eq!(
            expand("bin/~/miyu"),
            std::path::PathBuf::from("bin/~/miyu"),
            "只认开头的"
        );
        assert_eq!(
            super::expand_home("~/miyu".into(), None),
            std::path::PathBuf::from("~/miyu")
        );
    }

    #[test]
    fn addresses_resolve_to_files() {
        let home = Path::new("/home/me");
        let cwd = Path::new("/work");
        assert_eq!(
            path("~/a.png", Some(home), cwd),
            PathBuf::from("/home/me/a.png")
        );
        assert_eq!(
            path("file:///tmp/b.png", Some(home), cwd),
            PathBuf::from("/tmp/b.png")
        );
        assert_eq!(path("/x/c.png", Some(home), cwd), PathBuf::from("/x/c.png"));
        assert_eq!(
            path("img/d.png", Some(home), cwd),
            PathBuf::from("/work/img/d.png")
        );
    }

    #[test]
    fn a_written_path_counts_only_when_the_file_is_there() {
        let dir = std::env::temp_dir().join(format!("miyu-tui-existing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.rs");
        std::fs::write(&file, "x").unwrap();
        let file = file.canonicalize().unwrap();
        let shown = file.display().to_string();
        assert_eq!(super::existing(&shown).as_deref(), Some(shown.as_str()));
        assert_eq!(
            super::existing(&format!("{shown}:12")).as_deref(),
            Some(shown.as_str())
        );
        assert_eq!(
            super::existing(&format!("{shown}:12:3")).as_deref(),
            Some(shown.as_str())
        );
        assert_eq!(super::existing(&format!("{shown}.gone")), None);
        assert_eq!(
            super::existing(&format!("file://{shown}")).as_deref(),
            None,
            "地址不当路径"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
