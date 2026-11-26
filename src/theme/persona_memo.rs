//! 记住上一次照主色生成的那一套（蓝图 `tui.md`「跟随头像的配色」）：[`super::persona::overlay`] 是纯函数，同一个底子、
//! 同一个主色、同一个深浅算出来的每一格都一样。起来时三件事各挑一次主题（问到终端的颜色、照上次记下的配置项摆好、连上读到
//! `tui.theme*`），三次交给它的这三样一样，原来每次都在「palette」线程里重算一遍 MD3 配色（每次约 1 毫秒，两次在第一帧以前）。
//! 只记一份：换了主色、深浅、底子（换主题包）的照新的算，算完换掉记着的。
//!
//! 程序里全局一份；单元测试里每个测试线程各一份（同 `ground.rs`），并排跑的测试不会互相顶掉记着的那一份。

use super::Palette;

/// 记着的一份：底子、主色、深浅，和照它们算出来的那一套。
#[derive(Debug, Clone)]
pub struct Kept {
    base: Palette,
    seed: u32,
    dark: bool,
    made: Palette,
}

impl Kept {
    /// 记着的是不是照这三样算的。
    fn is(&self, base: &Palette, seed: u32, dark: bool) -> bool {
        self.seed == seed && self.dark == dark && self.base == *base
    }
}

/// 照这三样算过的，交回记着的那一套；没算过的照 `make` 算，记下再交回。
pub fn get_or_make(
    base: &Palette,
    seed: u32,
    dark: bool,
    make: impl FnOnce() -> Palette,
) -> Palette {
    if let Some(made) = store::with(|kept| {
        kept.as_ref()
            .filter(|k| k.is(base, seed, dark))
            .map(|k| k.made.clone())
    }) {
        return made;
    }
    let made = make();
    store::with(|kept| {
        *kept = Some(Kept {
            base: base.clone(),
            seed,
            dark,
            made: made.clone(),
        });
    });
    made
}

#[cfg(not(test))]
mod store {
    use std::sync::{Mutex, PoisonError};

    use super::Kept;

    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);

    pub fn with<T>(f: impl FnOnce(&mut Option<Kept>) -> T) -> T {
        f(&mut KEPT.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

#[cfg(test)]
mod store {
    use std::cell::RefCell;

    use super::Kept;

    thread_local! {
        static KEPT: RefCell<Option<Kept>> = const { RefCell::new(None) };
    }

    pub fn with<T>(f: impl FnOnce(&mut Option<Kept>) -> T) -> T {
        KEPT.with(|kept| f(&mut kept.borrow_mut()))
    }
}
