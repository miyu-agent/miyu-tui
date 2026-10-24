//! 在这个终端里用编辑器写字（`Ctrl+G` 写输入框、配置页和引导里改提示词）：用哪个编辑器、怎么跑。界面让出、收回屏幕在
//! `main.rs`。本机的文件点了不再进编辑器，照「展示」的退路开（2026-10-24，`app/mouse.rs`）。

use std::io;
use std::path::Path;
use std::process::Command;

/// 用哪个编辑器：`$VISUAL`，没有的用 `$EDITOR`；空的不算。Windows 上不用（照旧交给系统）。
pub fn command(var: impl Fn(&str) -> Option<String>) -> Option<String> {
    if !cfg!(unix) {
        return None;
    }
    ["VISUAL", "EDITOR"]
        .into_iter()
        .filter_map(var)
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

/// 用 `editor` 打开 `file`，等它退出。`editor` 可以带参数（`code --wait`），照 shell 的样子拆。
///
/// # Errors
///
/// 起不来 `sh`。
pub fn run(editor: &str, file: &Path) -> io::Result<()> {
    Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("miyu-edit")
        .arg(file)
        .status()
        .map(|_| ())
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
        if !cfg!(unix) {
            return;
        }
        assert_eq!(command(env(&[("EDITOR", "vim")])).as_deref(), Some("vim"));
        let both = env(&[("VISUAL", "nvim"), ("EDITOR", "vim")]);
        assert_eq!(command(both).as_deref(), Some("nvim"));
        assert_eq!(
            command(env(&[("VISUAL", "  "), ("EDITOR", "vim")])).as_deref(),
            Some("vim")
        );
        assert_eq!(command(env(&[])), None);
    }
}
