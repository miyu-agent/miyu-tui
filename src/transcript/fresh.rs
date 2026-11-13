//! 一份空的正文（从 `mod.rs` 拆出来，2026-10-11）：新会话、切过去还没读到的会话、读更早一页用的临时那一份都从它起。

use std::collections::HashMap;

use super::{CacheWatch, Link, Transcript, clock, model, pages};
use crate::core::{Bill, Level, Limits, Usage};

impl Default for Transcript {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            blocks: HashMap::new(),
            unnamed: Vec::new(),
            calls: HashMap::new(),
            progress: super::progress::Lines::default(),
            running: None,
            starting: super::Starting::No,
            clock: clock::Clock::default(),
            model: None,
            link: Link::Connecting,
            session: None,
            gone: false,
            read_only: false,
            title: None,
            named: false,
            retitle_asked: false,
            level: Level::Full,
            total: Usage::default(),
            bill: Bill::default(),
            persona: None,
            preset: None,
            turn_usage: Usage::default(),
            turn_speed: super::speed::Speed::default(),
            turn_level: Level::Full,
            context: 0,
            cache: CacheWatch::default(),
            limits: Limits::default(),
            compaction_pin: None,
            goal: None,
            compacting: None,
            compaction_trouble: false,
            manual_compaction: false,
            spoke: false,
            manual: false,
            cleared: false,
            speed: Default::default(),
            retry: None,
            failure: None,
            models: model::ModelWatch::default(),
            turn: None,
            seen: 0,
            reverted: Vec::new(),
            next_id: 0,
            older: pages::Older::default(),
            waited: super::waited::Waited::default(),
            workspace: None,
            returned: Vec::new(),
            opened_by: Vec::new(),
        }
    }
}
