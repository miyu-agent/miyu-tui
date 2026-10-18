//! 用程序开的退路（蓝图「展示」第 7 条）：Linux 上照假的 `xdg-mime`、`gio`、浏览器和桌面文件走几条路。

#[cfg(all(unix, not(target_os = "macos")))]
mod linux {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use crate::launch::{Env, Launched, launch};

    /// 一套假的：`bin` 里放假程序（`xdg-mime query filetype` 说 `filetype`、`query default` 说 `default`，空的是没有；
    /// 别的把名字和参数记进 `log`），`share/applications` 里放桌面文件（名字、写不写 `Terminal=true`）。
    struct Fake {
        dir: PathBuf,
        log: PathBuf,
    }

    impl Fake {
        fn new(
            tag: &str,
            programs: &[&str],
            filetype: &str,
            default: &str,
            desktops: &[(&str, bool)],
        ) -> Fake {
            let dir =
                std::env::temp_dir().join(format!("miyu-launch-{tag}-{}", std::process::id()));
            drop(std::fs::remove_dir_all(&dir));
            let bin = dir.join("bin");
            let apps = dir.join("share/applications");
            std::fs::create_dir_all(&bin).unwrap();
            std::fs::create_dir_all(&apps).unwrap();
            let log = dir.join("log");
            std::fs::write(&log, "").unwrap();
            for name in programs {
                let body = match *name {
                    "xdg-mime" => format!(
                        "#!/bin/sh\ncase \"$2\" in\n  filetype) printf '%s' '{filetype}' ;;\n  \
                         default) printf '%s' '{default}' ;;\nesac\n"
                    ),
                    _ => format!("#!/bin/sh\necho '{name}' \"$@\" >> '{}'\n", log.display()),
                };
                let file = bin.join(name);
                std::fs::write(&file, body).unwrap();
                std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            for (name, terminal) in desktops {
                let body =
                    format!("[Desktop Entry]\nName={name}\nExec={name} %U\nTerminal={terminal}\n");
                std::fs::write(apps.join(name), body).unwrap();
            }
            Fake { dir, log }
        }

        fn env(&self, browser: Option<&str>) -> Env {
            Env {
                path: Some(self.dir.join("bin").into()),
                browser: browser.map(Into::into),
                data_dirs: Some(vec![self.dir.join("share")]),
            }
        }

        fn dir(&self) -> &Path {
            &self.dir
        }

        fn log(&self) -> &Path {
            &self.log
        }

        /// 等假程序记下一行（打开它的程序在后台写）。
        fn logged(&self) -> String {
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let text = std::fs::read_to_string(&self.log).unwrap_or_default();
                if !text.is_empty() || Instant::now() > end {
                    return text.trim().to_string();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            drop(std::fs::remove_dir_all(&self.dir));
        }
    }

    #[test]
    fn the_default_program_is_launched_from_its_desktop_file() {
        let fake = Fake::new(
            "default",
            &["xdg-mime", "gio", "xdg-open"],
            "text/markdown",
            "typora.desktop",
            &[("typora.desktop", false)],
        );
        let got = launch(Path::new("/tmp/a.md"), "text/plain", &fake.env(None));
        assert_eq!(got, Launched::Default);
        let desktop = fake.dir().join("share/applications/typora.desktop");
        assert_eq!(
            fake.logged(),
            format!("gio launch {} /tmp/a.md", desktop.display()),
            "不交给 xdg-open 自己认"
        );
    }

    #[test]
    fn the_kind_is_asked_for_the_file_itself() {
        // 2026-10-18 项目主人验：核心认不出的给 application/octet-stream，Word 文档就被交给了别的程序；
        // 照文件本身问，问到的才是它的种类。
        let fake = Fake::new(
            "filetype",
            &["xdg-mime", "gio", "xdg-open"],
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "wordcraft.desktop",
            &[("wordcraft.desktop", false)],
        );
        let got = launch(
            Path::new("/tmp/a.docx"),
            "application/octet-stream",
            &fake.env(None),
        );
        assert_eq!(got, Launched::Default);
        let desktop = fake.dir().join("share/applications/wordcraft.desktop");
        assert_eq!(
            fake.logged(),
            format!("gio launch {} /tmp/a.docx", desktop.display())
        );
    }

    #[test]
    fn the_kind_the_core_said_is_used_when_the_file_says_nothing() {
        let fake = Fake::new(
            "fallback",
            &["xdg-mime", "gio", "xdg-open"],
            "",
            "imv.desktop",
            &[("imv.desktop", false)],
        );
        let got = launch(Path::new("/tmp/a.png"), "image/png", &fake.env(None));
        assert_eq!(got, Launched::Default);
        let desktop = fake.dir().join("share/applications/imv.desktop");
        assert_eq!(
            fake.logged(),
            format!("gio launch {} /tmp/a.png", desktop.display())
        );
    }

    #[test]
    fn a_default_that_needs_a_terminal_is_launched_too() {
        // 2026-10-18 项目主人验：gio launch 自己套一个终端窗口，vim 这样的程序照样开得起来；以前当没有默认程序。
        let fake = Fake::new(
            "terminal",
            &["xdg-mime", "gio", "chromium"],
            "text/plain",
            "vim.desktop",
            &[("vim.desktop", true)],
        );
        let got = launch(Path::new("/tmp/a.md"), "text/plain", &fake.env(None));
        assert_eq!(got, Launched::Default);
        let desktop = fake.dir().join("share/applications/vim.desktop");
        assert_eq!(
            fake.logged(),
            format!("gio launch {} /tmp/a.md", desktop.display()),
            "不交浏览器"
        );
    }

    #[test]
    fn without_a_default_browser_from_the_environment_opens_it() {
        let fake = Fake::new("browser-env", &["xdg-mime", "mybrowser"], "", "", &[]);
        let got = launch(
            Path::new("/tmp/b.md"),
            "text/markdown",
            &fake.env(Some("mybrowser --new %s")),
        );
        assert_eq!(got, Launched::Browser);
        assert_eq!(fake.logged(), "mybrowser --new /tmp/b.md");
    }

    #[test]
    fn nothing_to_open_it_with_says_so() {
        let fake = Fake::new("nothing", &["xdg-mime"], "", "", &[]);
        assert_eq!(
            launch(
                Path::new("/tmp/a.bin"),
                "application/octet-stream",
                &fake.env(None)
            ),
            Launched::Nothing
        );
        assert_eq!(std::fs::read_to_string(fake.log()).unwrap(), "");
    }
}
