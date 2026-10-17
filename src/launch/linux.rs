//! Linux 上的默认程序（蓝图 `tui.md`「展示」第 7 条）：照核心认的种类问 `xdg-mime` 默认程序是哪个桌面文件，照它启动
//! （`gio launch`，没有的 `gtk-launch`，再没有的 `xdg-open`）。不交给 `xdg-open` 自己认：它照内容认，Markdown 常被认成
//! `text/plain`、交给 vim；写着 `Terminal=true` 的程序（vim 这类）没有终端起不来，当没有默认程序，往下交浏览器
//! （2026-10-17 项目主人验 T-270：Markdown 卡片点了没反应，手动 `xdg-open` 开的是 vim）。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use super::{Env, spawn};

/// 交给默认程序开了交回 `true`；没有默认程序、默认的要终端、起不来的是 `false`。
pub(super) fn default_program(path: &Path, media_type: &str, env: &Env) -> bool {
    let Some(id) = default_id(media_type, env) else {
        return false;
    };
    let Some(desktop) = find(&id, env) else {
        return false;
    };
    if needs_terminal(&desktop) {
        return false;
    }
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

/// `xdg-mime query default <种类>` 说的桌面文件名；没有的是 `None`。
fn default_id(media_type: &str, env: &Env) -> Option<String> {
    let out = env
        .command("xdg-mime")
        .args(["query", "default", media_type])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !id.is_empty()).then_some(id)
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

/// 桌面文件的 `[Desktop Entry]` 里写着 `Terminal=true`。
fn needs_terminal(desktop: &Path) -> bool {
    let text = std::fs::read_to_string(desktop).unwrap_or_default();
    let mut main = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            main = line == "[Desktop Entry]";
        } else if main && let Some(value) = line.strip_prefix("Terminal=") {
            return value.trim().eq_ignore_ascii_case("true");
        }
    }
    false
}
