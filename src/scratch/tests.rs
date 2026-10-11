use super::write_in;

fn dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-scratch-{tag}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    dir
}

#[test]
fn each_draft_gets_its_own_new_file() {
    let dir = dir("names");
    let a = write_in(&dir, "prompt", "一").expect("写得进去");
    let b = write_in(&dir, "prompt", "二").expect("写得进去");
    assert_ne!(a, b, "名字带随机数，不是固定的那一个");
    assert_eq!(std::fs::read_to_string(&a).expect("读得出"), "一");
    assert_eq!(std::fs::read_to_string(&b).expect("读得出"), "二");
    drop(std::fs::remove_dir_all(&dir));
}

#[cfg(unix)]
#[test]
fn only_the_owner_can_read_the_draft() {
    // review A9（2026-10-11）：原来放共享的临时目录、0644，同机别的用户读得到草稿和人格提示词。
    use std::os::unix::fs::PermissionsExt;
    let dir = dir("modes");
    let file = write_in(&dir, "persona", "人格提示词").expect("写得进去");
    let mode = |p: &std::path::Path| std::fs::metadata(p).expect("在").permissions().mode() & 0o777;
    assert_eq!(mode(&file), 0o600);
    assert_eq!(mode(&dir), 0o700);
    drop(std::fs::remove_dir_all(&dir));
}
