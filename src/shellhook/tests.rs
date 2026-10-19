use std::fs;
use std::path::{Path, PathBuf};

use super::install::{Done, Found, block, found, install_at, uninstall_at};
use super::shells::{ALL, Shell};
use super::{Options, script, words};

/// 一个测试一个空目录，用完删掉。
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("miyu-shellhook-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn options_are_named_and_unknown_ones_are_refused() {
    let parsed = Options::parse(&args(&["--shell", "fish", "--program", "/x/miyu"])).unwrap();
    assert_eq!(parsed.shell, Some(Shell::Fish));
    assert_eq!(parsed.program, Some(PathBuf::from("/x/miyu")));
    for bad in [
        &["fish"][..],
        &["--shell"],
        &["--shell", "tcsh"],
        &["--force"],
    ] {
        assert!(Options::parse(&args(bad)).is_err(), "{bad:?}");
    }
}

#[test]
fn shells_are_known_by_name_path_or_login_dash() {
    assert_eq!(Shell::named("fish"), Some(Shell::Fish));
    assert_eq!(Shell::named("/usr/bin/zsh"), Some(Shell::Zsh));
    assert_eq!(Shell::named("-bash"), Some(Shell::Bash));
    assert_eq!(Shell::named("nu"), None);
}

#[test]
fn targets_follow_xdg_and_zdotdir() {
    let env = |name: &str| match name {
        "HOME" => Some("/h".to_string()),
        "XDG_CONFIG_HOME" => Some("/c".to_string()),
        "ZDOTDIR" => Some("/z".to_string()),
        _ => None,
    };
    assert_eq!(
        Shell::Fish.target(&env),
        Some(PathBuf::from("/c/fish/conf.d/miyu-shellhook.fish"))
    );
    assert_eq!(Shell::Bash.target(&env), Some(PathBuf::from("/h/.bashrc")));
    assert_eq!(Shell::Zsh.target(&env), Some(PathBuf::from("/z/.zshrc")));
    assert_eq!(
        Shell::Fish.old_hook(&env),
        Some(PathBuf::from("/c/fish/conf.d/miyu.fish")),
        "旧版的那一份和自己的在同一个目录、名字不一样"
    );
    let bare = |name: &str| (name == "HOME").then(|| "/h".to_string());
    assert_eq!(
        Shell::Fish.target(&bare),
        Some(PathBuf::from("/h/.config/fish/conf.d/miyu-shellhook.fish"))
    );
}

#[test]
fn a_fish_file_we_did_not_write_is_never_touched() {
    let dir = Dir::new("foreign");
    let path = dir.0.join("miyu-shellhook.fish");
    fs::write(&path, "# 自己写的\n").unwrap();
    assert_eq!(
        install_at(Shell::Fish, &path, Path::new("/x/miyu")).unwrap(),
        Done::Foreign
    );
    assert!(!uninstall_at(Shell::Fish, &path).unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), "# 自己写的\n");
    assert_eq!(found(Shell::Fish, &path).unwrap(), Found::Foreign);
}

#[test]
fn fish_installs_updates_and_removes_its_own_file() {
    let dir = Dir::new("fish");
    let path = dir.0.join("conf.d/miyu-shellhook.fish");
    let program = Path::new("/opt/it's/miyu");
    assert_eq!(found(Shell::Fish, &path).unwrap(), Found::Missing);
    assert_eq!(
        install_at(Shell::Fish, &path, program).unwrap(),
        Done::Installed
    );
    assert_eq!(
        found(Shell::Fish, &path).unwrap(),
        Found::Ours(Some(program.to_path_buf())),
        "带引号的路径照原样读回来"
    );
    let other = Path::new("/new/miyu");
    assert_eq!(
        install_at(Shell::Fish, &path, other).unwrap(),
        Done::Updated
    );
    assert_eq!(
        found(Shell::Fish, &path).unwrap(),
        Found::Ours(Some(other.into()))
    );
    assert!(uninstall_at(Shell::Fish, &path).unwrap());
    assert!(!path.exists());
    assert!(!uninstall_at(Shell::Fish, &path).unwrap());
}

#[test]
fn rc_files_keep_everything_around_the_block() {
    for shell in [Shell::Bash, Shell::Zsh] {
        let dir = Dir::new(shell.name());
        let path = dir.0.join("rc");
        fs::write(&path, "alias ll='ls -l'").unwrap();
        let program = Path::new("/opt/it's/miyu");
        assert_eq!(install_at(shell, &path, program).unwrap(), Done::Installed);
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("alias ll='ls -l'\n# >>> miyu shellhook >>>"),
            "{text}"
        );
        assert_eq!(
            found(shell, &path).unwrap(),
            Found::Ours(Some(program.into()))
        );
        fs::write(&path, format!("{text}export A=1\n")).unwrap();
        assert_eq!(
            install_at(shell, &path, Path::new("/b/miyu")).unwrap(),
            Done::Updated
        );
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(
            text.matches("miyu shellhook >>>").count(),
            1,
            "换掉，不再加一段"
        );
        assert!(
            text.ends_with("<<< miyu shellhook <<<\nexport A=1\n"),
            "{text}"
        );
        assert!(uninstall_at(shell, &path).unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "alias ll='ls -l'\nexport A=1\n"
        );
        assert_eq!(found(shell, &path).unwrap(), Found::Missing);
    }
}

#[cfg(unix)]
#[test]
fn a_symlinked_rc_file_stays_a_symlink() {
    let dir = Dir::new("link");
    let real = dir.0.join("dotfiles-bashrc");
    fs::write(&real, "# mine\n").unwrap();
    let link = dir.0.join(".bashrc");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    install_at(Shell::Bash, &link, Path::new("/x/miyu")).unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&real)
            .unwrap()
            .contains("miyu shellhook")
    );
}

#[test]
fn every_script_calls_the_recorded_program_and_parses() {
    let program = Path::new("/opt/it's/miyu");
    for shell in ALL {
        let text = script::render(shell, program);
        assert!(!text.contains("@PROGRAM@"), "{}", shell.name());
        assert!(text.contains(&format!("{} shellhook", shell.quote(program))));
        assert!(text.contains("shellhook say --shell"), "{}", shell.name());
        // 机器上装着这种 shell 的，照它自己的语法检查过一遍（`-n` 只读不跑）。
        let checked = std::process::Command::new(shell.name())
            .arg("-n")
            .arg("-c")
            .arg(&text)
            .status();
        if let Ok(status) = checked {
            assert!(status.success(), "{} 的脚本语法不对", shell.name());
        }
        let installed = block(shell, program);
        if let Ok(status) = std::process::Command::new(shell.name())
            .args(["-n", "-c", &installed])
            .status()
        {
            assert!(status.success(), "{} 装进去的那一段语法不对", shell.name());
        }
    }
}

#[test]
fn the_fish_script_hands_sentences_over_without_a_flash() {
    // 旧版 2026-10-01 报：粘一段长话回车，交给她之前闪一下。清空输入行和重放包在同一段同步输出里。
    let text = script::render(Shell::Fish, Path::new("/x/miyu"));
    let hand = &text[text.find("function __miyu_hand_to_ai").unwrap()..];
    let open = hand.find("\\e[?2026h").unwrap();
    let clear = hand.find("commandline -b -- \"\"").unwrap();
    assert!(open < clear);
    assert!(text.contains("--tokens-raw"), "单行只看没展开的首词");
    assert!(text.contains("classify --shell fish"), "多行交给程序判");
}

#[test]
fn every_language_has_the_same_words() {
    let table = crate::language::LanguageTable::builtin().unwrap();
    let mut codes: Vec<&str> = words::TEXTS.iter().map(|(c, _)| *c).collect();
    codes.sort_unstable();
    let mut known = table.codes();
    known.sort_unstable();
    assert_eq!(codes, known, "和终端界面的语言一样多");
    for (code, _) in words::TEXTS {
        words::Words::of(code).unwrap();
    }
    assert_eq!(
        words::fill("{shell}：{path}", &[("shell", "fish"), ("path", "/a")]),
        "fish：/a"
    );
}
