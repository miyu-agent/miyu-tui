//! 连上核心、握手、开会话、订阅（蓝图 `tui.md`「连核心」第 2–4、7、8 条）。断了以后重连也走这里：每次拉起核心
//! 都换令牌，连上以后再读（门面 `miyu_client::connect` 连的时候读）。

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use serde_json::json;

use miyu_client::connect::{ConnectError, Connection, StartError};
use miyu_client::places::DataRoot;
use miyu_client::places::Env;

use super::limits::Limits;
use super::rpc::{Failure, Rpc};
use super::{StartRefusal, Update};

/// 连上、握手：给了 `MIYU_CORE_BIN` 的，核心没在跑就拉起来（[`reach`]）；没给的只连。连上以前出的错（找数据根、建骨架、连核心、
/// 拉起核心）带着核心给的说法交给界面照它的语言说（核心 EN-1、EN-2，[`StartRefusal`]）。
pub(super) async fn connect(starting: Starting, head: super::Head) -> Result<Rpc, Update> {
    let env = Env::current();
    let core_bin = std::env::var_os("MIYU_CORE_BIN")
        .map(|bin| crate::local::expand_home(bin, std::env::home_dir().as_deref()));
    let bin = core_bin.as_deref();
    let root = DataRoot::locate(&env)
        .map_err(|e| StartRefusal::update(Some(e.said()), e.to_string(), bin))?;
    root.prepare()
        .map_err(|e| StartRefusal::update(e.said(), e.to_string(), bin))?;
    let (connection, token) = reach(&root, core_bin, starting).await?;
    let mut rpc = Rpc::new(connection);
    // 有确认、提问的抽屉：说这一头有人能当场回答（核心 D-1，`session.answer`）。
    let hello = json!({
        "protocol": [1, 1],
        // 全屏的写 `tui`，终端无缝集成写 `shellhook`（核心 VN-1：AI 照它知道话从哪来）。
        "head": {"kind": head.kind(), "version": env!("CARGO_PKG_VERSION")},
        // 系统的语言（核心 8-2：`ui.language` 是 `auto` 时核心照它说话），照 `miyu_client::places::locale`，和命令行认的一样。
        "locale": miyu_client::places::locale().unwrap_or_default(),
        // 前缀要变时先问人（蓝图「前缀要变」第 1 条）。
        "caps": {"input": true, "prefix_ask": true},
        "token": token,
    });
    rpc.call("hello", hello).await.map_err(refused)?;
    Ok(rpc)
}

/// 这一次连核心为什么（核心 CS-5「人停的核心，头不再自动拉起」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Starting {
    /// 人要用：界面刚打开、点了「启动」、核心是人停的时敲了话。照 `connect_or_start` 拉起，核心起来删掉「人停的」记号。
    Asked,
    /// 断了线自己重连：核心是人停的（`miyu core stop`、别的头的「停止」、托盘的「退出」）不拉，交回「核心已停止」。
    /// 崩了、空闲退出的照旧拉起。
    Auto,
}

/// 连上核心（还没握手），交回连接和本机令牌：给了核心程序的，没在跑就照 `starting` 拉起来（[`Starting`]）；没给的只连。
pub(super) async fn reach(
    root: &DataRoot,
    core_bin: Option<PathBuf>,
    starting: Starting,
) -> Result<(Connection, String), Update> {
    let Some(bin) = core_bin else {
        return miyu_client::connect::connect(root)
            .await
            .map_err(|e| match e {
                ConnectError::NotRunning => Update::NoCoreBin,
                e => StartRefusal::update(e.said(), e.to_string(), None),
            });
    };
    let path = bin.clone();
    let core = move || {
        let mut core = Process::new(bin);
        core.arg("core");
        core
    };
    let reached = match starting {
        Starting::Asked => miyu_client::connect::connect_or_start(root, core).await,
        Starting::Auto => miyu_client::connect::reconnect_or_start(root, core).await,
    };
    reached.map_err(|e| start_failed(e, &path))
}

/// 开会话时带上的：`/model` 选的模型、人格框、预设框选的（没选的不写，核心照默认），`/workspace` 换的目录。
#[derive(Debug, Default)]
pub(super) struct Chosen {
    pub model: Option<String>,
    pub persona: Option<String>,
    pub preset: Option<String>,
    /// 验过的工作区（核心 9-7 下）；没换的照终端所在的目录。
    pub cwd: Option<String>,
}

/// `/new` 以后的第一句话：开会话、订阅。被拒的（模型、人格、预设不在了这类）是这一句没发出去，照「没发出去」办：字放回
/// 输入框、弹核心的原话（review A5，2026-10-11：原来当成连不上核心，连接好好的却什么都发不出去）。
pub(super) async fn fresh(rpc: &mut Rpc, chosen: &Chosen) -> Result<(String, Joined), Update> {
    let unsent = |update| match update {
        Update::Failed(message) => Update::Unsent {
            reason: None,
            message,
        },
        other => other,
    };
    let (id, moved) = create(rpc, chosen).await.map_err(unsent)?;
    view(rpc, &id).await.map_err(unsent)?;
    let mut joined = subscribe(rpc, &id).await.map_err(unsent)?;
    joined.moved = moved;
    Ok((id, joined))
}

/// 开一个会话：工作目录照换过的、没换的照终端现在的，带上选了的模型、人格、预设（核心 8-8、P-1 上、P-2 上）。交回会话编号，
/// 和核心换了工作目录时的「交的、实际用的」（[`moved`]）。
async fn create(rpc: &mut Rpc, chosen: &Chosen) -> Result<(String, Option<Moved>), Update> {
    let params = create_params(chosen, cwd);
    let created = rpc
        .call("session.create", params.clone())
        .await
        .map_err(refused)?;
    let id = created["session"].as_str().unwrap_or_default().to_string();
    let given = params["cwd"].as_str().unwrap_or_default();
    Ok((id, moved(given, created["cwd"].as_str())))
}

/// 开好的会话先说「开着它」（`session.viewing`，核心 HV-1），等到回应再订阅、说第一句：别的头照会话列表的 `viewed` 不替它弹
/// 「回答完了」。原来要等界面下一拍（`app/viewing.rs`）才说，答得快的一轮（假模型、一句话的回答）在那之前就答完了，别的终端个个
/// 当成后台会话弹一条（2026-11-20 量内存时撞见：十个窗口各说一句，弹了 90 条）。界面那一拍照旧再说一遍，核心看是同一个不推。
/// 核心拒了的（旧核心没有这个方法）照旧往下走。
async fn view(rpc: &mut Rpc, session: &str) -> Result<(), Update> {
    match rpc
        .call("session.viewing", json!({"session": session}))
        .await
    {
        Ok(_) | Err(Failure::Refused(_)) => Ok(()),
        Err(failure) => Err(refused(failure)),
    }
}

/// 交的工作目录、核心实际用的（两个都是给人看的写法，家目录写成 `~`）。
pub type Moved = (String, String);

/// 核心说会话实际在别的目录干活（目录太宽、退回了这个会话的默认工作区，核心 WN-1）：和交的不一样就交回两个，照 `miyu ask`
/// 「工作目录 ~ 范围过大，改用 …」那一句提示（蓝图「新会话」第 4 条）。一样的、旧核心不回 `cwd` 的是 `None`。
pub(super) fn moved(given: &str, used: Option<&str>) -> Option<Moved> {
    let (given, used) = (
        crate::local::home_short(given),
        crate::local::home_short(used?),
    );
    (given != used).then_some((given, used))
}

/// `session.create` 的参数。工作目录照 `/workspace` 换过的，没换的照终端现在的目录（`here`）；太宽的（`~` 这类）核心都照用
/// （核心 5-14 上起，原来终端自动带的太宽的退回默认工作区、人换的带 `chosen: true` 才照用；「新会话」第 4 条）。
fn create_params(chosen: &Chosen, here: impl FnOnce() -> String) -> serde_json::Value {
    let mut params = json!({"cwd": chosen.cwd.clone().unwrap_or_else(here)});
    for (key, value) in [("model", &chosen.model), ("persona", &chosen.persona)] {
        if let Some(value) = value {
            params[key] = json!(value);
        }
    }
    // 选了「无人格」：明着不带人格，不看默认的（核心 P-4 上，`"persona": null`）。
    if chosen.persona.as_deref() == Some("") {
        params["persona"] = serde_json::Value::Null;
    }
    // 选了「无工具」：一样交 `null`（核心 P-16 下）。
    if let Some(preset) = &chosen.preset {
        params["preset"] = super::preset_param(preset);
    }
    params
}

/// 订阅回应里头要的几样：限额、现在用的模型、人格、预设（以前的会话没有人格、预设的不写）。
#[derive(Debug)]
pub(super) struct Joined {
    pub limits: Limits,
    pub current: Option<super::Current>,
    pub persona: Option<String>,
    pub preset: Option<String>,
    /// 会话的工作区（核心 9-7 上）：侧边栏照它写。
    pub workspace: Option<String>,
    /// 订阅回应里「当前的」几格：新会话真正的权限级别在这里（照配置的开局级别开，2026-10-14 项目主人报：开成开放权限、
    /// 界面却写工作区）。
    pub snapshot: Option<super::Snapshot>,
    /// 开会话时核心换了工作目录（[`moved`]）：交的、实际用的。订阅的回应里没有，开会话那一步填。
    pub moved: Option<Moved>,
}

/// 订阅刚开的会话的事件流，交回订阅的回应里头要的几样（已有的会话照 `serve.rs` 读页、带 `after` 订阅）。
pub(super) async fn subscribe(rpc: &mut Rpc, session: &str) -> Result<Joined, Update> {
    let subscribed = rpc
        .call("subscribe", json!({"session": session, "stream": "events"}))
        .await
        .map_err(refused)?;
    let text = |key: &str| subscribed[key].as_str().map(str::to_string);
    Ok(Joined {
        limits: Limits::of(&json!({ "result": subscribed })).unwrap_or_default(),
        current: super::models::current(&subscribed),
        persona: text("persona"),
        preset: super::session_preset(&subscribed),
        workspace: workspace(&subscribed),
        snapshot: super::Snapshot::read(&subscribed),
        moved: None,
    })
}

/// 订阅回应里会话的工作区（核心 9-7 上）；以前的核心没有。
pub(super) fn workspace(reply: &serde_json::Value) -> Option<String> {
    reply["workspace"]["cwd"].as_str().map(str::to_string)
}

/// 请求没成：连接断了的说断开，别的说连不上和原因。
pub(super) fn refused(failure: Failure) -> Update {
    match failure {
        Failure::Io(e) => Update::Failed(e.to_string()),
        Failure::Disconnected => Update::Disconnected,
        Failure::Refused(reason) => Update::Failed(reason),
    }
}

/// 拉不起核心：`bin` 指的程序不存在，说清是哪个路径（第 8 条）；别的带着核心给的说法交给界面照它的语言说（排期第 43 条，
/// 核心 EN-2：原因只是会变的那一段，整句在说法里）。
fn start_failed(error: StartError, bin: &Path) -> Update {
    match error {
        StartError::Io(e) if e.kind() == io::ErrorKind::NotFound && !bin.exists() => {
            Update::Missing(bin.display().to_string())
        }
        error => StartRefusal::update(error.said(), error.to_string(), Some(bin)),
    }
}

/// 启动时的目录，读不出来的写 `.`（照 `miyu ask`）。
pub(super) fn cwd() -> String {
    std::env::current_dir().map_or_else(|_| ".".to_string(), |d| d.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use miyu_client::connect::StartError;

    use super::{Chosen, Update, create_params, moved, start_failed};

    #[test]
    fn a_missing_core_program_names_its_path() {
        let gone = Path::new("/nonexistent/miyu");
        let not_found = || StartError::Io(io::Error::from(io::ErrorKind::NotFound));
        assert!(
            matches!(start_failed(not_found(), gone), Update::Missing(p) if p == "/nonexistent/miyu")
        );
        // 程序在，别的东西没找到：照拉不起核心那一句说（核心 EN-2）。
        let said = |update: Update| match update {
            Update::StartRefused(r) => (r.said.key, r.said.fields.get("reason").cloned()),
            other => panic!("交给界面照它的语言说：{other:?}"),
        };
        assert_eq!(
            said(start_failed(not_found(), Path::new("/"))).0,
            "core/start/spawn_failed"
        );
        assert_eq!(
            said(start_failed(StartError::Busy, gone)).0,
            "core/start/busy"
        );
        assert_eq!(
            said(start_failed(StartError::Timeout, gone)).0,
            "core/start/timeout"
        );
        // 核心说了起不来的：照 `start/<代码>`，原因只是会变的那一段，填进 `{reason}`；没有代码的照 `start/refused` 带上原话。
        let refused = |code: Option<&str>| StartError::Refused {
            code: code.map(str::to_string),
            reason: "File exists (os error 17)".into(),
        };
        assert_eq!(
            said(start_failed(refused(Some("log_failed")), gone)),
            (
                "core/start/log_failed".into(),
                Some("File exists (os error 17)".into())
            )
        );
        assert_eq!(
            said(start_failed(refused(None), gone)).0,
            "core/start/refused"
        );
        match start_failed(refused(Some("data_root_old_layout")), gone) {
            Update::StartRefused(r) => assert_eq!(r.bin.as_deref(), Some(gone)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_core_a_person_stopped_is_only_started_again_when_a_person_asks() {
        // 核心 CS-5：`miyu core stop`、别的头的「停止」、托盘的「退出」留下 `run/stopped`。断了线自己重连的不拉起、交回「核心已停止」；
        // 人要用的（界面刚打开、点「启动」）照拉：这里的核心程序不存在，拉的那一下交回「找不到核心程序」，证明它真去拉了。
        let dir = std::env::temp_dir().join(format!("miyu-tui-stopped-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let root = miyu_client::places::DataRoot::at(dir.canonicalize().unwrap());
        root.prepare().unwrap();
        miyu_ipc::mark_stopped(&root).unwrap();
        let bin = || {
            Some(std::path::PathBuf::from(
                "/nonexistent/miyu-tui-stopped/miyu",
            ))
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        match runtime.block_on(super::reach(&root, bin(), super::Starting::Auto)) {
            Err(Update::StartRefused(r)) => assert_eq!(r.said.key, "core/start/stopped_by_user"),
            other => panic!("自己重连不拉起人停的核心：{:?}", other.map(drop)),
        }
        match runtime.block_on(super::reach(&root, bin(), super::Starting::Asked)) {
            Err(Update::Missing(path)) => assert!(path.ends_with("miyu"), "{path}"),
            other => panic!("人要用的照拉起：{:?}", other.map(drop)),
        }
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn the_workspace_is_the_changed_one_else_the_terminal_one_and_never_marked() {
        // 核心 5-14 上起太宽的都照用，不再带 `chosen`（原来 2026-10-09 项目主人报：手动 `/workspace ~` 又被退回）。
        let here = || "/home/me/src".to_string();
        let picked = Chosen {
            cwd: Some("/home/me".into()),
            ..Chosen::default()
        };
        assert_eq!(
            create_params(&picked, here),
            serde_json::json!({"cwd": "/home/me"})
        );
        assert_eq!(
            create_params(&Chosen::default(), here),
            serde_json::json!({"cwd": "/home/me/src"}),
        );
    }

    #[test]
    fn no_persona_and_no_tools_are_sent_as_null() {
        let here = || "/w".to_string();
        let none = Chosen {
            persona: Some(String::new()),
            preset: Some(String::new()),
            ..Chosen::default()
        };
        assert_eq!(
            create_params(&none, here),
            serde_json::json!({"cwd": "/w", "persona": null, "preset": null})
        );
        let named = Chosen {
            persona: Some("miyu".into()),
            preset: Some("full".into()),
            ..Chosen::default()
        };
        assert_eq!(
            create_params(&named, here),
            serde_json::json!({"cwd": "/w", "persona": "miyu", "preset": "full"})
        );
    }

    #[test]
    fn a_workspace_the_core_changed_is_reported_and_the_same_one_is_not() {
        // 核心 WN-1：交的目录太宽，核心退回会话自己的默认工作区，回应里的 `cwd` 和交的不一样；家目录照 `~` 比。
        let home = std::env::home_dir().unwrap();
        let mine = home.join("src").display().to_string();
        assert_eq!(
            moved(&mine, Some("~/src")),
            None,
            "同一个目录，写法不同也不算"
        );
        assert_eq!(moved("/", Some("/")), None);
        assert_eq!(moved("/", None), None, "旧核心不回 cwd");
        assert_eq!(
            moved("/", Some("~/.miyu/workspace/abc")),
            Some(("/".to_string(), "~/.miyu/workspace/abc".to_string()))
        );
    }
}
