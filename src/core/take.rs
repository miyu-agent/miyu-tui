//! 读进来的一条怎么办（从 `serve.rs` 分出来）：等着的回应照请求编号交给界面，推送照会话分，配置流、会话列表流另外办。

use std::collections::HashMap;

use serde_json::Value;

use super::awaiting::Awaiting;
use super::limits::Limits;
use super::link::Link;
use super::rpc::Rpc;
use super::sessions;
use super::switch;
use super::{JobOutput, Push, Report, Snapshot, Update, config};

/// 处理一条读进来的：推送、回应。交回界面还在不在。
pub(super) async fn take(
    rpc: &mut Rpc,
    link: &mut Link,
    message: &Value,
    awaiting: &mut HashMap<String, Awaiting>,
    notify: &impl Fn(Update) -> bool,
) -> bool {
    let kind = message["id"].as_str().and_then(|id| awaiting.remove(id));
    if let Some(error) = message.get("error") {
        let reason = error["data"]["reason"].as_str().map(str::to_string);
        // 用不了的斜杠命令照连接的语言说为什么（`data.why`，核心 O-6 再补），比原话具体（蓝图「长期目标」第 2 条）。扩展办
        // 命令出了错的（`command_failed`，核心 X-14 下）：`why` 是扩展的原话，接在「命令执行失败」下一行，两句都给人看。
        let said = error["message"].as_str().unwrap_or_default();
        let message = match (reason.as_deref(), error["data"]["why"].as_str()) {
            (Some("command_failed"), Some(why)) => format!("{said}\n{why}"),
            (_, Some(why)) => why.to_string(),
            (_, None) => said.to_string(),
        };
        return match kind {
            Some(Awaiting::Answer(session, call)) => notify(Update::AnswerRefused {
                session,
                call,
                reason,
                message,
            }),
            Some(Awaiting::Ask(tag)) => notify(Update::Answer {
                tag,
                result: Err(super::Refusal {
                    reason,
                    message,
                    data: error["data"].clone(),
                }),
            }),
            Some(Awaiting::Send | Awaiting::Redo) => notify(Update::Unsent { reason, message }),
            // 读不了人格（旧核心）：当没有，不弹人格框。
            Some(Awaiting::Personas) => notify(Update::Personas(Vec::new())),
            Some(Awaiting::Presets) => notify(Update::Presets(Vec::new())),
            // 读不了软件包（旧核心）：当没有，输入历史照旧放老位置。
            Some(Awaiting::Packages) => notify(Update::Packages(Vec::new())),
            Some(Awaiting::Usage(kind)) => notify(Update::UsageRows {
                kind,
                rows: Err(message),
            }),
            // 订阅不上（那个会话已经删了）：不用说。读不了配置（核心旧）：照系统语言，不用说。
            Some(
                Awaiting::Watch(_)
                | Awaiting::UiLanguage
                | Awaiting::Human
                | Awaiting::Models
                | Awaiting::SessionsStream
                | Awaiting::PackagesStream,
            ) => true,
            Some(Awaiting::Choices) => notify(Update::Choices(Vec::new())),
            Some(Awaiting::Efforts) => notify(Update::Efforts(super::EffortList::default())),
            Some(Awaiting::LinkPreview(url)) => notify(Update::LinkCard { url, card: None }),
            Some(Awaiting::Blob(blob, _)) => notify(Update::BlobSaved { blob, path: None }),
            // 画不出、太长、核心没编进 mermaid（`unknown_method`）：写源码。
            Some(Awaiting::Mermaid(source)) => notify(Update::Mermaid { source, svg: None }),
            Some(Awaiting::Files(word)) => notify(Update::Files { word, result: None }),
            // 要最新一页被拒了：核心旧的补整份，别的照补发不成办（`page.rs`）。
            Some(Awaiting::Page(session)) => {
                super::page::take_refused(rpc, link, session, (reason, message), awaiting, notify)
                    .await
            }
            // 更早的一页读不成：界面去掉顶上那一行、弹原因。
            Some(Awaiting::Older(session)) => {
                super::page::take_older(link, session, Err(message), notify)
            }
            // 切过去、重连以后订阅不上：主会话不存在了的照「会话不存在」办（review B3），别的（日志坏了）不再当它在补发，
            // 照一般的拒绝说。
            Some(Awaiting::Replay(session) | Awaiting::Rejoin(session)) => {
                link.replays.remove(&session);
                let gone = link.gone(&session, reason.as_deref());
                notify(gone.unwrap_or(Update::Refused { reason, message }))
            }
            // 核心清单里的斜杠命令被拒：另报，界面把打的那一行放回输入框（「斜杠命令列表」第 4 条）。
            Some(Awaiting::Slash) => notify(Update::SlashRefused { message }),
            // 订阅不上界面块（核心旧、不认这个流）：什么都不画，不用说（「界面块」第 8 条）。
            Some(Awaiting::Widgets) => true,
            Some(Awaiting::PackageCall) => notify(Update::PackageCallRefused { message }),
            // 读不了输出（任务没了、是子代理）：不用说，界面不再等它。
            Some(Awaiting::Output(session, job, after)) => notify(Update::Output {
                session,
                job,
                output: None,
                after,
            }),
            _ => notify(Update::Refused { reason, message }),
        };
    }
    match kind {
        Some(Awaiting::Revert | Awaiting::Unrevert) => {
            let restore = kind == Some(Awaiting::Unrevert);
            let report = Report::read(&message["result"]);
            return notify(Update::Undone { restore, report });
        }
        // 说的话、重做成了：落盘、开轮都照推送来，回应不用管。
        Some(Awaiting::Send | Awaiting::Redo) => return true,
        // 回顾：写成了的已经照推送（`session.recapped`）画过；交回上一句的核心不推，照回应画（蓝图「回顾」第 3 条）。
        Some(Awaiting::Recap) => {
            let result = &message["result"];
            let text = result["text"].as_str().unwrap_or_default();
            return !(result["cached"].as_bool() == Some(true) && !text.is_empty())
                || notify(Update::Recap(text.to_string()));
        }
        Some(Awaiting::Rename(title)) => return notify(Update::Renamed(title)),
        Some(Awaiting::Widgets) => return notify(super::widgets::listed(&message["result"])),
        // 调方法成了：回应不用看（`widgets.md`「头」第 6 条）。
        Some(Awaiting::PackageCall) => return true,
        Some(Awaiting::Packages) => {
            return notify(Update::Packages(super::packages::listed(
                &message["result"],
            )));
        }
        Some(Awaiting::Presets) => {
            return notify(Update::Presets(super::read_presets(&message["result"])));
        }
        Some(Awaiting::Personas) => {
            return notify(Update::Personas(super::read_personas(&message["result"])));
        }
        Some(Awaiting::Usage(kind)) => {
            let rows = Ok(super::cost::rows(&message["result"]));
            return notify(Update::UsageRows { kind, rows });
        }
        Some(Awaiting::Catalog) => {
            return notify(Update::Catalog(super::catalog::read(&message["result"])));
        }
        Some(Awaiting::CommandRun | Awaiting::Slash) => return notify(ran(&message["result"])),
        Some(Awaiting::Output(session, job, after)) => {
            let output = Some(JobOutput::read(&message["result"]));
            return notify(Update::Output {
                session,
                job,
                output,
                after,
            });
        }
        // 要不到一行行的（核心旧、出错）：交回空的，框里写没有。
        Some(Awaiting::Choices) => {
            return notify(Update::Choices(super::models::choices(&message["result"])));
        }
        Some(Awaiting::Files(word)) => {
            let result = Some(message["result"].clone());
            return notify(Update::Files { word, result });
        }
        Some(Awaiting::LinkPreview(url)) => {
            let card = super::links::card(&message["result"]);
            return notify(Update::LinkCard { url, card });
        }
        Some(Awaiting::Mermaid(source)) => {
            let svg = super::mermaid::read(&message["result"]);
            return notify(Update::Mermaid { source, svg });
        }
        Some(Awaiting::Blob(blob, got)) => {
            return super::links::chunk(rpc, blob, got, &message["result"], awaiting, notify).await;
        }
        Some(Awaiting::Efforts) => {
            return notify(Update::Efforts(super::EffortList::read(&message["result"])));
        }
        Some(Awaiting::Configure(reference)) => return notify(Update::Configured(reference)),
        // 答成了不用说：抽屉照推来的 `question.answered`、`tool.approval_decided` 收、写结果。
        Some(Awaiting::Answer(..)) => return true,
        Some(Awaiting::Ask(tag)) => {
            let result = Ok(message["result"].clone());
            return notify(Update::Answer { tag, result });
        }
        Some(Awaiting::Models) => {
            return notify(Update::CoolingUntil(super::models::earliest_cooling(
                &message["result"],
            )));
        }
        Some(Awaiting::Human) => {
            return notify(Update::Human(crate::human::Human::from_reply(
                &message["result"],
            )));
        }
        Some(Awaiting::UiLanguage) => {
            return notify(Update::HeadConfig(config::head(&message["result"])));
        }
        Some(Awaiting::List | Awaiting::SessionsStream) => {
            return notify(Update::Sessions(sessions::read(&message["result"])));
        }
        Some(Awaiting::PackagesStream) => return true,
        // 最新一页到了：画进去，再带 `after` 订阅（`page.rs`）。
        Some(Awaiting::Page(session)) => {
            return super::page::take_latest(
                rpc,
                link,
                session,
                &message["result"],
                awaiting,
                notify,
            )
            .await;
        }
        Some(Awaiting::Older(session)) => {
            return super::page::take_older(link, session, Ok(&message["result"]), notify);
        }
        // 补完了：还记着的读出来、钟回到现在，再交限额（「会话列表」第 5 条）；重连以后重订的一样（`switch.rs`）。
        Some(Awaiting::Replay(session)) => {
            return switch::replayed(link, session, message, true, notify);
        }
        Some(Awaiting::Rejoin(session)) => {
            return switch::replayed(link, session, message, false, notify);
        }
        Some(Awaiting::Watch(session)) => {
            let reply = &message["result"];
            let named = [
                reply["persona"]
                    .as_str()
                    .map(|p| Update::SessionPersona(p.to_string())),
                reply["preset"]
                    .as_str()
                    .map(|p| Update::SessionPreset(p.to_string())),
            ];
            for update in named.into_iter().flatten() {
                let update = Box::new(update);
                if !notify(Update::Elsewhere {
                    session: session.clone(),
                    update,
                }) {
                    return false;
                }
            }
            if let Some(todos) = super::todos::read(&message["result"]) {
                let update = Box::new(Update::Push(Push::Todos { todos }));
                let session = session.clone();
                if !notify(Update::Elsewhere { session, update }) {
                    return false;
                }
            }
            // 长期目标（核心 X-8 补）：重订的回应里是这一刻的，没有目标的不写，照清掉了办。
            let goal = super::push::read_goal(&reply["goal"]);
            let update = Box::new(Update::Push(Push::Goal(goal)));
            if !notify(Update::Elsewhere {
                session: session.clone(),
                update,
            }) {
                return false;
            }
            if let Some(cwd) = super::connect::workspace(reply) {
                let update = Box::new(Update::Workspace { cwd, joined: false });
                let session = session.clone();
                if !notify(Update::Elsewhere { session, update }) {
                    return false;
                }
            }
            // 不带 `after` 订阅的（重连时也重订）：累计用量照三格换，中间漏掉的补回来（核心 9-6 上）。
            if let Some(snapshot) = Snapshot::read(reply) {
                let update = Box::new(Update::Snapshot(snapshot));
                let session = session.clone();
                if !notify(Update::Elsewhere { session, update }) {
                    return false;
                }
            }
            return Limits::of(message).is_none_or(|limits| {
                let update = Box::new(Update::Limits(limits));
                notify(Update::Elsewhere { session, update })
            });
        }
        None => {}
    }
    // 掉队后重新订阅主会话的回应：限额照样带着，照它更新（核心重启以后载入的也是这样）。
    if let Some(limits) = Limits::of(message) {
        return notify(Update::Limits(limits));
    }
    // 配置流：动了界面语言的再读一次最终值；掉了队重新订阅、再读一次（「界面语言」）。
    let params = &message["params"];
    // 配置页开着的要重读：哪一项变了都告诉界面（「配置页」第 25 条）。
    if message["method"] == "config.changed" && !notify(Update::ConfigChanged) {
        return false;
    }
    // 会话列表变了（核心 9-5）：整项换、删掉；掉了队重新订、整张换。它也带 `session`，别往下当会话的事件走。
    if message["method"] == "sessions.changed" {
        return sessions::change(params).is_none_or(|c| notify(Update::SessionChanged(c)));
    }
    if message["method"] == "resync" && params["stream"] == "sessions" {
        if let Ok(id) = sessions::follow(rpc).await {
            awaiting.insert(id, Awaiting::SessionsStream);
        }
        return true;
    }
    // 工具目录变了（装卸包、MCP 服务）：界面重取给人看的字；掉了队重新订（`packages.rs`）。
    if super::packages::tools_changed(message) {
        return notify(Update::ToolsChanged);
    }
    // 界面块（核心 X-15 中）：一块变了交给界面；掉了队照记着的重新订阅。
    if let Some(update) = super::widgets::changed(message) {
        return notify(update);
    }
    if super::widgets::lost(message) {
        if super::widgets::rejoin(rpc, link, awaiting).await.is_err() {
            // 发不出去是连接断了：下一条读不到，照断开重连，重连时照样再订阅。
        }
        return true;
    }
    if message["method"] == "resync" && params["stream"] == "packages" {
        if let Ok(id) = super::packages::follow(rpc).await {
            awaiting.insert(id, Awaiting::PackagesStream);
        }
        return true;
    }
    let reread = match message["method"].as_str() {
        Some("config.changed") => config::touches(params),
        Some("resync") => params["stream"] == "config",
        _ => false,
    };
    if reread {
        let sent = if message["method"] == "resync" {
            config::follow(rpc).await
        } else {
            config::read(rpc).await
        };
        if let Ok(id) = sent {
            awaiting.insert(id, Awaiting::UiLanguage);
        }
        return true;
    }
    let Some(session) = message["params"]["session"].as_str() else {
        return true;
    };
    // 只收主会话和另外订阅着的：`/new` 以后，旧会话退订之前推来的不要（蓝图「斜杠命令」`/new`）。
    let main = link.main.as_deref() == Some(session);
    if !main && !link.watched.contains(session) {
        return true;
    }
    // 一律带上是哪个会话的，界面照它分（`app/sessions.rs` 的 `route`）：切会话的那一下，原来那个会话在路上的推送
    // 不会画进新的正文（「会话列表」第 4 条）。
    let wrap = |update: Update| Update::Elsewhere {
        session: session.to_string(),
        update: Box::new(update),
    };
    match message["method"].as_str() {
        Some("event") => switch::event(link, session, &message["params"]["event"])
            .into_iter()
            .all(|p| notify(wrap(Update::Push(p)))),
        // 掉了队：带上读到的最后一个序号重新订阅，掉了的补回来（「会话列表」第 7 条）。发不出去是连接断了，下一条读不到，
        // 照断开重连。
        Some("resync") => {
            if let Ok(id) = switch::replay(rpc, link, session).await {
                awaiting.insert(id, Awaiting::Replay(session.to_string()));
            }
            true
        }
        _ => true,
    }
}

/// 斜杠命令办了（`command.run` 的回应）：回了 `opens` 的是扩展的整页带的命令，界面打开那一页（核心 X-15 下）；别的交回规范的
/// 命令名和回执。
pub(super) fn ran(result: &Value) -> Update {
    if let Some(page) = crate::widgets::PageRef::read(&result["opens"]) {
        return Update::OpenPage(page);
    }
    let text = |key: &str| result[key].as_str().unwrap_or_default().to_string();
    Update::CommandRan {
        command: text("command"),
        said: text("said"),
    }
}
