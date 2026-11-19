//! 主题的测试夹具。

use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());

/// 数「换过几次主题」的测试、换铺的底色（`set_ground`）的测试先拿这把锁：这两样全进程只有一份，并行跑时一个换了，另一个
/// 数的、画的就不准。配色方案本身每个测试线程各一份（`current.rs`），不用锁。
pub fn hold() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
