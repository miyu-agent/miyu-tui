//! 基础类型的样子和搬法：字、数、真假、`Option`、一串、定长的一串、表、原样的 JSON。

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use serde_json::Value;

use super::{Key, Kind, Node, Read, mismatch};

impl Read for String {
    const KIND: &'static Kind = &Kind::Str;

    fn build(node: Node) -> Self {
        match node {
            Node::Str(text) => text,
            _ => mismatch(),
        }
    }
}

impl Key for String {}

impl Read for char {
    const KIND: &'static Kind = &Kind::Char;

    fn build(node: Node) -> Self {
        match node {
            Node::Char(c) => c,
            _ => mismatch(),
        }
    }
}

impl Key for char {}

impl Read for bool {
    const KIND: &'static Kind = &Kind::Bool;

    fn build(node: Node) -> Self {
        match node {
            Node::Bool(b) => b,
            _ => mismatch(),
        }
    }
}

/// 整数：查的时候照这种整数读过，范围不会错。
macro_rules! integers {
    ($($ty:ty: $kind:ident($bits:literal) $node:ident),* $(,)?) => {$(
        impl Read for $ty {
            const KIND: &'static Kind = &Kind::$kind($bits);

            fn build(node: Node) -> Self {
                match node {
                    Node::$node(n) => n as $ty,
                    _ => mismatch(),
                }
            }
        }
    )*};
}

integers! {
    u8: Unsigned(8) Unsigned,
    u16: Unsigned(16) Unsigned,
    u32: Unsigned(32) Unsigned,
    u64: Unsigned(64) Unsigned,
    usize: Unsigned(0) Unsigned,
    i32: Signed(32) Signed,
    i64: Signed(64) Signed,
}

impl Read for f32 {
    const KIND: &'static Kind = &Kind::Float(32);

    fn build(node: Node) -> Self {
        match node {
            // 照 f32 读过再放宽成 f64 的，收回去一点不差。
            Node::Float(n) => n as f32,
            _ => mismatch(),
        }
    }
}

impl Read for f64 {
    const KIND: &'static Kind = &Kind::Float(64);

    fn build(node: Node) -> Self {
        match node {
            Node::Float(n) => n,
            _ => mismatch(),
        }
    }
}

impl Read for Value {
    const KIND: &'static Kind = &Kind::Any;

    fn build(node: Node) -> Self {
        match node {
            Node::Any(value) => value,
            _ => mismatch(),
        }
    }
}

impl<T: Read> Read for Option<T> {
    const KIND: &'static Kind = &Kind::Opt(T::KIND);

    fn build(node: Node) -> Self {
        match node {
            Node::Null => None,
            node => Some(T::build(node)),
        }
    }

    fn absent() -> Self {
        None
    }
}

impl<T: Read> Read for Vec<T> {
    const KIND: &'static Kind = &Kind::Seq(T::KIND);

    fn build(node: Node) -> Self {
        match node {
            Node::List(list) => list.into_iter().map(T::build).collect(),
            _ => mismatch(),
        }
    }
}

impl<T: Read, const N: usize> Read for [T; N] {
    const KIND: &'static Kind = &Kind::Array(N, T::KIND);

    fn build(node: Node) -> Self {
        let Node::List(list) = node else { mismatch() };
        let built: Vec<T> = list.into_iter().map(T::build).collect();
        built.try_into().unwrap_or_else(|_| mismatch())
    }
}

/// 表：同一个键写了两次的，照 serde 留后写的那个。
fn pairs<K: Read, V: Read>(node: Node) -> impl Iterator<Item = (K, V)> {
    let Node::Pairs(pairs) = node else { mismatch() };
    pairs.into_iter().map(|(k, v)| (K::build(k), V::build(v)))
}

impl<K: Key + Eq + Hash, V: Read> Read for HashMap<K, V> {
    const KIND: &'static Kind = &Kind::Map(K::KIND, V::KIND);

    fn build(node: Node) -> Self {
        pairs(node).collect()
    }
}

impl<K: Key + Ord, V: Read> Read for BTreeMap<K, V> {
    const KIND: &'static Kind = &Kind::Map(K::KIND, V::KIND);

    fn build(node: Node) -> Self {
        pairs(node).collect()
    }
}
