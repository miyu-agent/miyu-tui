//! 一家供应商真要的两项（核心 FX-11，`model.list` 的 `providers[].needs`；蓝图「配置页」第 17 条）：key、地址各是 `none`（不要，
//! 窗里不画这一项）、`optional`、`required`。老核心、扩展没装、驱动名对不上的不给，照旧都画。

use serde_json::Value;

use super::forms::Field;

/// 一项要不要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// 不要：窗里不画。
    None,
    /// 可以不填。
    Optional,
    /// 必须填。
    Required,
}

/// key、地址各要不要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Needs {
    /// key（连同认证那一行）。
    pub key: Need,
    /// 地址。
    pub base_url: Need,
}

impl Needs {
    /// 读 `needs`；不是这个样子的是 `None`（照旧都画）。
    pub fn read(v: &Value) -> Option<Needs> {
        Some(Needs {
            key: need(&v["key"])?,
            base_url: need(&v["base_url"])?,
        })
    }

    /// 窗里这一行画不画：不要 key 的不画认证、API 密钥，不要地址的不画 Base URL。
    pub fn shows(&self, field: Field) -> bool {
        match field {
            Field::Auth | Field::Key => self.key != Need::None,
            Field::BaseUrl => self.base_url != Need::None,
            _ => true,
        }
    }
}

fn need(v: &Value) -> Option<Need> {
    match v.as_str()? {
        "none" => Some(Need::None),
        "optional" => Some(Need::Optional),
        "required" => Some(Need::Required),
        _ => None,
    }
}
