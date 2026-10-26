//! 编辑窗里的值和字互相换（从 `forms.rs` 挪出来，2026-10-26）。

use serde_json::Value;

/// 一个值写成字：字照写，数照写，别的（`null`、表）是空的。
pub fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// 窗口写成人看的：和用量那一行一样写短（`128k`、`1M`、`1.4M`）。
pub fn window_text(n: u64) -> String {
    crate::meter::short(n)
}

/// 人敲的窗口读成数：`128k`、`1M`、`131072`；读不懂的是 `None`。
pub fn window_value(text: &str) -> Option<u64> {
    let t = text.trim().to_lowercase();
    let (digits, times) = match t.strip_suffix('m') {
        Some(d) => (d, 1_000_000),
        None => t.strip_suffix('k').map_or((t.as_str(), 1), |d| (d, 1_000)),
    };
    let n: f64 = digits.trim().parse().ok()?;
    (n > 0.0).then(|| (n * f64::from(times)).round() as u64)
}
