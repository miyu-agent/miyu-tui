//! 在这个终端里用编辑器写字（`Ctrl+G` 写输入框、配置页和引导里改提示词，三处都走这里）：用哪个编辑器、怎么跑。界面让出、收回屏幕在
//! `main.rs`。本机的文件点了不再进编辑器，照「展示」的退路开（2026-10-24，`app/mouse.rs`）。

use std::io;
use std::path::Path;
use std::process::Command;

/// 用哪个编辑器：`$VISUAL`，没有的用 `$EDITOR`；空的不算。都没写的是 `None`（照 [`or_default`] 补）。
pub fn command(var: impl Fn(&str) -> Option<String>) -> Option<String> {
    ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(var)
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

/// 没写编辑器时用的：Unix 上 `vi`，Windows 上记事本（2026-11-15 三平台摸底：原来 Windows 上 `Ctrl+G` 不起作用，提示却照样写）。
pub fn or_default(configured: Option<String>) -> String {
    configured.unwrap_or_else(|| if cfg!(windows) { "notepad" } else { "vi" }.to_string())
}

/// 用 `editor` 打开 `file`，等它退出。`editor` 可以带参数（`code --wait`）：Unix 上照 shell 的样子拆；Windows 上经
/// `cmd /S /C` 跑（同 shell 的样子拆，带空格的程序路径照 cmd 的写法自己加引号），cmd 跑图形程序也等它退出。
///
/// # Errors
///
/// 起不来 `sh`（Windows 上是 `cmd`）。
pub fn run(editor: &str, file: &Path) -> io::Result<()> {
    command_for(editor, file).status().map(|_| ())
}

#[cfg(unix)]
fn command_for(editor: &str, file: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("miyu-edit")
        .arg(file);
    command
}

#[cfg(windows)]
fn command_for(editor: &str, file: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut command = Command::new("cmd");
    command.raw_arg(windows_line(editor, file));
    command
}

/// Windows 上交给 `cmd` 的那一整行：`/S /C "<编辑器> "<文件>""`。`/S` 让 cmd 只剥最外面那一对引号，里面照原样跑。
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_line(editor: &str, file: &Path) -> String {
    format!("/S /C \"{editor} \"{}\"\"", file.display())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::command;

    #[test]
    fn visual_comes_before_editor_and_blanks_do_not_count() {
        let env = |pairs: &[(&str, &str)]| {
            let map: HashMap<String, String> = pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            move |name: &str| map.get(name).cloned()
        };
        assert_eq!(command(env(&[("EDITOR", "vim")])).as_deref(), Some("vim"));
        let both = env(&[("VISUAL", "nvim"), ("EDITOR", "vim")]);
        assert_eq!(command(both).as_deref(), Some("nvim"));
        assert_eq!(
            command(env(&[("VISUAL", "  "), ("EDITOR", "vim")])).as_deref(),
            Some("vim")
        );
        assert_eq!(command(env(&[])), None);
    }

    /// 没写编辑器的照系统补：Windows 上记事本，别处 `vi`；写了的照写的（2026-11-15 三平台摸底）。
    #[test]
    fn a_missing_editor_falls_back_by_system() {
        let expected = if cfg!(windows) { "notepad" } else { "vi" };
        assert_eq!(super::or_default(None), expected);
        assert_eq!(super::or_default(Some("code --wait".into())), "code --wait");
    }

    /// Windows 上交给 cmd 的那一行：`/S` 只剥最外面一对引号，文件路径带空格也在引号里。
    #[test]
    fn the_windows_line_quotes_the_file() {
        let file = std::path::Path::new(r"C:\Users\A B\AppData\miyu\prompt.md");
        assert_eq!(
            super::windows_line("notepad", file),
            r#"/S /C "notepad "C:\Users\A B\AppData\miyu\prompt.md"""#
        );
    }

    /// 写着 `Ctrl+G` 的地方（`/help` 的按键、编辑窗底部的按键提示）在每个系统上都真能用：每个系统都补得出一个编辑器，
    /// 三处（输入框、配置页、引导）都走 [`super::run`]（2026-11-15 三平台摸底：原来 Windows 上写着却不起作用）。
    #[test]
    fn every_place_that_shows_ctrl_g_has_an_editor_here() {
        let config = crate::config::Config::builtin().unwrap();
        assert!(
            config.text.help.keys.iter().any(|[k, _]| k == "Ctrl+G"),
            "/help 写着 Ctrl+G"
        );
        let sheet = &config.text.settings.more.sheet_hints;
        assert!(
            sheet.iter().any(|[k, _]| k == "Ctrl+G"),
            "编辑窗底部写着 Ctrl+G"
        );
        assert!(
            !super::or_default(None).is_empty(),
            "这个系统上补得出编辑器"
        );
    }
}
