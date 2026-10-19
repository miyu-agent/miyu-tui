use super::*;
use crate::core::Command;

#[test]
fn only_the_next_slash_command_subscribes_once_after_dream_was_asked() {
    let mut link = Link::default();
    assert!(
        !link.follows_dream(&Command::Slash("/dream".into())),
        "没打 /dream 不订阅"
    );
    link.dream_next = true;
    // 别的命令不算，记着的留给那一条斜杠命令。
    assert!(!link.follows_dream(&Command::Recap));
    assert!(link.follows_dream(&Command::Slash("/dream".into())));
    assert!(
        !link.follows_dream(&Command::Slash("/dream".into())),
        "问过就不再记"
    );
}
