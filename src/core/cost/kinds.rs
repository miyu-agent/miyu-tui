//! 换成的那几笔分四种各多少（核心 UP-1 的 `converted_parts`；`/context` 原来的花费那一段用它，发版前 check 32 去掉了）：币种同
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

    /// 照缓存读、缓存写、输出、输入（未命中）的先后。
    pub fn all(&self) -> [Option<f64>; 4] {
        [
            self.cache_read,
            self.cache_write,
            self.output,
            self.uncached,
        ]
    }
}
