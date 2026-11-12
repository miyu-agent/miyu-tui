//! 核心可用内存不到 5% 时新建、载入会话当场拒（核心 MEM-1，错误码 `low_memory`，`message` 照握手的语言写好了）：终端原样把
//! 核心的话写出来，不吞、不换成笼统的「出错了」。终端没有分叉会话的命令。

use crate::core::Update;
use crate::transcript::Kind;

#[test]
fn a_low_memory_refusal_shows_the_cores_own_words() {
    let (mut app, _sent) = crate::app::test_support::app();
    let said = "可用内存只剩 3%（480 MiB），先不开新会话。";
    app.core(Update::Refused {
        reason: Some("low_memory".into()),
        message: said.into(),
    });
    let shown = app
        .talk
        .transcript
        .entries
        .iter()
        .find(|e| e.kind == Kind::Error)
        .map(|e| e.text.clone())
        .expect("写了一行");
    assert!(shown.contains(said), "{shown}");
}

#[test]
fn a_new_session_refused_for_low_memory_gives_back_the_words() {
    // 开新会话发第一句时被拒（连接那头交 `Unsent`）：那句放回输入框，核心的话照样写出来。
    let (mut app, _sent) = crate::app::test_support::app();
    let said = "可用内存只剩 3%（480 MiB），先不开新会话。";
    app.core(Update::Unsent {
        reason: Some("low_memory".into()),
        message: said.into(),
    });
    assert!(
        app.talk
            .transcript
            .entries
            .iter()
            .any(|e| e.kind == Kind::Error && e.text.contains(said))
    );
}
