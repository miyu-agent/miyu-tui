//! 读配置合成一条问（蓝图 `tui.md`「连核心」第 9 条，2026-10-15 冷启动查出）：连上、配置变了时好几处各读几项（图标、
//! 时间线、吉祥物、主题、开局级别、引导走过没有、换的吉祥物），原来各发一条 `config.get`，在核心那头一条条排着。攒起来，
//! 这一条消息办完一起发一条；回来照原来的编号分给各处，各处照旧只看自己那几项。核心不认其中哪几项（终端包的清单比终端
//! 旧、没重装）整条拒 `unknown_config_key`：碰到那几项的照原来各发时一样当被拒，别的去掉那几项再问一次。

use std::collections::{HashMap, HashSet};

use serde_json::json;

use super::App;
use crate::core::{Command, Refusal, Update};
use crate::oobe::asker::TAG_BASE;

/// 合起来那一条的请求编号：从这里往下数，一千个轮一圈（别处的编号都在 `TAG_BASE - 10` 以上、配置页的从小往上数）。
const BATCH_TAG: u64 = TAG_BASE - 1000;

/// 要读的一处：原来的编号和它的几项。
type Part = (u64, Vec<String>);

/// 攒着的、问出去还没回来的。
#[derive(Debug, Default)]
pub struct ConfigAsks {
    queued: Vec<Part>,
    flying: HashMap<u64, Vec<Part>>,
    sent: u64,
}

impl App {
    /// 读几项：先攒着，这一条消息办完（[`App::flush_config_asks`]）和别处的一起发；回应照 `tag` 交回。
    pub(super) fn ask_config(&mut self, tag: u64, keys: &[&str]) {
        let keys = keys.iter().map(ToString::to_string).collect();
        self.link.config_asks.queued.push((tag, keys));
    }

    /// 攒着的一起发一条。
    pub(super) fn flush_config_asks(&mut self) {
        let parts = std::mem::take(&mut self.link.config_asks.queued);
        if !parts.is_empty() {
            self.send_parts(parts);
        }
    }

    fn send_parts(&mut self, parts: Vec<Part>) {
        let asks = &mut self.link.config_asks;
        let tag = BATCH_TAG - asks.sent % 1000;
        asks.sent += 1;
        let mut keys: Vec<&str> = Vec::new();
        for key in parts.iter().flat_map(|(_, keys)| keys) {
            if !keys.contains(&key.as_str()) {
                keys.push(key);
            }
        }
        let params = json!({ "keys": keys });
        asks.flying.insert(tag, parts);
        self.link.core.send(Command::Ask {
            tag,
            method: "config.get",
            params,
        });
    }

    /// 合起来那一条的回应归这里（交回 `true`）：照原来的编号一处一处交回去。
    pub(super) fn config_asks_update(&mut self, update: &Update) -> bool {
        let Update::Answer { tag, result } = update else {
            return false;
        };
        let Some(parts) = self.link.config_asks.flying.remove(tag) else {
            return false;
        };
        let unknown = match result {
            Err(refusal) => unknown_keys(refusal),
            Ok(got) => {
                self.note_items(got);
                HashSet::new()
            }
        };
        let (again, answered): (Vec<Part>, Vec<Part>) = parts.into_iter().partition(|(_, keys)| {
            !unknown.is_empty() && !keys.iter().any(|k| unknown.contains(k))
        });
        for (tag, _) in answered {
            self.core(Update::Answer {
                tag,
                result: result.clone(),
            });
        }
        if !again.is_empty() {
            self.send_parts(again);
        }
        true
    }
}

/// 拒的是有不认识的键：交回那几个键；别的拒是空的。
fn unknown_keys(refusal: &Refusal) -> HashSet<String> {
    if refusal.reason.as_deref() != Some("unknown_config_key") {
        return HashSet::new();
    }
    refusal.data["problems"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["key"].as_str().map(str::to_string))
        .collect()
}
