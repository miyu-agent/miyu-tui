//! 用程序开的退路（蓝图「展示」第 7 条）：Linux 上照假的 `xdg-mime`、`xdg-open`、浏览器走三条路。

#[cfg(all(unix, not(target_os = "macos")))]
mod linux {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use crate::launch::{Env, Launched, launch};

    /// 一个只放了假程序的目录：每个程序把自己的名字和参数记进 `log`。`mime` 是 `xdg-mime` 说的默认程序（空的是没有）。
    fn fake(tag: &str, programs: &[&str], mime: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("miyu-launch-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        std::fs::write(&log, "").unwrap();
        for name in programs {
            let body = if *name == "xdg-mime" {
                format!("#!/bin/sh\nprintf '%s' '{mime}'\n")
            } else {
                format!("#!/bin/sh\necho '{name}' \"$@\" >> '{}'\n", log.display())
            };
            let file = dir.join(name);
            std::fs::write(&file, body).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        (dir, log)
    }

    /// 等假程序记下一行（不等打开它的程序，它在后台写）。
    fn logged(log: &Path) -> String {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let text = std::fs::read_to_string(log).unwrap_or_default();
            if !text.is_empty() || Instant::now() > end {
                return text;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_default_program_comes_first() {
        let (dir, log) = fake(
            "default",
            &["xdg-mime", "xdg-open", "firefox"],
            "viewer.desktop",
        );
        let env = Env {
            path: Some(dir.clone().into()),
            browser: None,
        };
        let got = launch(Path::new("/tmp/a.html"), "text/html", &env);
        assert_eq!(got, Launched::Default);
        assert_eq!(logged(&log).trim(), "xdg-open /tmp/a.html");
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn without_a_default_program_a_browser_opens_it() {
        let (dir, log) = fake("browser", &["xdg-mime", "xdg-open", "chromium"], "");
        let env = Env {
            path: Some(dir.clone().into()),
            browser: None,
        };
        let got = launch(Path::new("/tmp/a.md"), "text/markdown", &env);
        assert_eq!(got, Launched::Browser);
        assert_eq!(logged(&log).trim(), "chromium /tmp/a.md", "xdg-open 不碰");
        // `$BROWSER` 写了的照它，`%s` 换成路径。
        let (dir2, log2) = fake("browser-env", &["xdg-mime", "mybrowser"], "");
        let env = Env {
            path: Some(dir2.clone().into()),
            browser: Some("mybrowser --new %s".into()),
        };
        assert_eq!(
            launch(Path::new("/tmp/b.md"), "text/markdown", &env),
            Launched::Browser
        );
        assert_eq!(logged(&log2).trim(), "mybrowser --new /tmp/b.md");
        drop(std::fs::remove_dir_all(&dir));
        drop(std::fs::remove_dir_all(&dir2));
    }

    #[test]
    fn nothing_to_open_it_with_says_so() {
        let (dir, log) = fake("nothing", &["xdg-mime"], "");
        let env = Env {
            path: Some(dir.clone().into()),
            browser: None,
        };
        assert_eq!(
            launch(Path::new("/tmp/a.bin"), "application/octet-stream", &env),
            Launched::Nothing
        );
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "");
        drop(std::fs::remove_dir_all(&dir));
    }
}
