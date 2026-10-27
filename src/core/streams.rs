//! 连上以后订的几条流（从 `serve.rs` 分出来）：配置、会话列表、软件包、给人的通知。回应都记进 `awaiting`，由 `take.rs` 认。

use std::collections::HashMap;

use super::awaiting::Awaiting;
use super::rpc::Rpc;

/// 一条条订上；发不出去的交回错（连接断了）。
pub(super) async fn follow(
    rpc: &mut Rpc,
    awaiting: &mut HashMap<String, Awaiting>,
) -> std::io::Result<()> {
    // 订阅配置流、读一次界面语言（「界面语言」）。
    awaiting.insert(super::config::follow(rpc).await?, Awaiting::UiLanguage);
    // 订阅会话列表（核心 9-5）：`/sessions`、别处来的话写来处都照这一份，变了推过来（「会话列表」）。
    awaiting.insert(
        super::sessions::follow(rpc).await?,
        Awaiting::SessionsStream,
    );
    // 订阅软件包列表，只为听工具目录变了：变了重取给人看的字（`packages.rs`，核心 9-8 七补）。
    awaiting.insert(
        super::packages::follow(rpc).await?,
        Awaiting::PackagesStream,
    );
    // 订阅给人的通知（核心 J-3 中）：定时跑完了这类，界面照「系统通知」第 8 条弹。
    awaiting.insert(super::notices::follow(rpc).await?, Awaiting::NoticesStream);
    Ok(())
}
