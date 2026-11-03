//! 换成的那几笔分四种各多少（核心 UP-1 的 `converted_parts`，蓝图「上下文 `/context`」第 7 条花费那一段）：币种同
//! `converted`；有一笔那一种用了、价格又不知道的，那一种核心不写，这边是 `None`。

use serde_json::Value;

/// 四种 token 各花了多少（同一个币种）。相等照位比（同 [`super::Cost`]），能放进要 `Eq` 的一行里。
#[derive(Debug, Clone, Copy, Default)]
pub struct KindMoney {
    pub uncached: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub output: Option<f64>,
}

impl PartialEq for KindMoney {
    fn eq(&self, other: &Self) -> bool {
        let bits = |v: &KindMoney| v.all().map(|x| x.map(f64::to_bits));
        bits(self) == bits(other)
    }
}

impl Eq for KindMoney {}

impl KindMoney {
    /// 读 `converted_parts`；没有这一格（老账、没换成的）是 `None`。
    pub fn read(parts: &Value) -> Option<Self> {
        parts.is_object().then(|| Self {
            uncached: parts["uncached"].as_f64(),
            cache_read: parts["cache_read"].as_f64(),
            cache_write: parts["cache_write"].as_f64(),
            output: parts["output"].as_f64(),
        })
    }

    /// 照缓存读、缓存写、输出、输入（未命中）的先后（`/context` 花费那一段一种一行的先后）。
    pub fn all(&self) -> [Option<f64>; 4] {
        [
            self.cache_read,
            self.cache_write,
            self.output,
            self.uncached,
        ]
    }

    /// 几行加起来：每一种都要每一行有才算得出，有一行没有这一格的整个是 `None`。
    pub fn sum<'a>(rows: impl IntoIterator<Item = Option<&'a KindMoney>>) -> Option<KindMoney> {
        let mut total = KindMoney {
            uncached: Some(0.0),
            cache_read: Some(0.0),
            cache_write: Some(0.0),
            output: Some(0.0),
        };
        for row in rows {
            let row = row?;
            let add = |a: Option<f64>, b: Option<f64>| Some(a? + b?);
            total = KindMoney {
                uncached: add(total.uncached, row.uncached),
                cache_read: add(total.cache_read, row.cache_read),
                cache_write: add(total.cache_write, row.cache_write),
                output: add(total.output, row.output),
            };
        }
        Some(total)
    }
}
