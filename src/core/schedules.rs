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
    /// 人生那一行（`loop` 是 `life`，核心 LF-13）才有：哪个人格的、什么时候开始、上一轮什么时候；别的循环是 `None`。
    pub life: Option<Life>,
}

/// 人生那一行多带的几格（核心 LF-13；`rounds` 只是记着，头不显示，不读）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Life {
    /// 哪个人格的（人格的编号）；核心刚起来、还没认出来的是 `None`。
    pub persona: Option<String>,
    /// 这一行建的时刻：人生开始的时刻。
    pub created: Option<jiff::Timestamp>,
    /// 上一轮开始的时刻（`last.at`）；还没跑过的是 `None`。
    pub last: Option<jiff::Timestamp>,
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
    /// 人生循环（核心 LF-13）：她自己定下一轮几点开始，`when` 只写一次的 `at`。
    Life,
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

/// 一项：带 `loop`、跑在已有会话上的才算。人生那一行（`loop` 是 `life`）的 `when` 是 `{at}`，没有 `every`、`cron`，照样认
/// （2026-11-17 项目主人本机验收第 23 条：人生会话右上角看不到下次几点）。
fn entry(item: &Value) -> Option<Loop> {
    let time = |v: &Value| v.as_str().and_then(|t| t.parse().ok());
    let every = match item["loop"].as_str()? {
        "dynamic" => Every::Dynamic,
        "life" => Every::Life,
        _ => match (
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
        life: (every == Every::Life).then(|| Life {
            persona: item["persona"].as_str().map(str::to_string),
            created: time(&item["created"]),
            last: time(&item["last"]["at"]),
        }),
        every,
    })
}

#[cfg(test)]
mod tests;
