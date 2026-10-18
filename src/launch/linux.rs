//! Linux 上的默认程序（蓝图 `tui.md`「展示」第 7 条）：照这份文件本身问 `xdg-mime` 是什么种类，再问那个种类的默认程序
//! 是哪个桌面文件，照它启动（`gio launch`，没有的 `gtk-launch`，再没有的 `xdg-open`）。
//!
//! 两处照着 2026-10-17、10-18 项目主人验的改：种类照文件本身问（`query filetype`），不照核心报的 `media_type`——核心认不出的
//! 一律给 `application/octet-stream`，那份 Word 文档就被当成二进制交给了别的程序；问不到才退回核心报的那种。写着
//! `Terminal=true` 的程序（vim 这类）照常交给它，`gio launch`、`gtk-launch` 自己套一个终端窗口把它跑起来；以前当没有默认
//! 程序、往下交浏览器，Markdown 卡片点了就开成浏览器（T-270）。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::{Env, spawn};

/// 交给默认程序开了交回 `true`；没有默认程序、起不来的是 `false`。
pub(super) fn default_program(path: &Path, media_type: &str, env: &Env) -> bool {
    let media_type = file_type(path, env).unwrap_or_else(|| media_type.to_string());
    let Some(id) = default_id(&media_type, env) else {
        return false;
    };
    let Some(desktop) = find(&id, env) else {
        return false;
    };
    let mut gio = env.command("gio");
    gio.arg("launch").arg(&desktop).arg(path);
    if spawn(gio) {
        return true;
    }
    let mut gtk = env.command("gtk-launch");
    gtk.arg(&id).arg(path);
    if spawn(gtk) {
        return true;
    }
    let mut open = env.command("xdg-open");
    open.arg(path);
    spawn(open)
}

/// `xdg-mime query filetype <路径>` 说的种类；问不到的是 `None`。
fn file_type(path: &Path, env: &Env) -> Option<String> {
    query(env, &["query", "filetype"], path.as_os_str())
}

/// `xdg-mime query default <种类>` 说的桌面文件名；没有的是 `None`。
fn default_id(media_type: &str, env: &Env) -> Option<String> {
    query(env, &["query", "default"], media_type.as_ref())
}

/// 问一次 `xdg-mime`：`query` 后面跟的那两下、再跟 `last`，它说的那一行；问不到的是 `None`。
fn query(env: &Env, query: &[&str], last: &std::ffi::OsStr) -> Option<String> {
    let out = env
        .command("xdg-mime")
        .args(query)
        .arg(last)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !said.is_empty()).then_some(said)
}

/// 找桌面文件的几个目录：`XDG_DATA_HOME`（没设的 `~/.local/share`）在前，接着 `XDG_DATA_DIRS`（没设的
/// `/usr/local/share:/usr/share`）。
fn data_dirs(env: &Env) -> Vec<PathBuf> {
    if let Some(dirs) = &env.data_dirs {
        return dirs.clone();
    }
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|h| h.join(".local/share")));
    let rest = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|d| !d.trim().is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    home.into_iter()
        .chain(rest.split(':').filter(|d| !d.is_empty()).map(PathBuf::from))
        .collect()
}

/// 桌面文件在哪：头一个有它的目录里的 `applications/<名字>`。
fn find(id: &str, env: &Env) -> Option<PathBuf> {
    data_dirs(env)
        .into_iter()
        .map(|dir| dir.join("applications").join(id))
        .find(|file| file.is_file())
}
