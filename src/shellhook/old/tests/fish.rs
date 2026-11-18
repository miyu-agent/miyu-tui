//! fish 的旧文件 `conf.d/miyu.fish`（蓝图「旧版那一份」第 1、4 条）。

use std::fs;

use super::super::super::install::Done;
use super::super::super::shells::Shell;
use super::{Home, says, shown};

/// 旧版给每份脚本盖的戳（`crates/miyu-base/src/shell/mod.rs`）。
const STAMPED: &str = "# miyu shell hook · v0.7.4 · 1a2b3c4d\nfunction __miyu\nend\n";

fn conf_d(home: &Home) -> std::path::PathBuf {
    let dir = home.at(".config/fish/conf.d");
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_stamped_old_file_is_renamed_to_its_backup() {
    let home = Home::new("fish-file", None);
    let old = conf_d(&home).join("miyu.fish");
    fs::write(&old, STAMPED).unwrap();
    let (done, said) = home.install(Shell::Fish);
    assert_eq!(done, Done::Installed);
    assert!(!old.exists(), "第 4 条：旧文件不在原处了");
    let bak = conf_d(&home).join("miyu.fish.miyu-old-hook.bak");
    assert_eq!(
        fs::read_to_string(&bak).unwrap(),
        STAMPED,
        "第 4 条：改名成同目录的备份，内容原样"
    );
    assert!(
        fs::read_to_string(conf_d(&home).join("miyu-shellhook.fish"))
            .unwrap()
            .starts_with("# >>> miyu shellhook >>>"),
        "新的照装"
    );
    assert!(
        says(&said, false, &[&shown(&old), &shown(&bak)]),
        "第 6 条：说清哪个文件、备份在哪：{said:?}"
    );
}

#[test]
fn a_fish_backup_name_already_taken_moves_on() {
    let home = Home::new("fish-bak-taken", None);
    let old = conf_d(&home).join("miyu.fish");
    fs::write(&old, STAMPED).unwrap();
    let taken = conf_d(&home).join("miyu.fish.miyu-old-hook.bak");
    fs::write(&taken, "keep me").unwrap();
    home.install(Shell::Fish);
    assert_eq!(fs::read_to_string(&taken).unwrap(), "keep me", "不盖");
    assert_eq!(
        fs::read_to_string(conf_d(&home).join("miyu.fish.miyu-old-hook.bak.1")).unwrap(),
        STAMPED
    );
    assert!(!old.exists());
}

#[test]
fn an_unstamped_miyu_fish_is_only_mentioned() {
    let home = Home::new("fish-foreign", None);
    let old = conf_d(&home).join("miyu.fish");
    fs::write(&old, "# my own\n").unwrap();
    let (done, said) = home.install(Shell::Fish);
    assert_eq!(done, Done::Installed);
    assert_eq!(
        fs::read_to_string(&old).unwrap(),
        "# my own\n",
        "第 4 条：有戳以外的一律不碰"
    );
    assert!(home.backups().is_empty());
    assert!(says(&said, true, &[&shown(&old)]), "照旧提醒：{said:?}");
}

#[cfg(unix)]
#[test]
fn a_link_to_a_stamped_file_loses_only_the_link() {
    let home = Home::new("fish-link", None);
    let hooks = home.at(".miyu/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let pointed = hooks.join("miyu.fish");
    fs::write(&pointed, STAMPED).unwrap();
    let link = conf_d(&home).join("miyu.fish");
    std::os::unix::fs::symlink(&pointed, &link).unwrap();
    let (_, said) = home.install(Shell::Fish);
    assert!(fs::symlink_metadata(&link).is_err(), "第 4 条：链接删掉了");
    assert_eq!(
        fs::read_to_string(&pointed).unwrap(),
        STAMPED,
        "第 4 条：它指的文件不动"
    );
    assert!(home.backups().is_empty(), "删链接不留备份");
    assert!(
        says(&said, false, &[&shown(&link), &shown(&pointed)]),
        "第 6 条：说清哪个链接、原来指到哪：{said:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_dangling_link_is_removed() {
    let home = Home::new("fish-dangling", None);
    let link = conf_d(&home).join("miyu.fish");
    let gone = home.at(".miyu/hooks/miyu.fish");
    std::os::unix::fs::symlink(&gone, &link).unwrap();
    let (_, said) = home.install(Shell::Fish);
    assert!(
        fs::symlink_metadata(&link).is_err(),
        "第 1 条：指到的文件已经不在的也认，删掉链接"
    );
    assert!(says(&said, false, &[&shown(&link)]), "{said:?}");
}

#[cfg(unix)]
#[test]
fn a_link_to_an_unstamped_file_is_left_alone() {
    let home = Home::new("fish-link-foreign", None);
    let pointed = home.at("mine.fish");
    fs::write(&pointed, "# my own\n").unwrap();
    let link = conf_d(&home).join("miyu.fish");
    std::os::unix::fs::symlink(&pointed, &link).unwrap();
    let (_, said) = home.install(Shell::Fish);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "第 1 条：指到的文件没有戳的不认，链接不动"
    );
    assert_eq!(fs::read_to_string(&pointed).unwrap(), "# my own\n");
    assert!(says(&said, true, &[&shown(&link)]), "照旧提醒：{said:?}");
}

#[test]
fn installing_bash_leaves_the_fish_file_alone() {
    let home = Home::new("fish-not-bash", None);
    let old = conf_d(&home).join("miyu.fish");
    fs::write(&old, STAMPED).unwrap();
    home.install(Shell::Bash);
    assert_eq!(
        fs::read_to_string(&old).unwrap(),
        STAMPED,
        "第 2 条：装哪种 shell 只清哪种的"
    );
}
