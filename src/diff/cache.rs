//! 照参数排出来的差异记着（发布前审查 T2-1，2026-10-30）：编辑这一步的标题 `+N -N`、收起那一行的合计、点开的差异都要
//! 同一份，还在进行的那一段每帧重排（`row_cache` 不缓存），原来每帧各比一遍。照参数里那几段字记最近的几份，比过的直接拿。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use serde_json::Value;

use super::Diff;

/// 记最近几份：一屏上的编辑、写入就这么多。
const KEEP: usize = 32;

thread_local! {
    /// 参数的指纹和排好的差异（不是编辑、写入的记成 `None`）。
    static SEEN: RefCell<VecDeque<(u64, Option<Rc<Diff>>)>> = const { RefCell::new(VecDeque::new()) };
}

/// 参数里要比的那几段字（写入的 `content`、编辑每一处的前后）的指纹；不拷字。
pub(super) fn key(args: &Value) -> u64 {
    let mut h = DefaultHasher::new();
    args["content"].as_str().hash(&mut h);
    for piece in args["edits"].as_array().into_iter().flatten() {
        piece["old_string"].as_str().hash(&mut h);
        piece["new_string"].as_str().hash(&mut h);
    }
    h.finish()
}

/// 比过的直接拿；没比过的照 `make` 比一遍记下（最近的在前，多了丢最旧的）。
pub(super) fn get_or(key: u64, make: impl FnOnce() -> Option<Diff>) -> Option<Rc<Diff>> {
    if let Some(hit) =
        SEEN.with_borrow(|seen| seen.iter().find(|(k, _)| *k == key).map(|(_, d)| d.clone()))
    {
        return hit;
    }
    let made = make().map(Rc::new);
    SEEN.with_borrow_mut(|seen| {
        seen.push_front((key, made.clone()));
        seen.truncate(KEEP);
    });
    made
}
