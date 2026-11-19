//! 主题的测试夹具。

use std::sync::{Mutex, MutexGuard};

static LOCK: Mutex<()> = Mutex::new(());

/// 以前数「换过几次主题」的测试、会让它往前走的测试先拿这把锁。现在配色方案（`current.rs`）、主题的表（`sheet/current.rs`）、
/// 铺的底色（`ground.rs`）、换代数（`generation.rs`）在测试里都是每个测试线程各一份，拿不拿锁不再影响别的测试。
pub fn hold() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
