//! 现在这个主题放在哪。程序里是全局一份：画的时候到处要用，不用一层层传下去。
//!
//! 单元测试里每个测试线程各一份，起头都是默认主题（同配色方案的 `theme/current.rs`）：换主题的测试换的是自己那一份，别的
//! 测试画出来的框线、记号不会正好被它换成纯 ASCII 的。

use serde_json::Value;

use super::Sheet;

/// 默认主题读成的表。
fn factory() -> Sheet {
    super::parse(&super::default_value()).unwrap_or_else(|e| panic!("默认主题读不懂：{e}"))
}

/// 照现在的表读一样东西。
pub fn read<R>(get: impl FnOnce(&Sheet) -> R) -> R {
    store::read(get)
}

/// 现在这个主题的整份。
pub fn whole() -> Value {
    store::whole()
}

/// 换成 `sheet`（整份是 `whole`）：整份照记；表和现在的一模一样的不换，交回假。
pub fn replace(sheet: Sheet, whole: Value) -> bool {
    store::replace(sheet, whole)
}

#[cfg(not(test))]
mod store {
    use std::sync::{Arc, LazyLock, PoisonError, RwLock};

    use serde_json::Value;

    use super::Sheet;

    static CURRENT: LazyLock<RwLock<Arc<Sheet>>> =
        LazyLock::new(|| RwLock::new(Arc::new(super::factory())));

    static WHOLE: LazyLock<RwLock<Value>> =
        LazyLock::new(|| RwLock::new(super::super::default_value()));

    pub fn read<R>(get: impl FnOnce(&Sheet) -> R) -> R {
        get(&CURRENT.read().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn whole() -> Value {
        WHOLE.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn replace(sheet: Sheet, whole: Value) -> bool {
        *WHOLE.write().unwrap_or_else(PoisonError::into_inner) = whole;
        let mut current = CURRENT.write().unwrap_or_else(PoisonError::into_inner);
        if **current == sheet {
            return false;
        }
        *current = Arc::new(sheet);
        true
    }
}

#[cfg(test)]
mod store {
    use std::cell::RefCell;

    use serde_json::Value;

    use super::Sheet;

    thread_local! {
        static CURRENT: RefCell<Sheet> = RefCell::new(super::factory());
        static WHOLE: RefCell<Value> = RefCell::new(super::super::default_value());
    }

    pub fn read<R>(get: impl FnOnce(&Sheet) -> R) -> R {
        CURRENT.with_borrow(get)
    }

    pub fn whole() -> Value {
        WHOLE.with_borrow(Clone::clone)
    }

    pub fn replace(sheet: Sheet, whole: Value) -> bool {
        WHOLE.set(whole);
        CURRENT.with_borrow_mut(|current| {
            if *current == sheet {
                return false;
            }
            *current = sheet;
            true
        })
    }
}
