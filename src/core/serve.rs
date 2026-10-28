//! 在一条连接上收发（蓝图 `tui.md`「连核心」「后台命令、子代理和侧边栏」「切进子会话」）：命令对着主会话，或者切进了
//! 的子会话；推来的照会话分，主会话的直接交给界面，另外订阅着的包成 [`Update::Elsewhere`]。

use std::collections::HashMap;
use std::io;

use serde_json::json;
use tokio::sync::mpsc;

use super::asides;
use super::awaiting::Awaiting;
use super::connect::{create, subscribe};
use super::link::Link;
use super::request::request;
use super::rpc::Rpc;
use super::spawn::Served;
use super::{Command, Update, page, upload};

/// 在一条连接上收发，直到界面关了或者连接断了。
pub(super) async fn serve(
    rpc: &mut Rpc,
    link: &mut Link,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    notify: &impl Fn(Update) -> bool,
) -> Served {
    let mut awaiting: HashMap<String, Awaiting> = HashMap::new();
    // 主会话：启动时进最近的那个、`--resume` 的，最新一页还没读到就断了的，先读最新一页，再带 `after` 订阅（「会话列表」
    // 第 5、8 条）；重连的带 `after` 接着补，断开期间错过的补回来（「连核心」第 7 条，review B7）。排在最前面发：核心照收到的
    // 先后答，这一页回来就带 `after` 订阅，订阅排在读配置、人格、给人看的字这些前面（2026-10-15 核心拆冷启动：原来这个会话的
    // 订阅排在它们后面，打开会话多等约 7 毫秒；首页、正文先照记下的样子画，读配置晚一点不碍事）。
    if let Some(main) = link.main.clone() {
        let sent = if link.reread_main(&main) {
            page::latest(rpc, link, &main)
                .await
                .map(|id| (id, Awaiting::Page(main)))
        } else {
            super::switch::replay(rpc, link, &main)
                .await
                .map(|id| (id, Awaiting::Rejoin(main)))
        };
        match sent {
            Ok((id, kind)) => awaiting.insert(id, kind),
            Err(_) => return Served::Lost,
        };
    }
    // 订阅配置、会话列表、软件包、给人的通知几条流（`streams.rs`）。
    if super::streams::follow(rpc, &mut awaiting).await.is_err() {
        return Served::Lost;
    }
    // 界面块照记着的再订阅（「界面块」第 8 条）：重连以前屏上有的，接着推。
    if super::widgets::rejoin(rpc, link, &mut awaiting)
        .await
        .is_err()
    {
        return Served::Lost;
    }
    // 重连以后，另外订阅着的照旧订阅上，从读到的那一条往后补（「连核心」第 7 条）。
    for session in link.watched.clone() {
        match watch(rpc, &session, link.seen.get(&session).copied()).await {
            Ok(id) => awaiting.insert(id, Awaiting::Watch(session)),
            Err(_) => return Served::Lost,
        };
    }
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { return Served::Quit };
                // 不对着会话的请求（列会话、置顶、删、模型资料、给人看的字、写配置、读输出）：照 `asides.rs` 的表发。
                if let Some((method, params, kind)) = asides::request(&command) {
                    let sent = rpc.send(method, params).await;
                    match sent {
                        Ok(id) => {
                            if let Some(kind) = kind {
                                awaiting.insert(id, kind);
                            }
                        }
                        Err(_) => return Served::Lost,
                    }
                    continue;
                }
                // 读一个 blob：一段段读，读完存成文件（`links.rs`）。
                if let Command::FetchBlob(blob) = &command {
                    if super::links::fetch(rpc, blob, &mut awaiting).await.is_err() {
                        return Served::Lost;
                    }
                    continue;
                }
                match command {
                    // 懒着开（施工会话 09-30 建议）：等第一句话再开，连按几下不留空会话。旧的有任务在跑的照样订阅着。
                    Command::New { keep } => {
                        link.viewing = None;
                        link.pending_cwd = None;
                        if let Some(old) = link.main.take() {
                            if keep {
                                link.watched.insert(old);
                            } else {
                                link.forget(&old);
                                if unsubscribe(rpc, &old).await.is_err() {
                                    return Served::Lost;
                                }
                            }
                        }
                        continue;
                    }
                    Command::Watch(session) => {
                        let fresh = Some(&session) != link.main.as_ref() && link.watched.insert(session.clone());
                        if fresh {
                            match watch(rpc, &session, None).await {
                                Ok(id) => awaiting.insert(id, Awaiting::Watch(session)),
                                Err(_) => return Served::Lost,
                            };
                        }
                        continue;
                    }
                    Command::Unwatch(session) => {
                        link.forget(&session);
                        let gone = link.watched.remove(&session) && Some(&session) != link.main.as_ref();
                        if gone && unsubscribe(rpc, &session).await.is_err() {
                            return Served::Lost;
                        }
                        continue;
                    }
                    Command::View(session) => {
                        link.viewing = session;
                        continue;
                    }
                    Command::Widgets(view) => {
                        if super::widgets::follow(rpc, link, view, &mut awaiting).await.is_err() {
                            return Served::Lost;
                        }
                        continue;
                    }
                    // 人打的 `/dream`：记着下一条斜杠命令交出去以前先订阅；整理完了退订（`memory.rs`）。
                    Command::Dream(follow) => {
                        link.dream_next = follow;
                        if !follow && super::memory::follow(rpc, None).await.is_err() {
                            return Served::Lost;
                        }
                        continue;
                    }
                    // 更早的一页：没按页读的（核心旧）不要，告诉界面没有了。
                    Command::Older(session) => {
                        match page::older(rpc, link, &session).await {
                            Some(Ok(id)) => {
                                awaiting.insert(id, Awaiting::Older(session));
                            }
                            Some(Err(_)) => return Served::Lost,
                            None => {
                                let none = Update::Older { pushes: Vec::new(), more: false, failed: None };
                                if !notify(Update::Elsewhere { session, update: Box::new(none) }) {
                                    return Served::Quit;
                                }
                            }
                        }
                        continue;
                    }
                    // 切会话：原来的还忙着的照样订阅着，不然退订；没订阅着的先读最新一页（「会话列表」第 4、5 条）。
                    Command::Open { session, keep } => {
                        if let Some(old) = link.main.clone().filter(|old| *old != session) {
                            if keep {
                                link.watched.insert(old);
                            } else {
                                link.forget(&old);
                                if unsubscribe(rpc, &old).await.is_err() {
                                    return Served::Lost;
                                }
                            }
                        }
                        link.viewing = None;
                        link.main = Some(session.clone());
                        if !link.watched.remove(&session) {
                            match page::latest(rpc, link, &session).await {
                                Ok(id) => awaiting.insert(id, Awaiting::Page(session)),
                                Err(_) => return Served::Lost,
                            };
                        }
                        continue;
                    }
                    // 还没开会话：记着，开会话时带上，当场算换成了（「配置与模型」第 1 条）。
                    Command::Configure(reference) if link.target().is_none() => {
                        link.pending_model = Some(reference.clone());
                        if !notify(Update::Configured(reference)) {
                            return Served::Quit;
                        }
                        continue;
                    }
                    // 新会话的人格、预设：记着，开会话时带上；开着会话的中途换（「新会话：人格、工作区」第 2a 条，核心 P-13：
                    // 无人格发 `null`；换成了照推来的 `session.policy_changed` 办，被拒的照一般的拒绝弹）。
                    Command::Persona(id) => {
                        match link.target() {
                            None => link.pending_persona = Some(id),
                            Some(session) => {
                                let persona = (!id.is_empty()).then_some(id);
                                let params = json!({"session": session, "persona": persona});
                                if rpc.send("session.configure", params).await.is_err() {
                                    return Served::Lost;
                                }
                            }
                        }
                        continue;
                    }
                    Command::Preset(id) => {
                        match link.target() {
                            None => link.pending_preset = Some(id),
                            Some(session) => {
                                let preset = super::preset_param(&id);
                                let params = json!({"session": session, "preset": preset});
                                if rpc.send("session.configure", params).await.is_err() {
                                    return Served::Lost;
                                }
                            }
                        }
                        continue;
                    }
                    // 新会话开在哪（核心 9-7 下）：记着，开会话时带上。
                    Command::NewWorkspace(dir) => {
                        link.pending_cwd = dir;
                        continue;
                    }
                    // 还没开会话时验一个目录：连问两次，交回结果（`workspace.rs`）。
                    Command::CheckDir { path, cwd } => {
                        let result = match super::workspace::check(rpc, &path, &cwd).await {
                            Ok(result) => result,
                            Err(_) => return Served::Lost,
                        };
                        if !notify(Update::DirChecked(result)) {
                            return Served::Quit;
                        }
                        continue;
                    }
                    Command::Level(level) if link.target().is_none() => {
                        link.pending = Some(level);
                        continue;
                    }
                    // 说第一句话、设长期目标（「长期目标」第 3 条）：先开会话。
                    Command::Send { .. } | Command::Goal(_) | Command::Slash(_)
                        if link.target().is_none() =>
                    {
                        let chosen = super::connect::Chosen {
                            model: link.pending_model.take(),
                            persona: link.pending_persona.clone(),
                            preset: link.pending_preset.clone(),
                            cwd: link.pending_cwd.clone(),
                        };
                        match fresh(rpc, &chosen).await {
                            Ok((id, joined)) => {
                                if !notify(Update::Ready(id.clone()))
                                    || !notify(Update::Limits(joined.limits))
                                    || joined.current.is_some_and(|c| !notify(Update::CurrentModel(c)))
                                    || joined.persona.is_some_and(|p| !notify(Update::SessionPersona(p)))
                                    || joined.preset.is_some_and(|p| !notify(Update::SessionPreset(p)))
                                    || joined.workspace.is_some_and(|cwd| {
                                        !notify(Update::Workspace { cwd })
                                    })
                                    || joined.snapshot.is_some_and(|s| !notify(Update::Snapshot(s)))
                                {
                                    return Served::Quit;
                                }
                                // 会话还没开时切过权限级别：先补发，再说话。
                                if let Some(level) = link.pending.take() {
                                    let mut params = level.permission();
                                    params["session"] = json!(id);
                                    if rpc.send("session.set_permission_level", params).await.is_err() {
                                        return Served::Lost;
                                    }
                                }
                                link.main = Some(id);
                                link.pending_persona = None;
                                link.pending_preset = None;
                                link.pending_cwd = None;
                            }
                            Err(Update::Disconnected) => return Served::Lost,
                            Err(update) => {
                                // 没开成：选的模型留着，下一句照它开；`/dream` 的订阅不留。
                                link.pending_model = chosen.model.clone();
                                link.dream_next = false;
                                if !notify(update) {
                                    return Served::Quit;
                                }
                                continue;
                            }
                        }
                    }
                    _ => {}
                }
                // 还没开会话时别的命令没有对象：界面那头当场说了（`app/keys.rs`）。
                let Some(session) = link.target() else { continue };
                // 人打的 `/dream`：会话开好了、命令交出去以前先订阅它的记忆流，开头那一批不漏（`memory.rs`）。
                if link.follows_dream(&command) {
                    match super::memory::follow(rpc, Some(&session)).await {
                        Ok(id) => awaiting.extend(id.map(|id| (id, Awaiting::Dream))),
                        Err(_) => return Served::Lost,
                    }
                }
                match send(rpc, command, &session, notify).await {
                    Sent::Lost => return Served::Lost,
                    Sent::Quit => return Served::Quit,
                    Sent::Skipped => {}
                    Sent::Awaiting(id, kind) => {
                        awaiting.insert(id, kind);
                    }
                }
            }
            message = rpc.next() => {
                let Some(message) = message else { return Served::Lost };
                if !super::take::take(rpc, link, &message, &mut awaiting, notify).await {
                    return Served::Quit;
                }
            }
        }
    }
}

/// 一个命令发出去的结果。
enum Sent {
    /// 发出去了，回应要另外办。
    Awaiting(String, Awaiting),
    /// 发出去了不用管，或者没发（没有对应的方法、附件传不上已经告诉界面了）。
    Skipped,
    /// 连接断了。
    Lost,
    /// 界面关了。
    Quit,
}

/// 照命令发一条请求给 `session`：带附件的先把每个文件交给核心，传不上的整句不发（「输入框」第 12 条）；重做换了附件的
/// 也一样，换成没有附件的带一个空的（「输入框」第 13 条）。
async fn send(
    rpc: &mut Rpc,
    command: Command,
    session: &str,
    notify: &impl Fn(Update) -> bool,
) -> Sent {
    let kind = match command {
        Command::Revert => Some(Awaiting::Revert),
        Command::Withdraw(_) => Some(Awaiting::Withdraw),
        Command::Unrevert => Some(Awaiting::Unrevert),
        Command::Send { .. } => Some(Awaiting::Send),
        Command::Redo { .. } => Some(Awaiting::Redo),
        Command::Recap => Some(Awaiting::Recap),
        Command::Configure(ref reference) => Some(Awaiting::Configure(reference.clone())),
        Command::Rename(ref title) => Some(Awaiting::Rename(title.clone())),
        Command::Answer {
            session: ref asker,
            ref call,
            ..
        } => Some(Awaiting::Answer(
            asker.clone().unwrap_or_else(|| session.to_string()),
            call.clone(),
        )),
        Command::Run(_) | Command::Goal(_) | Command::Workspace { .. } => {
            Some(Awaiting::CommandRun)
        }
        Command::Slash(_) => Some(Awaiting::Slash),
        Command::DreamStop => Some(Awaiting::DreamStop),
        _ => None,
    };
    // 回答里贴的图：传不上照样交回答、不带图，告诉界面一声（抽屉已经关了）。
    let answer_files = match &command {
        Command::Answer { files, .. } if !files.is_empty() => {
            let paths: Vec<_> = files.iter().map(|(_, f)| f.clone()).collect();
            match upload::attach(rpc, &paths).await {
                Ok(got) => Some(
                    files
                        .iter()
                        .map(|(slot, _)| *slot)
                        .zip(got)
                        .collect::<Vec<_>>(),
                ),
                Err(Update::Disconnected) => return Sent::Lost,
                Err(Update::Refused { message, .. }) => {
                    if !notify(Update::AnswerFilesLost(message)) {
                        return Sent::Quit;
                    }
                    None
                }
                Err(update) => {
                    if !notify(update) {
                        return Sent::Quit;
                    }
                    None
                }
            }
        }
        _ => None,
    };
    let files = match &command {
        Command::Send { files, .. } if !files.is_empty() => Some(files.clone()),
        Command::Redo { files, .. } => files.clone(),
        _ => None,
    };
    let mut attachments = None;
    if let Some(files) = files {
        match upload::attach(rpc, &files).await {
            Ok(got) => attachments = Some(got),
            Err(Update::Disconnected) => return Sent::Lost,
            // 传不上：这一句没发出去（「输入框」第 12 条）。
            Err(Update::Refused { reason, message }) => {
                let told = notify(Update::Unsent { reason, message });
                return if told { Sent::Skipped } else { Sent::Quit };
            }
            Err(update) => {
                return if notify(update) {
                    Sent::Skipped
                } else {
                    Sent::Quit
                };
            }
        }
    }
    let Some((method, mut params)) = request(command, session) else {
        return Sent::Skipped;
    };
    if let Some(attachments) = attachments {
        params["attachments"] = json!(attachments);
    }
    if let Some(files) = answer_files {
        place_answer_files(&mut params, files);
    }
    match (rpc.send(method, params).await, kind) {
        (Err(_), _) => Sent::Lost,
        (Ok(id), Some(kind)) => Sent::Awaiting(id, kind),
        (Ok(_), None) => Sent::Skipped,
    }
}

/// `/new` 以后的第一句话：开会话、订阅。被拒的（模型、人格、预设不在了这类）是这一句没发出去，照「没发出去」办：字放回
/// 输入框、弹核心的原话（review A5，2026-10-11：原来当成连不上核心，连接好好的却什么都发不出去）。
async fn fresh(
    rpc: &mut Rpc,
    chosen: &super::connect::Chosen,
) -> Result<(String, super::connect::Joined), Update> {
    let unsent = |update| match update {
        Update::Failed(message) => Update::Unsent {
            reason: None,
            message,
        },
        other => other,
    };
    let id = create(rpc, chosen).await.map_err(unsent)?;
    let joined = subscribe(rpc, &id).await.map_err(unsent)?;
    Ok((id, joined))
}

/// 订阅另一个会话，不等回应（回应里的限额由 `take` 交给界面）。重连时带 `after`：从读到的那一条往后补。
async fn watch(rpc: &mut Rpc, session: &str, after: Option<u64>) -> io::Result<String> {
    let mut params = json!({"session": session, "stream": "events"});
    if let Some(after) = after {
        params["after"] = json!(after);
    }
    rpc.send("subscribe", params).await
}

/// 退订一个会话。
async fn unsubscribe(rpc: &mut Rpc, session: &str) -> io::Result<String> {
    let params = json!({"session": session, "stream": "events"});
    rpc.send("unsubscribe", params).await
}

/// 回答里传上去的图放进 `session.answer` 的参数：`Some(i)` 的进第 `i` 道回答的 `attachments`，`None` 的进外面的（拒绝）。
fn place_answer_files(
    params: &mut serde_json::Value,
    files: Vec<(Option<usize>, serde_json::Value)>,
) {
    for (slot, attachment) in files {
        let into = match slot {
            Some(i) => &mut params["answers"][i]["attachments"],
            None => &mut params["attachments"],
        };
        if !into.is_array() {
            *into = json!([]);
        }
        if let Some(list) = into.as_array_mut() {
            list.push(attachment);
        }
    }
}

#[cfg(test)]
mod tests;
