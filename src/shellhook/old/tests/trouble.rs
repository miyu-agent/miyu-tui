//! 出了错（蓝图「旧版那一份」第 5 条）、`status` 只说不删（第 7 条）、引导拿得到那几行（第 6 条）。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::super::super::install::status_to;
use super::super::super::setup::install_in;
use super::super::super::shells::Shell;
use super::{Home, old, program, says, shown, words};

/// 家目录下每个文件（不跟链接）的内容：跑之前、跑之后比，一个字节都不许变。
fn snapshot(home: &Home) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut dirs = vec![home.root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                dirs.push(path);
            } else if meta.is_file() {
                files.insert(path.clone(), fs::read(&path).unwrap());
            } else {
                let to = fs::read_link(&path).unwrap();
                files.insert(path, to.display().to_string().into_bytes());
            }
        }
    }
    files
}

/// 把 `dir` 设成只读；root 下照样写得进去的交回 `false`（这时测不了，跳过），权限已经还原。
#[cfg(unix)]
fn read_only(dir: &std::path::Path) -> bool {
    super::set_mode(dir, 0o555);
    let probe = dir.join(".probe");
    if fs::write(&probe, "").is_ok() {
        fs::remove_file(&probe).unwrap();
        super::set_mode(dir, 0o755);
        return false;
    }
    true
}

#[cfg(unix)]
#[test]
fn a_backup_that_cannot_be_written_leaves_the_old_block_and_still_installs() {
    // .bashrc 是链接：备份放在链接所在的家目录（只读、写不进），改的是它指的 dotfiles/bashrc（写得进）。
    let home = Home::new("bak-fails", None);
    fs::create_dir_all(home.at("dotfiles")).unwrap();
    let real = home.at("dotfiles/bashrc");
    let before = format!("a\n{}", old("bash"));
    fs::write(&real, &before).unwrap();
    let link = home.at(".bashrc");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    if !read_only(&home.root) {
        return;
    }
    let env = home.env();
    let result = super::install(Shell::Bash, &link, program(), &env, &words());
    super::set_mode(&home.root, 0o755);
    let (done, said) = result.unwrap();
    assert_eq!(done, super::Done::Installed, "第 5 条：新的照装");
    assert_eq!(
        fs::read_to_string(&real).unwrap(),
        format!("{before}{}", super::block(Shell::Bash, program())),
        "第 5 条：备份写不了，旧段原样留着，这个文件只装上新段"
    );
    assert!(home.backups().is_empty());
    assert!(
        says(&said, true, &[&shown(&link)]),
        "第 5 条：说清原因、提醒旧的还在：{said:?}"
    );
    assert!(
        said.iter().filter(|s| s.trouble).count() >= 2,
        "出错一行、提醒旧的还在一行：{said:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_changed_keeps_its_block_and_the_backup_stays() {
    // .bash_profile 是链接：备份在家目录（写得进），它指的文件在只读的目录里，先写在旁边再换过去写不成。
    let home = Home::new("write-fails", None);
    fs::create_dir_all(home.at("ro")).unwrap();
    let real = home.at("ro/profile");
    let before = format!("a\n{}", old("bash"));
    fs::write(&real, &before).unwrap();
    let link = home.at(".bash_profile");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    if !read_only(&home.at("ro")) {
        return;
    }
    let (done, said) = home.install(Shell::Bash);
    super::set_mode(&home.at("ro"), 0o755);
    assert_eq!(done, super::Done::Installed, "第 5 条：新的照装");
    assert_eq!(
        fs::read_to_string(&real).unwrap(),
        before,
        "第 5 条：那一处原样不动"
    );
    let bak = home.at(".bash_profile.miyu-old-hook.bak");
    assert_eq!(fs::read_to_string(&bak).unwrap(), before, "备份成了的留着");
    assert!(
        says(&said, true, &[&shown(&bak)]),
        "第 5 条：话里说到备份留着：{said:?}"
    );
}

#[cfg(unix)]
#[test]
fn the_rc_file_that_cannot_be_changed_fails_the_install_and_names_the_backup() {
    let home = Home::new("rc-write-fails", None);
    fs::create_dir_all(home.at("ro")).unwrap();
    let real = home.at("ro/bashrc");
    let before = format!("a\n{}", old("bash"));
    fs::write(&real, &before).unwrap();
    let link = home.at(".bashrc");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    if !read_only(&home.at("ro")) {
        return;
    }
    let env = home.env();
    let result = super::install(Shell::Bash, &link, program(), &env, &words());
    super::set_mode(&home.at("ro"), 0o755);
    let bak = home.at(".bashrc.miyu-old-hook.bak");
    let error = result
        .expect_err("装新的那一段的文件写不成，install 失败")
        .to_string();
    assert!(
        error.contains(&shown(&bak)),
        "第 5 条：话里说到备份留着：{error}"
    );
    assert_eq!(fs::read_to_string(&real).unwrap(), before, "原样不动");
    assert_eq!(fs::read_to_string(&bak).unwrap(), before);
}

#[test]
fn status_names_every_old_copy_and_removes_nothing() {
    let home = Home::new("status", None);
    let profile = home.at(".profile");
    fs::write(&profile, format!("a\n{}", old("bash"))).unwrap();
    let zshrc = home.at(".zshrc");
    fs::write(&zshrc, format!("a\nb\n{}", old("zsh"))).unwrap();
    let conf_d = home.at(".config/fish/conf.d");
    fs::create_dir_all(&conf_d).unwrap();
    let fish = conf_d.join("miyu.fish");
    fs::write(&fish, "# miyu shell hook · v0.7.4 · 1a2b\n").unwrap();
    let before = snapshot(&home);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    status_to(
        &[Shell::Fish, Shell::Bash, Shell::Zsh],
        &home.env(),
        &words(),
        &mut out,
        &mut err,
    )
    .unwrap();
    let err = String::from_utf8(err).unwrap();
    assert_eq!(
        snapshot(&home),
        before,
        "第 7 条：status 只说不删，一个字节不变"
    );
    for (path, lines) in [(&profile, "2–4"), (&zshrc, "3–5")] {
        assert!(
            err.lines()
                .any(|l| l.contains(&shown(path)) && l.contains(lines)),
            "第 7 条：bash、zsh 写明哪个文件第几行：{err}"
        );
    }
    assert!(err.contains(&shown(&fish)), "第 7 条：fish 的也看：{err}");
    assert!(
        err.lines().filter(|l| !l.is_empty()).count() >= 3,
        "没装的 shell 也说旧版那一份：{err}"
    );
}

#[test]
fn the_setup_gets_the_lines_back() {
    let home = Home::new("setup", None);
    let rc = home.at(".bashrc");
    fs::write(&rc, format!("a\n{}", old("bash"))).unwrap();
    let lines = install_in("bash", &home.env(), program(), &words()).unwrap();
    let bak = home.at(".bashrc.miyu-old-hook.bak");
    assert!(bak.exists());
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&shown(&rc)) && l.contains("2–4") && l.contains(&shown(&bak))),
        "第 6 条：引导不往标准输出印，清了哪几处跟着结果交回去：{lines:?}"
    );
}
