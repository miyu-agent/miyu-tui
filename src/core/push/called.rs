//! 一次请求记了什么（`model.called`）：落定时截掉的、用量、金额、发出去了没有、速度、出错。

use serde_json::Value;

use super::{Compaction, Push, Usage};
use crate::core::Cost;

/// 读一条 `model.called`，照先后放进 `out`。
pub(super) fn read(body: &Value, out: &mut Vec<Push>) {
    let usage = &body["usage"];
    let n = |k: &str| usage[k].as_u64().unwrap_or_default();
    let spent = usage.is_object().then(|| Usage {
        uncached: n("uncached"),
        cache_read: n("cache_read"),
        cache_write: n("cache_write"),
        output: n("output"),
        aux: 0,
    });
    // 辅助请求（回顾）：只算用量，不动上下文、缓存、速度，出错也不算这一轮的错。
    if body["purpose"].as_str().is_some_and(|p| !p.is_empty()) {
        out.extend(spent.map(Push::AuxUsage));
        out.extend(spent.map(|_| Push::Billed(Cost::read(&body["cost"]))));
        return;
    }
    // 落定时截掉的（核心 FT-1）：先于用量交，界面先把那几块截短。没写、写空的不交。
    let cut: Vec<(u64, usize)> = body["cut"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let at = usize::try_from(c["at"].as_u64()?).ok()?;
            Some((c["block"].as_u64()?, at))
        })
        .collect();
    if !cut.is_empty() {
        out.push(Push::Cut(cut));
    }
    out.extend(spent.map(Push::Usage));
    out.extend(spent.map(|_| Push::Billed(Cost::read(&body["cost"]))));
    let summary = !body["compaction"].is_null();
    if !body["request"].is_null() {
        out.push(Push::Sent {
            seen: body["seen"].as_u64().unwrap_or_default(),
            changed: body["first_difference"].is_object(),
            summary,
        });
    }
    let output = usage["output"].as_u64().unwrap_or_default();
    let (duration, first) = (
        body["duration_ms"].as_u64(),
        body["first_token_ms"].as_u64(),
    );
    // 首字到结束；没报首字的照整次（同核心 `miyu_view::Speed::of`，第 42 条）。是不是一口气到的由 `Speed::record` 判。
    if let Some(duration) = duration
        && output > 0
    {
        let ms = first.map_or(duration, |first| duration.saturating_sub(first));
        out.push(Push::Speed { output, ms });
    }
    // 摘要请求出错说压缩失败，不算这一轮的出错（蓝图「正文」第 9 条）。
    let error = || CallError::read(&body["error"]);
    match (body["result"].as_str(), summary) {
        (Some("error"), true) => out.push(Push::Compaction(Compaction::Failed(error()))),
        (Some("error"), false) => {
            // 试的是哪个端点、模型：换端点那一行的「原来的」照它写（核心 8-9）。
            if let (Some(endpoint), Some(model)) =
                (body["endpoint"].as_str(), body["model"].as_str())
            {
                out.push(Push::Tried {
                    endpoint: endpoint.to_string(),
                    model: model.to_string(),
                });
            }
            out.push(Push::CallFailed(error()));
        }
        (Some("ok"), false) => out.push(Push::CallOk),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Push, read};

    #[test]
    fn a_cut_comes_before_the_usage_and_none_gives_nothing() {
        // 核心 FT-1：`cut` 是落定时截掉的，第几块、从第几个字节起。
        let called = |cut: serde_json::Value| {
            let usage = json!({"uncached": 10, "cache_read": 0, "cache_write": 0, "output": 5});
            let mut out = Vec::new();
            read(
                &json!({"result": "ok", "usage": usage, "cut": cut}),
                &mut out,
            );
            out
        };
        let got = called(json!([
            {"block": 0, "at": 38, "bytes": 212, "role": "user"},
            {"block": 2, "at": 0, "bytes": 40, "role": "user"}
        ]));
        assert_eq!(got[0], Push::Cut(vec![(0, 38), (2, 0)]), "先截再记用量");
        assert!(matches!(got[1], Push::Usage(_)));
        // 没写、写空的、写得不像的：不交。
        for odd in [json!(null), json!([]), json!([{"block": "x"}])] {
            let got = called(odd);
            assert!(!got.iter().any(|p| matches!(p, Push::Cut(_))), "{got:?}");
        }
    }
}

/// 一次请求出的错（`model.called` 的 `error`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallError {
    /// 分类，例如 `auth`。
    pub class: String,
    /// 原话。
    pub message: String,
    /// HTTP 状态码（施工 3-5 三补）；没有状态的（连不上、流里报的、内核自己造的）、以前的核心没有。
    pub status: Option<u16>,
    /// 还要等多久才恢复，毫秒（核心 3-5 四补：供应商的 retry-after、原话里的 try again in，池全在冷却的是最早恢复的那个）。
    pub retry_after_ms: Option<u64>,
}

impl CallError {
    fn read(error: &Value) -> Self {
        Self {
            class: error["class"].as_str().unwrap_or_default().to_string(),
            message: error["message"].as_str().unwrap_or_default().to_string(),
            status: error["status"].as_u64().and_then(|s| u16::try_from(s).ok()),
            retry_after_ms: error["retry_after_ms"].as_u64(),
        }
    }
}
