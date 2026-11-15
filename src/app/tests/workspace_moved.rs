//! 开会话时核心换了工作目录（目录太宽，核心 WN-1）：提示行照 `miyu ask` 说一句（蓝图「新会话」第 4 条）。

use super::app;
use crate::core::Update;

#[test]
fn a_moved_workspace_is_said_on_the_hint_line() {
    let (mut app, _sent) = app();
    app.core(Update::WorkspaceMoved((
        "/".into(),
        "~/.miyu/workspace/abc".into(),
    )));
    let said = app.typing.notice.as_ref().map(|n| n.text.clone());
    assert_eq!(
        said.as_deref(),
        Some("工作目录 / 范围过大，改用 ~/.miyu/workspace/abc")
    );
}
