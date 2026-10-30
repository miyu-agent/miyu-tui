//! 滑块（蓝图「配置页」第 33 条子规则，核心设计 53 PC-1，2026-10-31 项目主人：「单独滑块不用有滑块窗」）：光标停在滑块那一行
//! 时 `←` `→`（`h` `l`）照 `step` 调、夹在范围里，那一行当场照调着的值画；停手半秒存一次（按住连调只存最后一次），按别的键
//! 先存。`Enter` 照旧开数的编辑窗打精确值。

use std::time::{Duration, Instant};

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::Value;

use super::current;
use super::schema::Item;
use crate::settings::{Settings, Texts};

/// 停手多久存。
const SETTLE: Duration = Duration::from_millis(500);

/// 调了还没存的：哪一项、调到的值、到点存。
#[derive(Debug, Clone, PartialEq)]
pub struct Sliding {
    /// 哪一项。
    pub key: String,
    /// 调到的值。
    pub value: Value,
    /// 到点存。
    pub due: Instant,
}

/// 往上（`up`）、往下调一步：照 `step` 对齐到从 `min` 起的格子上、夹在范围里，照 `step` 的小数位取整（免得写出
/// `0.30000000000000004`）；整数的项存整数。不是滑块的是 `None`。
pub fn nudge(item: &Item, now: &Value, up: bool) -> Option<Value> {
    let (min, max, step) = item.slide?;
    let now = now.as_f64().unwrap_or(min);
    let moved = now + if up { step } else { -step };
    let steps = ((moved - min) / step).round();
    let places = decimals(step);
    let scale = 10f64.powi(places);
    let next = ((min + steps * step).clamp(min, max) * scale).round() / scale;
    if item.kind == "float" {
        serde_json::Number::from_f64(next).map(Value::Number)
    } else {
        Some(Value::from(next.round() as i64))
    }
}

/// 一步有几位小数（最多 6 位）。
fn decimals(step: f64) -> i32 {
    (0..6)
        .find(|&p| {
            let scaled = step * 10f64.powi(p);
            (scaled - scaled.round()).abs() < 1e-9
        })
        .unwrap_or(6)
}

/// 值在范围里的几成（画短条用）：不是滑块的、不是数的是 `None`。
pub fn fraction(item: &Item, value: &Value) -> Option<f64> {
    let (min, max, _) = item.slide?;
    let v = value.as_f64()?;
    Some(((v - min) / (max - min)).clamp(0.0, 1.0))
}

impl Settings {
    /// 一页里第 `at` 项上按了一个键：是滑块、键是 `←` `→` `h` `l` 的调一步、交回 `true`；别的键先把调着的存掉、交回 `false`
    /// 照旧往下走。
    pub(in crate::settings) fn slide_key(
        &mut self,
        page: &str,
        at: usize,
        key: KeyEvent,
        texts: &Texts,
    ) -> bool {
        let up = match key.code {
            KeyCode::Right | KeyCode::Char('l') => true,
            KeyCode::Left | KeyCode::Char('h') => false,
            _ => {
                self.slide_flush(texts);
                return false;
            }
        };
        let (Some(schema), Some(data)) = (self.more.schema.as_ref(), self.data.as_ref()) else {
            return false;
        };
        let Some(item) = self
            .page_items(page)
            .get(at)
            .and_then(|&i| schema.items.get(i))
            .filter(|i| i.slide.is_some() && i.writable())
        else {
            return false;
        };
        let now = match &self.more.sliding {
            Some(s) if s.key == item.key => s.value.clone(),
            _ => current(item, data).0.clone(),
        };
        if let Some(value) = nudge(item, &now, up) {
            self.more.sliding = Some(Sliding {
                key: item.key.clone(),
                value,
                due: Instant::now() + SETTLE,
            });
        }
        true
    }

    /// 什么时候该存调着的（App 照它醒来）。
    pub fn slide_due(&self) -> Option<Instant> {
        self.more.sliding.as_ref().map(|s| s.due)
    }

    /// 存调着的（到点了、按了别的键）：写这一项，当场存。
    pub fn slide_flush(&mut self, texts: &Texts) {
        if let Some(s) = self.more.sliding.take() {
            // 读回来以前那一行照新的画，不闪回旧值。
            if let Some(data) = self.data.as_mut() {
                data.values
                    .insert(s.key.clone(), (s.value.clone(), "system".to_string()));
            }
            self.write_setting(&s.key, s.value, texts);
        }
    }
}

#[cfg(test)]
mod tests;
