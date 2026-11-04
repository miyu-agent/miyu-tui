use std::path::{Path, PathBuf};

use super::StartRefusal;

/// 一个假的核心安装：`<dir>/miyu` 是程序，旁边 `resources/core/human/` 下是中文、英文两份。
fn installed(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-start-words-{name}-{}", std::process::id()));
    let human = dir.join("resources/core/human");
    std::fs::create_dir_all(&human).unwrap();
    std::fs::write(
        human.join("zh.json"),
        r#"{"said": {"start/data_root_old_layout": "{path} 是旧版的数据根"}}"#,
    )
    .unwrap();
    std::fs::write(
        human.join("en.json"),
        r#"{"said": {"start/data_root_old_layout": "{path} is an old data root"}}"#,
    )
    .unwrap();
    std::fs::write(dir.join("miyu"), b"").unwrap();
    dir
}

fn refusal(code: &str, bin: Option<&Path>) -> StartRefusal {
    StartRefusal {
        code: code.into(),
        reason: "核心的原话".into(),
        root: PathBuf::from("/home/me/.miyu"),
        bin: bin.map(Path::to_path_buf),
    }
}

#[test]
fn a_known_refusal_is_said_in_the_interface_language_from_the_core_resources() {
    // 设了 `MIYU_RESOURCES` 的照它找，这里量的是照核心程序找。
    if std::env::var_os("MIYU_RESOURCES").is_some() {
        return;
    }
    let dir = installed("known");
    let bin = dir.join("miyu");
    let old = refusal("data_root_old_layout", Some(&bin));
    assert_eq!(old.say("zh"), "/home/me/.miyu 是旧版的数据根");
    assert_eq!(old.say("en"), "/home/me/.miyu is an old data root");
    // 没有这种语言的照英文。
    assert_eq!(old.say("ja"), "/home/me/.miyu is an old data root");
    // 资源里没有这一句的照原话。
    assert_eq!(
        refusal("data_root_not_ours", Some(&bin)).say("zh"),
        "核心的原话"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn without_the_core_resources_the_core_words_stand() {
    if std::env::var_os("MIYU_RESOURCES").is_some() {
        return;
    }
    let gone = Path::new("/nonexistent/miyu-start-words/miyu");
    assert_eq!(
        refusal("data_root_old_layout", Some(gone)).say("zh"),
        "核心的原话"
    );
}
