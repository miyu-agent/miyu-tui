//! 旧版那一份的备份（蓝图「旧版那一份」第 3、4 条）：同目录的 `<名字>.miyu-old-hook.bak`，已有的不盖、往后编号。

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::super::words::{Words, fill};

/// 同目录下 `<名字>.miyu-old-hook.bak`，第 `n` 个往后是 `.bak.<n>`。
fn backup_name(path: &Path, n: usize) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".miyu-old-hook.bak");
    if n > 0 {
        name.push(format!(".{n}"));
    }
    path.with_file_name(name)
}

/// 还没有的备份名（fish 改名用：改名会盖掉已有的，先看）。
pub(super) fn free(path: &Path) -> io::Result<PathBuf> {
    let mut n = 0;
    loop {
        let name = backup_name(path, n);
        match fs::symlink_metadata(&name) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(name),
            Err(e) => return Err(e),
            Ok(_) => n += 1,
        }
    }
}

/// 启动文件 `path` 整份（`text`，读到的原样）备份到同目录（链接的放在链接所在的目录）：已有的不盖、往后编号，权限照
/// 原文件，写完读回来和原样逐字节一样才算成。没成的不留半份。
pub(super) fn backup(path: &Path, text: &[u8], words: &Words) -> io::Result<PathBuf> {
    let permissions = fs::metadata(path)?.permissions();
    let mut n = 0;
    loop {
        let name = backup_name(path, n);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        // 先只给自己读写，写完再照原文件的：原文件只有自己读得了的，备份也不会有一会儿别人读得了。
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = match options.open(&name) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                n += 1;
                continue;
            }
            Err(e) => return Err(e),
        };
        let written = write_backup(file, &name, text, permissions, words);
        return match written {
            Ok(()) => Ok(name),
            Err(e) => match fs::remove_file(&name) {
                Ok(()) => Err(e),
                Err(left) => Err(io::Error::new(
                    e.kind(),
                    format!("{e}; {}: {left}", name.display()),
                )),
            },
        };
    }
}

/// 写备份、落盘、权限照原文件，读回来核对。
fn write_backup(
    mut file: fs::File,
    name: &Path,
    text: &[u8],
    permissions: fs::Permissions,
    words: &Words,
) -> io::Result<()> {
    file.write_all(text)?;
    file.sync_all()?;
    drop(file);
    fs::set_permissions(name, permissions)?;
    if fs::read(name)? != text {
        let shown = name.display().to_string();
        return Err(io::Error::other(fill(
            &words.old_backup_differs,
            &[("backup", &shown)],
        )));
    }
    Ok(())
}
