//! 循环（核心 J-1 下、J-2，`schedules` 流；蓝图 `tui.md`「循环」第 1 条）：连上就订唤醒表，回应是整张，之后推一项一项的
//! 变化；终端只留跑在已有会话上的循环，照会话认是不是看着的这个。掉了队重新订。

use serde_json::{Value, json};

/// 一个循环。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    /// 唤醒表里的编号（`t3`）。
    pub id: String,
    /// 跑在哪个会话。
    pub session: String,
    /// 多久一次。
    pub every: Every,
    /// 下一次；停着的、动态间隔这一轮还没定的没有。
    pub next: Option<jiff::Timestamp>,
    /// 停着。
    pub paused: bool,
    /// 交进会话的那句话。
    pub prompt: String,
    /// 上一次交进会话的那一刻和结果（`last` 的 `at`、`result`：`ran`、`queued`、`held`、`skipped:…`）；还没交过的没有。
    pub last: Option<(String, String)>,
}

/// 循环多久一次。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Every {
    /// 固定间隔，秒。
    Seconds(u64),
    /// 照 cron 的固定间隔。
    Cron(String),
    /// 动态间隔：每一轮结束时定下一次。
    Dynamic,
    /// AI 用 `schedule_next` 挂的一次定时触发（核心 FX-21：`by` 是 `model`、`when` 是 `at`、没有 `loop`），到点交一次就删。
    Once,
}

/// 唤醒表的变化。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// 整张（订阅的回应）。
    All(Vec<Loop>),
    /// 这一项换成它；不是循环了、不在已有会话上的照删掉办（`None`）。
    One(String, Option<Loop>),
}

/// 订阅。交回请求编号。
pub(super) async fn follow(rpc: &mut super::rpc::Rpc) -> std::io::Result<String> {
    rpc.send("subscribe", json!({"stream": "schedules"})).await
}

/// 订阅的回应：整张表里的循环。
pub fn table(result: &Value) -> Vec<Loop> {
    result["schedules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(entry)
        .collect()
}

/// 推来的 `schedules.changed`；不是的是 `None`。
pub fn changed(message: &Value) -> Option<Change> {
    if message["method"] != "schedules.changed" {
        return None;
    }
    let params = &message["params"];
    let id = params["id"].as_str()?.to_string();
    let one = (params["removed"] != true)
        .then(|| entry(&params["entry"]))
        .flatten();
    Some(Change::One(id, one))
}

/// 掉了队：要重新订。
pub fn lost(message: &Value) -> bool {
    message["method"] == "resync" && message["params"]["stream"] == "schedules"
}

/// 照一条变化改手上那一份。
pub fn apply(loops: &mut Vec<Loop>, change: Change) {
    match change {
        Change::All(all) => *loops = all,
        Change::One(id, one) => match (loops.iter_mut().find(|l| l.id == id), one) {
            (Some(slot), Some(one)) => *slot = one,
            (None, Some(one)) => loops.push(one),
            (_, None) => loops.retain(|l| l.id != id),
        },
    }
}

/// 这一项变化是看着的会话（`viewing`）上的循环刚交了一次、只记下没开轮（`last.result` 是 `held`，核心 SC-1：比如还能恢复
/// 撤销时）。整张（刚订上）的不算，上一次就是这一次的（别的字段变了又推一遍）不算。
pub fn newly_held(loops: &[Loop], change: &Change, viewing: Option<&str>) -> bool {
    let Change::One(id, Some(now)) = change else {
        return false;
    };
    let held = now
        .last
        .as_ref()
        .is_some_and(|(_, result)| result == "held");
    let before = loops
        .iter()
        .find(|l| &l.id == id)
        .and_then(|l| l.last.as_ref());
    // 「循环这一轮没开始」只说循环（AI 挂的一次定时触发不算）
    held && now.every != Every::Once
        && Some(now.session.as_str()) == viewing
        && before != now.last.as_ref()
}

/// 一项：带 `loop`（`fixed`、`dynamic`）的循环、AI 挂的一次定时触发（核心 FX-21），跑在已有会话上的才算；人建的定时任务不算。
fn entry(item: &Value) -> Option<Loop> {
    let time = |v: &Value| v.as_str().and_then(|t| t.parse().ok());
    let once = item["by"]["kind"] == "model" && item["when"]["at"].is_string();
    let every = match item["loop"].as_str() {
        None if once => Every::Once,
        None => return None,
        Some("dynamic") => Every::Dynamic,
        Some(_) => match (
            item["when"]["every"].as_u64(),
            item["when"]["cron"].as_str(),
        ) {
            (Some(seconds), _) => Every::Seconds(seconds),
            (None, Some(cron)) => Every::Cron(cron.to_string()),
            (None, None) => return None,
        },
    };
    Some(Loop {
        id: item["id"].as_str()?.to_string(),
        session: item["target"]["session"].as_str()?.to_string(),
        next: time(&item["next"]),
        paused: item["paused"] == true,
        prompt: item["prompt"].as_str().unwrap_or_default().to_string(),
        last: item["last"]["at"]
            .as_str()
            .zip(item["last"]["result"].as_str())
            .map(|(at, result)| (at.to_string(), result.to_string())),
        every,
    })
}

#[cfg(test)]
mod tests;
