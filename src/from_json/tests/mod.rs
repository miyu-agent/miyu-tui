//! 对照测试：同一份 JSON，新读法和 serde 派生的旧读法（只在测试里留着）读出来逐字段相等，读不成的报的话一字不差。
//! 原文（带行号列号）、JSON 值两种读法都比；出厂的每份资源（`config/same.rs`、`shellhook/words.rs`）再照它造一批写坏的样本（`mutants`）一起比。

mod kinds;
mod mutants;

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::Read;

/// 两种读法比一份原文：读成的相等，读不成的报的话一样。交回旧读法读没读成。
pub fn same_text<T: Read + DeserializeOwned + PartialEq + Debug>(label: &str, json: &str) -> bool {
    let new = super::from_str::<T>(json);
    let old = serde_json::from_str::<T>(json);
    agree(label, new, old)
}

/// 两种读法比一份 JSON 值。
pub fn same_value<T: Read + DeserializeOwned + PartialEq + Debug>(
    label: &str,
    value: &Value,
) -> bool {
    let new = super::from_value::<T>(value.clone());
    let old = serde_json::from_value::<T>(value.clone());
    agree(label, new, old)
}

fn agree<T: PartialEq + Debug>(
    label: &str,
    new: serde_json::Result<T>,
    old: serde_json::Result<T>,
) -> bool {
    match (new, old) {
        (Ok(new), Ok(old)) => {
            assert!(
                new == old,
                "{label}：读出来不一样\n新：{new:?}\n旧：{old:?}"
            );
            true
        }
        (Err(new), Err(old)) => {
            assert_eq!(new.to_string(), old.to_string(), "{label}：报的话不一样");
            false
        }
        (Ok(_), Err(old)) => panic!("{label}：新读法读成了，旧的报：{old}"),
        (Err(new), Ok(_)) => panic!("{label}：旧读法读成了，新的报：{new}"),
    }
}

/// 一份出厂资源：原样两种读法都读得成、相等，再加写坏的样本逐个比。交回比了几个样本、其中几个读不成。
pub fn resource<T: Read + DeserializeOwned + PartialEq + Debug>(
    label: &str,
    json: &str,
) -> (usize, usize) {
    let value: Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{label} 不是 JSON：{e}"));
    assert!(same_text::<T>(label, json), "{label}：出厂的原文读不成");
    assert!(
        same_value::<T>(label, &value),
        "{label}：出厂的 JSON 值读不成"
    );
    let mut count = 0;
    let mut broken = 0;
    for (what, mutant) in mutants::of(&value) {
        let text = serde_json::to_string_pretty(&mutant).unwrap_or_default();
        let at = format!("{label} {what}");
        let read_text = same_text::<T>(&at, &text);
        let read_value = same_value::<T>(&at, &mutant);
        count += 2;
        broken += usize::from(!read_text) + usize::from(!read_value);
    }
    for (what, text) in mutants::doubled(json) {
        let at = format!("{label} {what}");
        count += 1;
        broken += usize::from(!same_text::<T>(&at, &text));
    }
    (count, broken)
}
