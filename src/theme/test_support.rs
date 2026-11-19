//! 主题的测试夹具。

use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());

/// 数「换过几次主题」的测试、会让它往前走的测试先拿这把锁：换代数全进程只有一份，并行跑时一个换了，另一个数的就不准。配色方案
/// （`current.rs`）、主题的表（`sheet/current.rs`）、铺的底色（`ground.rs`）每个测试线程各一份，不用锁。
pub fn hold() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
