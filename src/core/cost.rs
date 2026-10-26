//! 花了多少钱（核心 8-15，蓝图 `tui.md`「配置与模型」第 5 条）：一次请求冻结在 `model.called` 的 `cost`，
//! `usage.query` 交回一行行合计。头不自己算金额：几种币各加各的；核心换算好的一个数（UC-1）另记着（`converted.rs`）。

use serde_json::Value;

mod converted;

pub use converted::Converted;

/// 一笔钱：多少、什么币。相等照位比（和核心的 `Real` 一样），所以能放进要 `Eq` 的推送里。
#[derive(Debug, Clone)]
pub struct Cost {
    /// 多少，不取整。
    pub amount: f64,
    /// ISO 4217 的三个大写字母。
    pub currency: String,
}

impl PartialEq for Cost {
    fn eq(&self, other: &Self) -> bool {
        self.amount.to_bits() == other.amount.to_bits() && self.currency == other.currency
    }
}

impl Eq for Cost {}

impl Cost {
    /// 读 `model.called` 的 `cost`；算不出的（没有这一格、写坏了）是 `None`。
    pub fn read(cost: &Value) -> Option<Cost> {
        Some(Cost {
            amount: cost["amount"].as_f64()?,
            currency: cost["currency"].as_str()?.to_string(),
        })
    }
}

/// 加起来的账：照币种各加各的，另记有用量、没金额的几次。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bill {
    /// 每种币一项，照头一次见到的先后（显示时再排）。
    pub amounts: Vec<Cost>,
    /// 没有价格的几次。
    pub unpriced: u64,
    /// 核心换算成 `usage.currency` 的（UC-1）；一笔都没换成的、旧核心的是 `None`，照 `amounts` 显示。
    pub converted: Option<Converted>,
}

impl Bill {
    /// 读一份合计：`usage.query` 的一行、订阅回应的 `usage`（`amounts`、`unpriced`、`converted`）。
    pub fn read(value: &Value) -> Self {
        Self {
            amounts: value["amounts"]
                .as_array()
                .map(|list| list.iter().filter_map(Cost::read).collect())
                .unwrap_or_default(),
            unpriced: value["unpriced"].as_u64().unwrap_or_default(),
            converted: Converted::read(&value["converted"]),
        }
    }

    /// 记一次请求：有金额的加进它的币种（换算好的那个数照 [`Converted::put`] 跟着加），没有的算一次没价格。
    pub fn add(&mut self, cost: Option<&Cost>) {
        match cost {
            Some(cost) => {
                add_to(&mut self.amounts, cost);
                if let Some(converted) = &mut self.converted {
                    converted.put(cost);
                }
            }
            None => self.unpriced += 1,
        }
    }

    /// 把另一份账（子代理的）加进来。换算好的两份同一个币种的加总数；只有一份换算过的，另一份没换的接进它。
    pub fn merge(&mut self, other: &Bill) {
        self.converted = match (self.converted.take(), &other.converted) {
            (Some(mut mine), Some(theirs)) => {
                mine.put(&theirs.total);
                theirs.rest.iter().for_each(|c| mine.put(c));
                Some(mine)
            }
            (Some(mut mine), None) => {
                other.amounts.iter().for_each(|c| mine.put(c));
                Some(mine)
            }
            (None, Some(theirs)) => {
                let mut theirs = theirs.clone();
                self.amounts.iter().for_each(|c| theirs.put(c));
                Some(theirs)
            }
            (None, None) => None,
        };
        for cost in &other.amounts {
            add_to(&mut self.amounts, cost);
        }
        self.unpriced += other.unpriced;
    }
}

impl Bill {
    /// 原币种的金额和另一份一样（每种币差不过百亿分之一）：核心的账本记全了这边见过的每一笔（`app/bill_refresh.rs`）。
    pub fn same_amounts(&self, other: &Bill) -> bool {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.0);
        self.amounts.len() == other.amounts.len()
            && self.amounts.iter().all(|mine| {
                other.amounts.iter().any(|theirs| {
                    theirs.currency == mine.currency && close(theirs.amount, mine.amount)
                })
            })
    }
}

/// 一笔加进照币种各加各的那一份。
fn add_to(list: &mut Vec<Cost>, cost: &Cost) {
    match list.iter_mut().find(|c| c.currency == cost.currency) {
        Some(have) => have.amount += cost.amount,
        None => list.push(cost.clone()),
    }
}

/// `/usage` 要的三样（蓝图「配置与模型」第 5 条）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    /// 全部的合计，不分组。
    Total,
    /// 按天，全部时间。
    Days,
    /// 按模型，全部时间。
    Models,
    /// 按会话，全部时间。
    Sessions,
    /// 按用途，全部时间。
    Purposes,
}

/// 一次 `usage.query`：哪一样、从哪个时刻起、分天照哪个时区（`+09:00`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageAsk {
    /// 哪一样。
    pub kind: UsageKind,
    /// 从这个时刻起（RFC 3339）；全部时间的是 `None`。
    pub from: Option<String>,
    /// 分天的时区；不分天的是 `None`。
    pub offset: Option<String>,
}

impl UsageAsk {
    /// 请求的参数。
    pub fn params(&self) -> Value {
        let mut params = match self.kind {
            UsageKind::Total => serde_json::json!({}),
            UsageKind::Days => serde_json::json!({"group": ["day"]}),
            UsageKind::Models => serde_json::json!({"group": ["model"]}),
            UsageKind::Sessions => serde_json::json!({"group": ["session"]}),
            UsageKind::Purposes => serde_json::json!({"group": ["purpose"]}),
        };
        if let Some(from) = &self.from {
            params["from"] = Value::from(from.clone());
        }
        if let Some(offset) = &self.offset {
            params["offset"] = Value::from(offset.clone());
        }
        params
    }
}

/// `usage.query` 交回的一行。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageRow {
    /// 按天分的：那一天（`2026-10-07`）。
    pub day: Option<String>,
    /// 按模型分的：`供应商/模型`。
    pub model: Option<String>,
    /// 按会话分的：会话编号（一次性调用没有）。
    pub session: Option<String>,
    /// 按用途分的：`title`、`recap`、`vision`……；主请求、摘要请求的是 `None`。
    pub purpose: Option<String>,
    /// 按 `use` 分的（核心 CX-1 上）：`main`、`compaction`、`subagent`、`recap`、`title`、`memory`、`other`。
    pub used_for: Option<String>,
    /// 发出去的请求数。
    pub requests: u64,
    /// 四项用量加起来。
    pub tokens: u64,
    /// 四项各多少（`/context` 的累计明细写）。
    pub split: Split,
    /// 金额和没价格的几次。
    pub bill: Bill,
}

/// 一行的四项用量：没命中缓存的输入、缓存读、缓存写、输出。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Split {
    pub uncached: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub output: u64,
}

/// `usage.query` 的回应用到的汇率源（顶上的 `rates`，核心 UC-1：只在用到要署名的兜底源时写）：来源的名字。
pub fn rate_sources(result: &Value) -> Vec<String> {
    result["rates"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|rate| rate["source"].as_str().map(str::to_string))
        .collect()
}

/// 读 `usage.query` 的回应：一行行。
pub fn rows(result: &Value) -> Vec<UsageRow> {
    let Some(rows) = result["rows"].as_array() else {
        return Vec::new();
    };
    rows.iter()
        .map(|row| {
            let usage = &row["usage"];
            let n = |k: &str| usage[k].as_u64().unwrap_or_default();
            let split = Split {
                uncached: n("uncached"),
                cache_read: n("cache_read"),
                cache_write: n("cache_write"),
                output: n("output"),
            };
            UsageRow {
                day: row["day"].as_str().map(str::to_string),
                model: row["model"].as_str().map(str::to_string),
                session: row["session"].as_str().map(str::to_string),
                purpose: row["purpose"].as_str().map(str::to_string),
                used_for: row["use"].as_str().map(str::to_string),
                requests: row["requests"].as_u64().unwrap_or_default(),
                tokens: split.uncached + split.cache_read + split.cache_write + split.output,
                split,
                bill: Bill::read(row),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Bill, Cost, rows};

    #[test]
    fn currencies_add_up_separately_and_missing_prices_are_counted() {
        let usd = Cost::read(&json!({"amount": 0.25, "currency": "USD", "multiplier": 1})).unwrap();
        let cny = Cost::read(&json!({"amount": 1.3, "currency": "CNY"})).unwrap();
        let mut bill = Bill::default();
        bill.add(Some(&usd));
        bill.add(None);
        bill.add(Some(&cny));
        let mut child = Bill::default();
        child.add(Some(&usd));
        bill.merge(&child);
        assert_eq!(bill.amounts.len(), 2, "不换算");
        assert!((bill.amounts[0].amount - 0.5).abs() < 1e-12);
        assert_eq!(bill.unpriced, 1);
        assert!(Cost::read(&json!(null)).is_none(), "算不出的没有");
    }

    #[test]
    fn a_converted_total_takes_live_costs_and_merges_with_other_bills() {
        // 核心 UC-1：订阅回应、`usage.query` 给换好的数；之后实时来的原币种，币种对得上的加进总数，对不上的接进 rest。
        let usd = |amount: f64| Cost {
            amount,
            currency: "USD".into(),
        };
        let cny = |amount: f64| Cost {
            amount,
            currency: "CNY".into(),
        };
        let mut bill = Bill::read(&json!({
            "amounts": [{"currency": "USD", "amount": 1.0}], "unpriced": 2,
            "converted": {"currency": "CNY", "amount": 7.0}}));
        assert_eq!(bill.unpriced, 2);
        bill.add(Some(&cny(0.5)));
        bill.add(Some(&usd(0.25)));
        bill.add(Some(&usd(0.25)));
        let converted = bill.converted.clone().expect("换好的还在");
        assert!(
            (converted.total.amount - 7.5).abs() < 1e-12,
            "同币种加进总数"
        );
        assert_eq!(converted.rest, [usd(0.5)], "别的币种接在后面，不自己换算");
        assert_eq!(bill.amounts.len(), 2, "原币种照旧各加各的");
        // 合并：两份都换好的加总数；一份没换的（旧核心、换不成）整个接进 rest。
        let mut child = Bill::read(&json!({"amounts": [{"currency": "CNY", "amount": 1.0}],
            "converted": {"currency": "CNY", "amount": 1.0, "rest": [{"currency": "HKD", "amount": 3.0}]}}));
        child.add(Some(&usd(0.1)));
        bill.merge(&child);
        let merged = bill.converted.clone().unwrap();
        assert!((merged.total.amount - 8.5).abs() < 1e-12);
        assert_eq!(merged.rest.len(), 2, "{:?}", merged.rest);
        let mut plain = Bill::default();
        plain.add(Some(&usd(2.0)));
        plain.merge(&Bill::read(
            &json!({"converted": {"currency": "CNY", "amount": 3.0}}),
        ));
        let got = plain.converted.unwrap();
        assert!((got.total.amount - 3.0).abs() < 1e-12);
        assert_eq!(got.rest, [usd(2.0)], "没换算的那一份接进 rest");
        assert!(
            Bill::read(&json!({"amounts": []})).converted.is_none(),
            "没有这一格的照旧"
        );
    }

    #[test]
    fn a_usage_query_reply_reads_into_rows() {
        let got = rows(
            &json!({"rows": [{"day": "2026-10-07", "model": null, "requests": 2,
            "usage": {"uncached": 10, "cache_read": 20, "cache_write": 0, "output": 5},
            "amounts": [{"currency": "USD", "amount": 0.0002}], "unpriced": 1}]}),
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].day.as_deref(), Some("2026-10-07"));
        assert_eq!(got[0].tokens, 35);
        assert_eq!(got[0].split.cache_read, 20);
        assert_eq!(got[0].bill.unpriced, 1);
        assert!(rows(&json!({})).is_empty());
    }
}
