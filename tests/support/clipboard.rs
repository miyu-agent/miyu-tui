//! 带图发消息要的（从 `tests/refused.rs` 挪来，`tests/prefix_ask.rs` 也用）：工作目录里放一张图；假的 `wl-paste`（放在
//! `PATH` 前面），剪贴板里是文件管理器复制的那个文件，不碰跑测试那台机器上的剪贴板。`wl-paste` 只有 Linux 上读，按
//! `Ctrl+V` 贴图的测试只在 Linux 上跑。

use super::Home;
#[cfg(target_os = "linux")]
use super::Tui;

/// 在工作目录里放一张图，交回它的路径。
pub fn picture(home: &Home) -> String {
    let path = home.work.join("晚霞.png");
    std::fs::write(&path, b"\x89PNG\r\n\x1a\n").expect("写得进去");
    path.display().to_string()
}

/// 一个假的 `wl-paste`：剪贴板里是文件管理器复制的这个文件（`text/uri-list`）。交回要设的 `PATH`。
#[cfg(target_os = "linux")]
pub fn copied(home: &Home, path: &str) -> String {
    let bin = home.work.join("bin");
    std::fs::create_dir_all(&bin).expect("建得了目录");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  --list-types) printf 'text/uri-list\\ntext/plain\\n' ;;\n  --type) [ \"$2\" = text/uri-list ] && printf 'file://%s\\n' '{path}' ;;\n  *) printf '%s' '{path}' ;;\nesac\n"
    );
    let fake = bin.join("wl-paste");
    std::fs::write(&fake, script).expect("写得进去");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("改得了权限");
    // 假剪贴板在最前面，接着测具的假通知、放音程序（`fake_notifiers`），再是原来的 `PATH`。
    format!(
        "{}:{}:{}",
        bin.display(),
        super::fake_notifiers(home.root()).display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

/// 起界面：剪贴板是上面那个假的。
#[cfg(target_os = "linux")]
pub fn with_clipboard(home: &Home, path: &str) -> Tui {
    let path_env = copied(home, path);
    home.tui_with(
        "zh_CN.UTF-8",
        &[
            ("PATH", path_env.as_str()),
            ("WAYLAND_DISPLAY", "miyu-test"),
        ],
    )
}

/// 数据根里记着 `said` 那一句的 `message.user`（会话日志一行一件，不管放在哪个目录）。
pub fn logged_message(root: &std::path::Path, said: &str) -> Option<serde_json::Value> {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "jsonl") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let found = text
                    .lines()
                    .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                    .find(|e| {
                        e.to_string().contains("message.user") && e.to_string().contains(said)
                    });
                if found.is_some() {
                    return found;
                }
            }
        }
    }
    None
}
