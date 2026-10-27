//! 换算成一个币种的金额（核心 UC-1，`usage.query` 一行、订阅回应的 `usage` 里的 `converted`）：每一笔照它那天的汇率换成
//! `usage.currency`，换不成的照币种留在 `rest`。头不自己换算：之后实时来的一笔笔（`model.called` 的 `cost`，原币种）币种
//! 对得上的加进总数，对不上的接进 `rest`，等下一次核心交回换好的。

use serde_json::Value;

use super::Cost;

/// 换好的一个数，和没换成的那几笔。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    /// 换成 `usage.currency` 的总数。
    pub total: Cost,
    /// 没换成的，照币种各加各的。
    pub rest: Vec<Cost>,
}

impl Converted {
    /// 读 `converted`（`{"currency", "amount", "rest"?}`）；没有这一格（一笔都没换成、旧核心）的是 `None`。
    pub fn read(value: &Value) -> Option<Self> {
        Some(Self {
            total: Cost::read(value)?,
            rest: value["rest"]
                .as_array()
                .map(|list| list.iter().filter_map(Cost::read).collect())
                .unwrap_or_default(),
        })
    }

    /// 加一笔原币种的：和总数同一个币种的加进总数，别的接进 `rest`。
    pub fn put(&mut self, cost: &Cost) {
        if cost.currency == self.total.currency {
            self.total.amount += cost.amount;
        } else {
            super::add_to(&mut self.rest, cost);
        }
    }
}
