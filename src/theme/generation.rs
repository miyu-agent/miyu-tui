//! 换过几次主题。排好存着的东西（回答的 Markdown、时间线的行、图）把它算进键里，换了主题就重排。
//!
//! 程序里全局一份。单元测试里每个测试线程各一份（同 `current.rs`）：换主题的测试往前走的只是自己那一份，别的测试照缓存数
//! 重排了几条、等图做好时，键不会正好被它换掉（2026-11-19 放大复现：换主题的测试来回换时 `figures::` 的测试成片红）。

/// 往前走一格。
pub fn bump() {
    store::bump();
}

/// 现在是第几代。
pub fn get() -> u64 {
    store::get()
}

#[cfg(not(test))]
mod store {
    use std::sync::atomic::{AtomicU64, Ordering};

    static GENERATION: AtomicU64 = AtomicU64::new(0);

    pub fn bump() {
        GENERATION.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get() -> u64 {
        GENERATION.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod store {
    use std::cell::Cell;

    thread_local! {
        static GENERATION: Cell<u64> = const { Cell::new(0) };
    }

    pub fn bump() {
        GENERATION.set(GENERATION.get() + 1);
    }

    pub fn get() -> u64 {
        GENERATION.get()
    }
}
