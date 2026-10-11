//! 交给编辑器改的临时文件（`Ctrl+G` 的草稿、人格提示词，review A9，2026-10-11）：放在自己的缓存目录
//! `<缓存目录>/tui/edit/` 里（Unix 上目录 0700），新建、Unix 上 0600、名字带进程号和随机数，已有的不覆盖、不顺着链接写。
//! 原来放共享的临时目录、名字可预测、0644：同机别的用户读得到草稿和提示词，没开 `protected_symlinks` 的系统上还能被
//! 链接引到别处。找不到缓存目录的退回系统的临时目录，照样新建、0600。

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use miyu_client::places::{Env, cache_root};

/// 写一份临时文件，交回它在哪。`stem` 是名字的开头（`prompt`、`persona` 这类）。
pub fn write(stem: &str, text: &str) -> io::Result<PathBuf> {
    write_in(&dir(), stem, text)
}

/// 放临时文件的目录：建好、Unix 上收紧成 0700，交回它（缩小的头像也写在这里，`avatars/shrink.rs`）。建不成、收紧不了的
/// 退回系统的临时目录。
pub fn private_dir() -> PathBuf {
    let dir = dir();
    match prepare(&dir) {
        Ok(()) => dir,
        Err(_) => std::env::temp_dir(),
    }
}

/// `<缓存目录>/tui/edit/`；找不到缓存目录的是系统的临时目录。
fn dir() -> PathBuf {
    cache_root(&Env::current())
        .map(|root| root.join("tui").join("edit"))
        .unwrap_or_else(|_| std::env::temp_dir())
}

/// 同 [`write`](fn@write)，写在 `dir` 里。
pub fn write_in(dir: &Path, stem: &str, text: &str) -> io::Result<PathBuf> {
    prepare(dir)?;
    let salt = crate::rng::Rng::from_clock().next();
    let path = dir.join(format!("miyu-{stem}-{}-{salt:x}.md", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    file.write_all(text.as_bytes())?;
    Ok(path)
}

/// 建好目录；Unix 上收紧成 0700，只在自己建的目录上收紧，系统的临时目录（退回的那一种）不动。
fn prepare(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if dir != std::env::temp_dir() {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
