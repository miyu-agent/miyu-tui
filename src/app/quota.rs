//! `/usage` 框里的套餐额度（核心 QU-1，蓝图「配置与模型」第 5 条，2026-11-14 项目主人定样子）：开框时和 `usage.query`
//! 一起问一次看着的会话的 `session.quota`，回来了交给框画（`ui/usage/quota.rs`）。没有额度的核心回 `{}`，框里不画。
//! 核心那边缓存 60 秒、每轮后台刷新，头只在开框时问，不轮询。用 DeepSeek 官方供应商的回应是账户余额（核心 QU-2，
//! `balance`），和窗口各是各的。

use serde_json::{Value, json};

use super::App;
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 问额度的那一次。
const QUOTA_TAG: u64 = TAG_BASE - 46;

/// 一个额度窗口：名字照连接的语言（「5 小时」「每周」「每月」），用了百分之几，什么时候重置。
#[derive(Debug, Clone, PartialEq)]
pub struct QuotaWindow {
    /// 名字。
    pub label: String,
    /// 用了百分之几（可能带小数）。
    pub percent: f64,
    /// 什么时候重置；没给、读不懂的不写。
    pub resets_at: Option<jiff::Timestamp>,
}

/// 账户余额的一种币（核心 QU-2）：金额都是官方写的原样字符串，不换成数。
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceItem {
    /// 币种代码，照官方写的（`CNY`、`USD`，也可能是别的）。
    pub currency: String,
    /// 充值的（官网「充值余额」照这一格写）。
    pub topped_up: String,
    /// 赠送的。
    pub granted: String,
}

/// 账户余额（核心 QU-2）：能不能用（官方说的原样），每种币一项、照官方的先后；`items` 可能是空的。
#[derive(Debug, Clone, PartialEq)]
pub struct Balance {
    /// 能不能用。
    pub available: bool,
    /// 每种币一项。
    pub items: Vec<BalanceItem>,
}

/// 读回应里的 `balance`：没查到的（不带这一格）、`available` 不是布尔的是 `None`；币种、金额缺的那一项不要。
pub(crate) fn balance(result: &Value) -> Option<Balance> {
    let b = result.get("balance")?;
    let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);
    let items = b
        .get("items")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |list| {
            list.iter()
                .filter_map(|i| {
                    Some(BalanceItem {
                        currency: text(i, "currency")?,
                        topped_up: text(i, "topped_up")?,
                        granted: text(i, "granted").unwrap_or_default(),
                    })
                })
                .collect()
        });
    Some(Balance {
        available: b.get("available")?.as_bool()?,
        items,
    })
}

/// 读 `session.quota` 的回应：`windows` 一项一个，照核心排好的先后；没有额度的（`{}`）、名字或百分比缺的不要。
pub(crate) fn windows(result: &Value) -> Vec<QuotaWindow> {
    let Some(list) = result.get("windows").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|w| {
            Some(QuotaWindow {
                label: w.get("label")?.as_str()?.to_string(),
                percent: w.get("percent")?.as_f64()?,
                resets_at: w
                    .get("resets_at")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse().ok()),
            })
        })
        .collect()
}

impl App {
    /// 开框时问一次看着的会话的额度；还没开会话的不问。
    pub(super) fn ask_quota(&mut self) {
        if let Some(session) = self.talk.transcript.session.clone() {
            self.link.core.send(Command::Ask {
                tag: QUOTA_TAG,
                method: "session.quota",
                params: json!({ "session": session }),
            });
        }
    }

    /// 问的回应归这里交回 `true`：框还开着就记下；拒了的（旧核心不认）当没有。
    pub(super) fn quota_answer(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        if *tag != QUOTA_TAG {
            return false;
        }
        if let (Some(data), Ok(result)) = (self.boxes.usage.as_mut(), result) {
            data.quota = windows(result);
            data.balance = balance(result);
            // 核心照会话接下来请求的那一家查（`subscribe` 的 `model.endpoint`，换模型推来的同一个）：标题照它写名字。
            data.quota_from = self
                .talk
                .transcript
                .model
                .as_ref()
                .map(|(_, endpoint)| endpoint.clone())
                .filter(|endpoint| !endpoint.is_empty());
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::windows;

    #[test]
    fn windows_are_read_in_order_and_empty_means_none() {
        let got = windows(&json!({"windows": [
            {"id": "rolling", "label": "5 小时", "percent": 4.2, "status": "ok", "resets_at": "2026-11-15T03:00:00Z"},
            {"id": "weekly", "label": "每周", "percent": 81, "status": "ok"},
            {"id": "monthly", "percent": 10},
        ]}));
        assert_eq!(got.len(), 2, "名字缺的不要");
        assert_eq!(got[0].label, "5 小时");
        assert!((got[0].percent - 4.2).abs() < 1e-9);
        assert_eq!(
            got[0].resets_at.unwrap().to_string(),
            "2026-11-15T03:00:00Z"
        );
        assert_eq!(got[1].resets_at, None);
        assert!(windows(&json!({})).is_empty(), "没有额度回空的");
    }
}
