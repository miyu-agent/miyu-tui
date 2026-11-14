//! 交给系统弹的通知（蓝图 `tui.md`「系统通知」第 4 条）：别的终端、tmux、herdr 里用。Linux 用 `notify-send`，
//! macOS 用 `osascript`，Windows 用自带的 Windows PowerShell 调 Windows 运行时的 Toast（2026-11-15 三平台摸底：原来 Windows
//! 也去调 `notify-send`，拉不起来什么都不弹）。丢出去不等，另起一个线程收尸；拉不起来什么都不做。

use std::process::{Command, Stdio};

/// 这一条通知要跑的命令：程序和参数。
/// 有图标文件的（第 3 条），`notify-send` 用 `-i` 带上；`osascript` 带不了图标。
pub fn command(
    app: &str,
    title: &str,
    body: &str,
    icon: Option<&std::path::Path>,
) -> (String, Vec<String>) {
    if cfg!(target_os = "macos") {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            quoted(body),
            quoted(title)
        );
        ("osascript".into(), vec!["-e".into(), script])
    } else if cfg!(windows) {
        ("powershell".into(), toast_args(title, body))
    } else {
        let mut args = vec!["-a".into(), app.to_string()];
        if let Some(icon) = icon {
            args.push("-i".into());
            args.push(icon.display().to_string());
        }
        args.extend(["--".into(), title.to_string(), body.to_string()]);
        ("notify-send".into(), args)
    }
}

/// 测试里不许真弹系统通知、真放声音（core1 11-04：伪终端测试起的界面走了系统通知，真调了 `notify-send`，弹到了项目主人的桌面
/// 上）：单元测试里、测具设了 `MIYU_TUI_FAKES`（假程序的目录）的界面里，要拉起的程序只许在临时目录、测试的数据根里；照 `PATH`
/// 找到的是别处的（真的系统程序）当场崩，测试跟着失败。找不到的不管（照样拉不起来）。
///
/// Windows 上不拦也不崩，交回 `false`、不拉起：那里先找 System32 再找 `PATH`，测具的假程序排不到前面，`powershell` 又一定在，
/// 测试里一律不弹不响（2026-11-15 给 Windows 补系统通知、提示音时定）。别处交回 `true`。
pub fn guard(program: &str) -> bool {
    if !cfg!(test) && std::env::var_os("MIYU_TUI_FAKES").is_none() {
        return true;
    }
    if cfg!(windows) {
        return false;
    }
    let Some(found) = which(program) else {
        return true;
    };
    let real = |p: &std::path::Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let found = real(&found);
    let mut allowed = vec![real(&std::env::temp_dir())];
    allowed.extend(std::env::var_os("MIYU_HOME").map(|h| real(std::path::Path::new(&h))));
    assert!(
        allowed.iter().any(|dir| found.starts_with(dir)),
        "测试里要拉起真的 {}（{}）：系统通知、放音要用测具的假程序（MIYU_TUI_FAKES）",
        program,
        found.display()
    );
    true
}

/// 照 `PATH` 找程序（带路径的照它）；找不到的是 `None`。
fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(program);
    if path.components().count() > 1 {
        return path.is_file().then(|| path.to_path_buf());
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|dir| {
            let exe = dir.join(program);
            [exe.with_extension("exe"), exe]
        })
        .find(|candidate| candidate.is_file())
}

/// Windows 上弹 Toast 的 PowerShell 参数：照「两行字」的模板写标题、正文，挂在 Windows PowerShell 自己的程序编号下（没登记过的
/// 编号 Windows 10 起不弹；不装第三方模块，`pwsh` 7 认不了 Windows 运行时的类型，所以用系统自带的 `powershell` 5.1）。字放进
/// 单引号字串，单引号成对写。
pub fn toast_args(title: &str, body: &str) -> Vec<String> {
    const APP: &str =
        r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe";
    let script = format!(
        "$m=[Windows.UI.Notifications.ToastNotificationManager,Windows.UI.Notifications,ContentType=WindowsRuntime];\
         $x=$m::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02);\
         $t=$x.GetElementsByTagName('text');\
         [void]$t.Item(0).AppendChild($x.CreateTextNode('{}'));\
         [void]$t.Item(1).AppendChild($x.CreateTextNode('{}'));\
         $m::CreateToastNotifier('{APP}').Show([Windows.UI.Notifications.ToastNotification]::new($x))",
        single(title),
        single(body)
    );
    [
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ]
    .map(str::to_string)
    .to_vec()
}

/// PowerShell 单引号字串里，单引号成对写。
fn single(text: &str) -> String {
    text.replace('\'', "''")
}

/// AppleScript 的字符串里，反斜杠和引号要转义。
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 跑一个命令，不等它：输入输出接空，另起一个线程等它退出（不留僵尸进程）。拉不起来的交回 `false`。
pub fn spawn(program: &str, args: &[String]) -> bool {
    if !guard(program) {
        return false;
    }
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            std::thread::spawn(move || child.wait());
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::command;

    #[test]
    fn the_system_gets_the_title_and_the_body() {
        let (program, args) = command("Miyu", "Miyu", "出错", None);
        if cfg!(target_os = "macos") {
            assert_eq!(program, "osascript");
        } else if cfg!(windows) {
            assert_eq!(program, "powershell");
            assert!(args.last().unwrap().contains("CreateTextNode('出错')"));
            assert_eq!(args[1], "display notification \"出错\" with title \"Miyu\"");
        } else {
            assert_eq!(program, "notify-send");
            assert_eq!(args, ["-a", "Miyu", "--", "Miyu", "出错"]);
        }
    }

    /// Windows 的 Toast：标题、正文进单引号字串，单引号成对写；挂在 Windows PowerShell 的程序编号下（2026-11-15 三平台摸底）。
    #[test]
    fn the_windows_toast_quotes_its_words() {
        let args = super::toast_args("O'Neil 的会话", "回答'完了");
        assert_eq!(
            args[..5],
            [
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command"
            ]
        );
        let script = &args[5];
        assert!(
            script.contains("CreateTextNode('O''Neil 的会话')"),
            "{script}"
        );
        assert!(script.contains("CreateTextNode('回答''完了')"), "{script}");
        assert!(
            script.contains("ToastText02")
                && script.contains(r"WindowsPowerShell\v1.0\powershell.exe"),
            "{script}"
        );
    }

    #[test]
    fn quotes_in_applescript_are_escaped() {
        assert_eq!(super::quoted(r#"说"好"\"#), r#"说\"好\"\\"#);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn notify_send_gets_the_icon_file() {
        let icon = std::path::Path::new("/tmp/miyu.png");
        let (program, args) = command("Miyu", "修登录页", "回答完了", Some(icon));
        assert_eq!(program, "notify-send");
        assert_eq!(
            args,
            [
                "-a",
                "Miyu",
                "-i",
                "/tmp/miyu.png",
                "--",
                "修登录页",
                "回答完了"
            ]
        );
    }
}
