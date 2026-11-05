use super::super::shells::Shell;
use super::is_shell_command;

#[test]
fn commands_are_commands() {
    for line in [
        "echo hi",
        "cd /tmp",
        "# comment\necho hi",
        "for item in a b",
        "time printf hi",
    ] {
        assert!(is_shell_command(line, Shell::Fish), "{line:?}");
    }
}

/// 照 PATH、照路径找得到的：要真有 `sh`、`ls`（Windows 的 CI 上没有）。
#[cfg(unix)]
#[test]
fn programs_on_the_path_are_commands() {
    for line in [
        "FOO=bar sh -c true",
        "# comment\nls",
        "/bin/sh -c true",
        "time sh -c true",
        "sh -c \\\n  true",
    ] {
        assert!(is_shell_command(line, Shell::Fish), "{line:?}");
    }
}

#[test]
fn sentences_are_sentences() {
    for line in [
        "你觉得 a;b 是什么意思",
        "解释 <tag> 是什么",
        "第一行\n第二行",
        "# note\n解释一下这个问题",
        "time 是什么命令？",
        "this-command-probably-does-not-exist",
        "GTK_IM_MODULE=fcitx 是什么意思？",
    ] {
        assert!(!is_shell_command(line, Shell::Fish), "{line:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_path_counts_only_if_it_runs() {
    // 旧版 08-26 实测：粘两张图的路径加一句话，整段被 fish 逐行执行，图全丢了。
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("miyu-shellhook-classify-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let image = dir.join("1.png");
    std::fs::write(&image, b"png").unwrap();
    let script = dir.join("run.sh");
    std::fs::write(&script, b"#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pasted = format!(
        "{}\n{}\n把这两张图上下拼起来",
        image.display(),
        image.display()
    );
    assert!(!is_shell_command(&pasted, Shell::Fish));
    assert!(is_shell_command(
        &format!("{} --flag\nmore", script.display()),
        Shell::Fish
    ));
    assert!(!is_shell_command(
        &format!("{}/nope\nmore", dir.display()),
        Shell::Fish
    ));
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn fish_builtins_count_only_in_fish() {
    assert!(is_shell_command("string trim a", Shell::Fish));
    assert!(!is_shell_command("string trim a", Shell::Bash));
}
