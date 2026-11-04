//! 你用 `!` 打的命令（蓝图 `tui.md`「人自己跑命令 `!`」第 2、4 条，排期第 21 条）：结果照序号挂回那一句；排着的命令不和别的话
//! 拼成一段；补发来的照 `shell: true` 认出来。

use super::super::{Kind, Transcript};
use super::apply;
use crate::core::{Push, ShellRan};

fn ran(message: u64, exit: i64) -> Push {
    Push::ShellRan(ShellRan {
        message,
        exit: Some(exit),
        signal: None,
        duration_ms: Some(30),
        text: "out\n".into(),
        reason: None,
    })
}

#[test]
fn the_result_hangs_under_its_command_by_sequence_number() {
    let mut t = Transcript::default();
    t.shell_user("ls".into());
    apply(&mut t, vec![Push::UserMessage(3), ran(3, 0)]);
    let entry = t.entries.iter().find(|e| e.kind == Kind::User).unwrap();
    let got = entry.shell.as_ref().and_then(|s| s.ran.as_ref()).unwrap();
    assert_eq!((got.exit, got.text.as_str()), (Some(0), "out\n"));
    assert_eq!(entry.typed(), "!ls", "放回框里、记进历史时 `!` 打头");
}

#[test]
fn a_replayed_message_marked_shell_is_drawn_as_a_command() {
    let mut t = Transcript::default();
    apply(
        &mut t,
        vec![
            Push::Said {
                seq: 7,
                text: "pwd".into(),
                attached: Vec::new(),
            },
            Push::ShellAsked(7),
            ran(7, 1),
        ],
    );
    let entry = t.entries.iter().find(|e| e.seq == Some(7)).unwrap();
    assert_eq!(
        entry
            .shell
            .as_ref()
            .and_then(|s| s.ran.as_ref())
            .and_then(|r| r.exit),
        Some(1)
    );
}

#[test]
fn queued_commands_stay_separate_from_the_words_heard_with_them() {
    // 排着的几句一起被听到：普通的话拼成一段，跑命令的各自一条（结果照各自的序号挂回去）。
    let mut t = Transcript::default();
    t.user("先跑一下".into(), Vec::new());
    apply(
        &mut t,
        vec![Push::UserMessage(1), Push::TurnStarted(2, Some(1))],
    );
    t.user("一".into(), Vec::new());
    apply(&mut t, vec![Push::UserMessage(4)]);
    t.shell_user("make".into());
    apply(&mut t, vec![Push::UserMessage(5)]);
    t.user("二".into(), Vec::new());
    apply(&mut t, vec![Push::UserMessage(6)]);
    apply(&mut t, vec![Push::Heard(6)]);
    let users: Vec<(String, bool)> = t
        .entries
        .iter()
        .filter(|e| e.kind == Kind::User && !e.queued)
        .map(|e| (e.text.clone(), e.shell.is_some()))
        .collect();
    assert_eq!(
        users,
        [
            ("先跑一下".to_string(), false),
            ("一".to_string(), false),
            ("make".to_string(), true),
            ("二".to_string(), false),
        ]
    );
}
