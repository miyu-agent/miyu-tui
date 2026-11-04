//! 假的系统通知、放音程序（core1 11-04：伪终端测试起的界面走系统通知，真调了 `notify-send`，弹到了项目主人的桌面上）：测具起界面时
//! 放在数据根旁边、排在 `PATH` 最前面（`mod.rs` 的 `program_sized`），界面照 `MIYU_TUI_FAKES` 只许拉起这里的（`notify/system.rs`
//! 的 `guard`）。

use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;

/// 起界面的命令接上假程序：放它们的目录排在 `PATH` 最前面，设 `MIYU_TUI_FAKES`（界面看到它就只许拉起测试目录里的程序）。
/// 测试自己给 `PATH` 的照它的（后设的盖掉这里的）。
pub(super) fn wire(command: &mut CommandBuilder, home: &Path) {
    let fakes = fake_notifiers(home);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs = vec![fakes.clone()];
    dirs.extend(std::env::split_paths(&path));
    command.env("PATH", std::env::join_paths(dirs).expect("拼得成 PATH"));
    command.env("MIYU_TUI_FAKES", &fakes);
}

/// 假的系统通知、放音程序（`notify-send`、`osascript`、`pw-play` 这些）：放在数据根旁边的 `<数据根>-fakes/` 里（不放进数据根：
/// 空着当数据根的测试，多一样东西核心就认不出它），被调了把程序名和参数记进同一个目录的 `notify.log`（[`notify_log`]），一次一行。
/// 交回放它们的目录。
pub fn fake_notifiers(home: &Path) -> PathBuf {
    let dir = fakes_dir(home);
    std::fs::create_dir_all(&dir).expect("建得了假程序目录");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let log = notify_log(home);
        for name in [
            "notify-send",
            "osascript",
            "pw-play",
            "paplay",
            "canberra-gtk-play",
            "aplay",
            "afplay",
        ] {
            let file = dir.join(name);
            let body = format!(
                "#!/bin/sh\nprintf '%s %s\\n' '{name}' \"$*\" >> '{}'\n",
                log.display()
            );
            std::fs::write(&file, body).expect("写得进假程序");
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
                .expect("可执行");
        }
    }
    dir
}

/// 放假程序的目录：数据根旁边，名字接 `-fakes`。
pub(super) fn fakes_dir(home: &Path) -> PathBuf {
    let name = home
        .file_name()
        .map_or_else(|| "home".to_string(), |n| n.to_string_lossy().into_owned());
    home.with_file_name(format!("{name}-fakes"))
}

/// 假的系统通知、放音程序记下的调用（[`fake_notifiers`]）。
pub fn notify_log(home: &Path) -> PathBuf {
    fakes_dir(home).join("notify.log")
}
