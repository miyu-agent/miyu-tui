use std::path::{Path, PathBuf};

use miyu_client::protocol::Said;

use super::StartRefusal;

/// 一个假的核心安装：`<dir>/miyu` 是程序，旁边 `resources/core/human/` 下是中文、英文两份。
fn installed(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-start-words-{name}-{}", std::process::id()));
    let human = dir.join("resources/core/human");
    std::fs::create_dir_all(&human).unwrap();
    std::fs::write(
        human.join("zh.json"),
        r#"{"said": {"start/data_root_old_layout": "{path} 是旧版的数据根",
            "start/log_failed": "核心无法启动：无法写入运行日志：{reason}"}}"#,
    )
    .unwrap();
    std::fs::write(
        human.join("en.json"),
        r#"{"said": {"start/data_root_old_layout": "{path} is an old data root",
            "start/log_failed": "The core couldn't start: cannot write its log: {reason}"}}"#,
    )
    .unwrap();
    std::fs::write(dir.join("miyu"), b"").unwrap();
    dir
}

/// 核心照 `StartError::said` 交的：`path`、`reason` 都填会变的那一段（核心 EN-2）。
fn refusal(code: &str, part: &str, bin: Option<&Path>) -> StartRefusal {
    StartRefusal {
        said: Said::new(format!("core/start/{code}"))
            .with("reason", part)
            .with("path", part),
        reason: "核心的原话".into(),
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
    let old = refusal("data_root_old_layout", "/home/me/.miyu", Some(&bin));
    assert_eq!(old.say("zh"), "/home/me/.miyu 是旧版的数据根");
    assert_eq!(old.say("en"), "/home/me/.miyu is an old data root");
    // 没有这种语言的照英文。
    assert_eq!(old.say("ja"), "/home/me/.miyu is an old data root");
    // 原因只是会变的那一段：整句是核心资源里的那一句（核心 EN-2）。
    let log = refusal("log_failed", "File exists (os error 17)", Some(&bin));
    assert_eq!(
        log.say("en"),
        "The core couldn't start: cannot write its log: File exists (os error 17)"
    );
    assert_eq!(
        log.say("zh"),
        "核心无法启动：无法写入运行日志：File exists (os error 17)"
    );
    // 资源里没有这一句的照原话。
    assert_eq!(
        refusal("data_root_not_ours", "/home/me/.miyu", Some(&bin)).say("zh"),
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
        refusal("data_root_old_layout", "/home/me/.miyu", Some(gone)).say("zh"),
        "核心的原话"
    );
}
