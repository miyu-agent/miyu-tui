//! 主题（蓝图 `tui.md`「主题」，2026-10-21 项目主人定）：框线、修饰这些形状和材质一张表，颜色归配色方案（`theme/`）。
//! 出厂那份是默认主题包的 `packages/theme-default/theme.json`，编进程序里；主题包只写想换的那几格，盖在它上面
//! （[`merged`]）。
//!
//! 画的时候到处要用，放在一个全局的读写锁里（同配色方案）；换了主题照配色方案的代数一起往前走，各处的缓存跟着重画。
//! 表里的 `layout`、`timeline` 两节是盖在 `resources/layout.json`、`timeline.json` 上的记号，读配置时盖上（[`overlay`]），
//! 不进这张表。

mod parts;

use std::sync::{Arc, LazyLock, RwLock};

use serde_json::{Map, Value};

pub use parts::{Marks, Mod, Pick, Sheet, Text};

/// 默认主题包的编号：配置里没设主题就是它。
pub const DEFAULT_PACKAGE: &str = "theme-default";

/// 默认主题，编进程序里（没装默认主题包、包读不成时照它画）。
pub const DEFAULT: &str = include_str!("../../packages/theme-default/theme.json");

static CURRENT: LazyLock<RwLock<Arc<Sheet>>> = LazyLock::new(|| {
    let sheet = parse(&default_value()).unwrap_or_else(|e| panic!("默认主题读不懂：{e}"));
    RwLock::new(Arc::new(sheet))
});

/// 现在这个主题的整份（[`merged`] 交回的）：读配置（换界面语言重读）时照它盖 `layout.json`、`timeline.json`。
static WHOLE: LazyLock<RwLock<Value>> = LazyLock::new(|| RwLock::new(default_value()));

/// 默认主题的整份（去掉了说明）。
pub fn default_value() -> Value {
    let value = serde_json::from_str(DEFAULT).unwrap_or_else(|e| panic!("默认主题读不懂：{e}"));
    strip(value)
}

/// 主题包的那一份（`pack`，只写了想换的几格）盖在默认主题上，交回整份。
pub fn merged(pack: &Value) -> Value {
    let mut whole = default_value();
    deep_merge(&mut whole, &strip(pack.clone()));
    whole
}

/// 整份主题（[`merged`] 交回的）读成表。
///
/// # Errors
///
/// 有一格不认识、值不对时，交回说哪一格的一句。
pub fn parse(theme: &Value) -> Result<Sheet, String> {
    serde_json::from_value(theme.clone()).map_err(|e| e.to_string())
}

/// 资源文件 `base`（`layout.json`、`timeline.json`）盖上主题里同名的那一节（`section`）。
///
/// # Errors
///
/// `base` 不是 JSON 时交回那一句。
pub fn overlay(base: &str, section: &str, theme: &Value) -> Result<Value, String> {
    let mut value: Value = serde_json::from_str(base).map_err(|e| e.to_string())?;
    if let Some(over) = theme.get(section) {
        deep_merge(&mut value, &strip(over.clone()));
    }
    Ok(value)
}

/// 照现在的主题读一样东西。
pub fn read<R>(get: impl FnOnce(&Sheet) -> R) -> R {
    match CURRENT.read() {
        Ok(current) => get(&current),
        Err(poisoned) => get(&poisoned.into_inner()),
    }
}

/// 一个记号（`get` 从记号里挑哪一个）。
pub fn mark(get: impl FnOnce(&Marks) -> &String) -> String {
    read(|sheet| get(&sheet.marks).clone())
}

/// 一行里几样东西之间（` · `）。
pub fn sep() -> String {
    mark(|m| &m.sep)
}

/// 截掉的地方（`…`）。
pub fn ellipsis() -> String {
    mark(|m| &m.ellipsis)
}

/// 现在这个主题的整份。
pub fn whole() -> Value {
    match WHOLE.read() {
        Ok(whole) => whole.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

/// 换主题（`whole` 是 [`merged`] 交回的整份）：和现在的一样的不动；换了的配色方案的代数往前走一格，缓存照它重画。
///
/// # Errors
///
/// 读不成（有一格不认识、值不对）时什么都不换，交回说哪一格的一句。
pub fn set(whole: &Value) -> Result<(), String> {
    let sheet = parse(whole)?;
    *WHOLE
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = whole.clone();
    let mut current = CURRENT
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if **current != sheet {
        *current = Arc::new(sheet);
        crate::theme::touch();
    }
    Ok(())
}

/// 去掉 `_` 打头的说明，各层都去。
fn strip(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(k, _)| !k.starts_with('_'))
                .map(|(k, v)| (k, strip(v)))
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

/// `over` 盖在 `base` 上：两边都是表的往里走，别的整个换掉（列表也是整个换）。
fn deep_merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(base), Value::Object(over)) => {
            for (key, value) in over {
                match base.get_mut(key) {
                    Some(slot) => deep_merge(slot, value),
                    None => {
                        base.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (slot, value) => *slot = value.clone(),
    }
}

#[cfg(test)]
mod guard;
#[cfg(test)]
mod tests;
