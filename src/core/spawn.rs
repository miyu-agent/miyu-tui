//! 起一条和核心说话的线程：连上、断了照退避重连，界面的命令照顺序发（从 `mod.rs` 分出来）。

use super::*;

/// 起一个线程去连核心。`reconnect` 是连不上时隔多久再试（`layout.json` 的 `reconnect_ms`）；`notify` 把消息
/// 交给界面，界面那头关了就交回 `false`，这边跟着停。
pub fn spawn(
    reconnect: [u64; 2],
    start: Start,
    notify: impl Fn(Update) -> bool + Send + 'static,
) -> Core {
    let (commands, receiver) = mpsc::unbounded_channel();
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        match runtime {
            Ok(runtime) => runtime.block_on(run(receiver, Backoff::new(reconnect), start, &notify)),
            Err(e) => {
                notify(Update::Failed(e.to_string()));
            }
        }
    });
    Core { commands }
}

/// 一条连接用到头了：界面关了，或者连接断了。
pub(super) enum Served {
    Quit,
    Lost,
}

/// 连上，然后收发；断了就重连，接着原来那个会话（蓝图「连核心」第 7 条）。主会话怎么接上（读最新一页、带 `after`
/// 补）是收发那一头的事（`serve.rs`）。
async fn run(
    mut commands: mpsc::UnboundedReceiver<Command>,
    mut wait: Backoff,
    start: Start,
    notify: &impl Fn(Update) -> bool,
) {
    let recent = start == Start::Usual;
    let resume = match start {
        Start::Resume(id) => Some(id),
        Start::Usual | Start::Bare => None,
    };
    // 主会话、另外订阅着的、命令对着哪个：断了重连也记着（`link.rs`）。
    let mut link = super::link::Link::default();
    // 启动时进哪个会话只在头一次连上时看（「会话列表」第 8 条）；之后重连、`/new` 照旧。
    let mut first = true;
    loop {
        // 连不上一直试；这期间界面发的命令在通道里排着，连上再发（第 1、7 条）。
        let (mut rpc, account) = loop {
            match connect().await {
                Ok(connected) => break connected,
                Err(update) => {
                    if !notify(update) {
                        return;
                    }
                    tokio::time::sleep(wait.next()).await;
                }
            }
        };
        wait.reset();
        if !notify(Update::Account(account)) {
            return;
        }
        // 还没有机主：只能注册，进哪个会话等注册完、重连上再看（核心这时别的都回 `setup_first`）。
        let said = if !account.setup && std::mem::take(&mut first) {
            match entered(&mut rpc, resume.clone(), recent).await {
                // 进了已有的会话：先读最新一页再带 `after` 订阅；会话编号先告诉界面。
                Some(id) => {
                    link.main = Some(id.clone());
                    link.replay_main = true;
                    notify(Update::Ready(id))
                }
                None => notify(Update::Reconnected),
            }
        } else {
            notify(Update::Reconnected)
        };
        if !said {
            return;
        }
        match serve::serve(&mut rpc, &mut link, &mut commands, notify).await {
            Served::Quit => return,
            Served::Lost if !notify(Update::Disconnected) => return,
            Served::Lost => {}
        }
    }
}

/// 头一次连上时进哪个已有的会话：`--resume` 指定的（不验证：读最新一页时核心说不存在的，照「会话不存在」办，不回退到
/// 最近的那个）；配置 `ui.startup` 是 `recent` 的进最近的那个，一个都没有的照样等第一句话。都不是的不进，和 `/new`
/// 一样等第一句话时才开（蓝图「连核心」第 4 条：没说话就退出的不留空会话）。
///
/// 原来 `--resume` 先订阅一次验证、订阅一直留着：读最新一页的回应到达之前这条订阅推来的事件先把读到的序号推到最新，
/// 页里的事件几乎全被当成读过的丢掉（review B2，2026-10-11）。
async fn entered(rpc: &mut Rpc, resume: Option<String>, recent: bool) -> Option<String> {
    if resume.is_some() {
        return resume;
    }
    if !recent || !switch::wants_recent(rpc).await {
        return None;
    }
    let list = rpc
        .call("session.list", serde_json::json!({"dream": false}))
        .await
        .ok()?;
    switch::recent(&list)
}
