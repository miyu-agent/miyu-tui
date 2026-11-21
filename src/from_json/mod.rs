//! 界面自己的资源（字表、布局、外观、图标表这些编进程序或者放在 `resources/` 的 JSON）读成结构体的小读法（施工 LW-9）。
//!
//! serde 的派生给每个结构体、每种读法各展开一套状态机，界面的字表那一百多个结构体加起来 2 MB 代码。这里每个类型只写一张
//! 静态的「样子」（[`Kind`]：哪几个键、每一格是什么、缺了怎么办），读的时候照着样子走 serde_json 的读法（`seed.rs`，整个程序
//! 只展开一套），先读成一棵 [`Node`]，再由各类型的 [`Read::build`] 一格一格搬进结构体。
//!
//! 规矩和 serde 派生的一样：缺必填的格、多出不认识的键（写了 `deny`）、类型不对、`flatten` 进来的格，报的话、行号列号都是
//! serde_json 自己出的那一句（照同一串调用走到同一处出错），对照测试（`tests/`）守着逐字段相等、报错一字不差。
//! 协议里的类型（经 `miyu-client` 来的视图流、事件）照旧用 serde。

mod build;
mod macros;
mod seed;

#[cfg(test)]
pub(crate) mod tests;

pub(crate) use macros::{choice, record};

use serde::de::DeserializeSeed;
use serde_json::Value;

/// 一格是什么样的值。
#[derive(Debug)]
pub enum Kind {
    /// 字。
    Str,
    /// 一个字符。
    Char,
    /// 真假。
    Bool,
    /// 无符号的数，后面是位数（`usize` 记作 0）。
    Unsigned(u8),
    /// 有符号的数，后面是位数。
    Signed(u8),
    /// 小数，后面是位数（32、64）。
    Float(u8),
    /// 字，读进来以后再查一遍写法（颜色）：查不过的照它交回的那句报错。
    Checked(fn(&str) -> Result<(), String>),
    /// 可以不写、可以写 `null`。
    Opt(&'static Kind),
    /// 一串。
    Seq(&'static Kind),
    /// 定长的一串。
    Array(usize, &'static Kind),
    /// 键到值的表；键只会是字、字符或者枚举（[`Key`]）。
    Map(&'static Kind, &'static Kind),
    /// 一个结构体。
    Struct(&'static Shape),
    /// 一个枚举。
    Enum(&'static Variants),
    /// 原样留着的 JSON。
    Any,
}

/// 一个结构体的样子。
#[derive(Debug)]
pub struct Shape {
    /// 类型的名字：报错写 `struct <名字>`。
    pub name: &'static str,
    /// 不认识的键报错（serde 的 `deny_unknown_fields`）。
    pub deny: bool,
    /// 认的键，照字段的先后（不含跳过的、`flatten` 的）。
    pub keys: &'static [&'static str],
    /// 每个键那一格，和 `keys` 一一对应。
    pub fields: &'static [Field],
    /// 摊平进来的那个结构体（serde 的 `flatten`，最多一个）。
    pub flatten: Option<&'static Shape>,
}

/// 结构体的一格。
#[derive(Debug)]
pub struct Field {
    /// 值的样子。
    pub kind: &'static Kind,
    /// 没写的用默认值（serde 的 `default`）；不然没写是错，`Option` 的是 `None`。
    pub default: bool,
}

/// 一个枚举的样子。
#[derive(Debug)]
pub struct Variants {
    /// 类型的名字：报错写 `enum <名字>`。
    pub name: &'static str,
    /// 每一种在 JSON 里的写法。
    pub names: &'static [&'static str],
    /// 每一种带的值：`None` 是不带值的那种。
    pub payloads: &'static [Option<&'static Kind>],
}

/// 照样子读出来、还没搬进结构体的值。
#[derive(Debug)]
pub enum Node {
    /// `null`（`Option` 的 `None`）。
    Null,
    /// 真假。
    Bool(bool),
    /// 无符号的数。
    Unsigned(u64),
    /// 有符号的数。
    Signed(i64),
    /// 小数。
    Float(f64),
    /// 一个字符。
    Char(char),
    /// 字。
    Str(String),
    /// 一串。
    List(Vec<Node>),
    /// 键到值的表，照读到的先后。
    Pairs(Vec<(Node, Node)>),
    /// 结构体：每个键一格（没写的是 `None`），后面是摊平进来的那个。
    Fields(Vec<Option<Node>>, Option<Box<Node>>),
    /// 枚举：第几种、带的值。
    Variant(usize, Option<Box<Node>>),
    /// 原样留着的 JSON。
    Any(Value),
}

/// 能照样子读出来的类型。
pub trait Read: Sized {
    /// 它的样子。
    const KIND: &'static Kind;

    /// 照样子查过的值搬成它。
    fn build(node: Node) -> Self;

    /// 结构体里没写这一格、又没有默认值：查的时候已经报了错，只有 `Option` 走得到这里（`None`）。
    fn absent() -> Self {
        mismatch()
    }
}

/// 能做表的键的类型：字、字符、枚举。
pub trait Key: Read {}

/// 照 `T` 的样子读 JSON 原文，同 `serde_json::from_str`。
///
/// # Errors
///
/// 读不成时交回 serde_json 那一句（带行号列号）。
pub fn from_str<T: Read>(json: &str) -> serde_json::Result<T> {
    let mut de = serde_json::Deserializer::from_str(json);
    let node = seed::Seed(T::KIND).deserialize(&mut de)?;
    de.end()?;
    Ok(T::build(node))
}

/// 照 `T` 的样子读一份 JSON 值，同 `serde_json::from_value`。
///
/// # Errors
///
/// 读不成时交回 serde_json 那一句。
pub fn from_value<T: Read>(value: Value) -> serde_json::Result<T> {
    seed::Seed(T::KIND).deserialize(value).map(T::build)
}

/// 字段 `field` 的样子：宏里写 `kind_of(|s: &类型| &s.字段)`，不用再写一遍字段的类型。
pub const fn kind_of<S, T: Read>(_field: fn(&S) -> &T) -> &'static Kind {
    T::KIND
}

/// 结构体的样子（`flatten` 那一格用）。
///
/// # Panics
///
/// 不是结构体：编译时就报（只在常量里用）。
pub const fn shape_of(kind: &'static Kind) -> &'static Shape {
    match kind {
        Kind::Struct(shape) => shape,
        _ => panic!("flatten 的只能是结构体"),
    }
}

/// 结构体的下一格搬成 `T`，没写的照 `T::absent`（宏里一个字段一次调用）。不内联：一百多个字段的结构体，每格只占一次调用。
#[inline(never)]
pub fn take<T: Read>(slots: &mut std::vec::IntoIter<Option<Node>>) -> T {
    match slots.next().flatten() {
        Some(node) => T::build(node),
        None => T::absent(),
    }
}

/// 同 [`take`]，没写的用 `make` 做的默认值。
#[inline(never)]
pub fn take_or<T: Read>(slots: &mut std::vec::IntoIter<Option<Node>>, make: fn() -> T) -> T {
    match slots.next().flatten() {
        Some(node) => T::build(node),
        None => make(),
    }
}

/// 搬的时候值和样子对不上：查过的值走不到这里。
#[cold]
pub fn mismatch() -> ! {
    unreachable!("照样子查过的值和样子对不上")
}
