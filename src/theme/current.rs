//! 当前主题放在哪。程序里是全局一份：各处画的时候直接取，不用一层层传下去。
//!
//! 单元测试里每个测试线程各一份，起头都是出厂的第一套：换主题的测试换的是自己那一份，别的测试拿 `theme::dim()` 这类和
//! 画出来的样子比时不会正好被它换掉（2026-11-18 CI 偶发红：一个测试把主题临时换成浅色又换回，另一个没拿锁的测试画出来
//! 是深色、比的时候取到浅色）。主题的表（`sheet/current.rs`）、铺的底色（`ground.rs`）照同样的办法放。

use super::Palette;
use super::palette::BUILTIN;

/// 出厂的第一套。
fn factory() -> Palette {
    serde_json::from_str(BUILTIN[0].1).unwrap_or_else(|e| panic!("出厂主题读不懂：{e}"))
}

/// 照当前主题取一样东西。
pub fn read<R>(get: impl FnOnce(&Palette) -> R) -> R {
    store::read(get)
}

/// 换成 `palette`；和现在的一模一样的不换，交回假。
pub fn replace(palette: Palette) -> bool {
    store::replace(palette)
}

#[cfg(not(test))]
mod store {
    use std::sync::{LazyLock, PoisonError, RwLock};

    use super::Palette;

    static CURRENT: LazyLock<RwLock<Palette>> = LazyLock::new(|| RwLock::new(super::factory()));

    pub fn read<R>(get: impl FnOnce(&Palette) -> R) -> R {
        get(&CURRENT.read().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn replace(palette: Palette) -> bool {
        let mut current = CURRENT.write().unwrap_or_else(PoisonError::into_inner);
        if *current == palette {
            return false;
        }
        *current = palette;
        true
    }
}

#[cfg(test)]
mod store {
    use std::cell::RefCell;

    use super::Palette;

    thread_local! {
        static CURRENT: RefCell<Palette> = RefCell::new(super::factory());
    }

    pub fn read<R>(get: impl FnOnce(&Palette) -> R) -> R {
        CURRENT.with_borrow(get)
    }

    pub fn replace(palette: Palette) -> bool {
        CURRENT.with_borrow_mut(|current| {
            if *current == palette {
                return false;
            }
            *current = palette;
            true
        })
    }
}
