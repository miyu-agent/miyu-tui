//! 照样子走 serde_json 的读法：调的是派生出来的代码会调的那几个方法（`deserialize_struct`、`deserialize_identifier`……），
//! 出错的地方、报的话、行号列号就和派生的一样。整个程序只展开这一套（读原文、读 JSON 值各一份）。

use std::fmt;

use serde::Deserialize;
use serde::de::{
    DeserializeSeed, Deserializer, EnumAccess, Error, IgnoredAny, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};
use serde_json::Value;

use super::{Kind, Node, Shape, Variants};

/// 照一个样子读一个值。
pub(super) struct Seed(pub &'static Kind);

impl<'de> DeserializeSeed<'de> for Seed {
    type Value = Node;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Node, D::Error> {
        Ok(match self.0 {
            Kind::Str => Node::Str(String::deserialize(d)?),
            Kind::Char => Node::Char(char::deserialize(d)?),
            Kind::Bool => Node::Bool(bool::deserialize(d)?),
            Kind::Unsigned(bits) => Node::Unsigned(unsigned(*bits, d)?),
            Kind::Signed(bits) => Node::Signed(match bits {
                32 => i32::deserialize(d)?.into(),
                _ => i64::deserialize(d)?,
            }),
            Kind::Float(bits) => Node::Float(match bits {
                32 => f32::deserialize(d)?.into(),
                _ => f64::deserialize(d)?,
            }),
            Kind::Checked(check) => {
                let text = String::deserialize(d)?;
                check(&text).map_err(D::Error::custom)?;
                Node::Str(text)
            }
            Kind::Opt(inner) => d.deserialize_option(Opt(inner))?,
            Kind::Seq(inner) => d.deserialize_seq(List(inner))?,
            Kind::Array(len, inner) => d.deserialize_tuple(*len, Array(*len, inner))?,
            Kind::Map(key, value) => d.deserialize_map(Pairs(key, value))?,
            // 带 `flatten` 的结构体，派生的照表读（`deserialize_map`），不认一串的写法。
            Kind::Struct(shape) if shape.flatten.is_some() => d.deserialize_map(Record(shape))?,
            Kind::Struct(shape) => d.deserialize_struct(shape.name, shape.keys, Record(shape))?,
            Kind::Enum(variants) => {
                d.deserialize_enum(variants.name, variants.names, Choice(variants))?
            }
            Kind::Any => Node::Any(Value::deserialize(d)?),
        })
    }
}

/// 无符号的数照位数读：越界的报错是那种整数自己的那句。
fn unsigned<'de, D: Deserializer<'de>>(bits: u8, d: D) -> Result<u64, D::Error> {
    Ok(match bits {
        8 => u8::deserialize(d)?.into(),
        16 => u16::deserialize(d)?.into(),
        32 => u32::deserialize(d)?.into(),
        64 => u64::deserialize(d)?,
        _ => usize::deserialize(d)? as u64,
    })
}

/// 表的键：字、字符、枚举（[`super::Key`]）。
struct KeySeed(&'static Kind);

impl<'de> DeserializeSeed<'de> for KeySeed {
    type Value = Node;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Node, D::Error> {
        match self.0 {
            Kind::Char => char::deserialize(d).map(Node::Char),
            Kind::Enum(variants) => {
                d.deserialize_enum(variants.name, variants.names, Choice(variants))
            }
            _ => String::deserialize(d).map(Node::Str),
        }
    }
}

struct Opt(&'static Kind);

impl<'de> Visitor<'de> for Opt {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("option")
    }

    fn visit_none<E: Error>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }

    fn visit_unit<E: Error>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Node, D::Error> {
        Seed(self.0).deserialize(d)
    }
}

struct List(&'static Kind);

impl<'de> Visitor<'de> for List {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut out = Vec::new();
        while let Some(node) = seq.next_element_seed(Seed(self.0))? {
            out.push(node);
        }
        Ok(Node::List(out))
    }
}

struct Array(usize, &'static Kind);

impl<'de> Visitor<'de> for Array {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an array of length {}", self.0)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut out = Vec::with_capacity(self.0);
        for at in 0..self.0 {
            match seq.next_element_seed(Seed(self.1))? {
                Some(node) => out.push(node),
                None => return Err(A::Error::invalid_length(at, &self)),
            }
        }
        Ok(Node::List(out))
    }
}

struct Pairs(&'static Kind, &'static Kind);

impl<'de> Visitor<'de> for Pairs {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut out = Vec::new();
        while let Some(key) = map.next_key_seed(KeySeed(self.0))? {
            out.push((key, map.next_value_seed(Seed(self.1))?));
        }
        Ok(Node::Pairs(out))
    }
}

/// 结构体：照派生的 `visit_map`、`visit_seq` 走。
struct Record(&'static Shape);

/// 结构体里读到的一个键。
enum Slot {
    /// 第几格。
    Field(usize),
    /// 不认识的，留给摊平进来的那个（带 `flatten` 的才有）。
    Other(String),
    /// 不认识的，不要。
    Ignored,
}

impl<'de> Visitor<'de> for Record {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "struct {}", self.0.name)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let shape = self.0;
        let mut slots: Vec<Option<Node>> = shape.keys.iter().map(|_| None).collect();
        let mut rest = Vec::new();
        while let Some(key) = map.next_key_seed(FieldKey(shape))? {
            match key {
                Slot::Field(at) => {
                    if slots[at].is_some() {
                        return Err(A::Error::duplicate_field(shape.keys[at]));
                    }
                    slots[at] = Some(map.next_value_seed(Seed(shape.fields[at].kind))?);
                }
                Slot::Other(name) => rest.push(Some((name, map.next_value::<Value>()?))),
                Slot::Ignored => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }
        missing(shape, &slots)?;
        let flat = match shape.flatten {
            Some(inner) => Some(Box::new(flattened(inner, &mut rest)?)),
            None => None,
        };
        // 派生的：摊平进来的也没要的键，`deny` 时报第一个。
        if shape.deny
            && let Some((name, _)) = rest.into_iter().flatten().next()
        {
            return Err(A::Error::custom(format_args!("unknown field `{name}`")));
        }
        Ok(Node::Fields(slots, flat))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let shape = self.0;
        let mut slots = Vec::with_capacity(shape.fields.len());
        for (at, field) in shape.fields.iter().enumerate() {
            match seq.next_element_seed(Seed(field.kind))? {
                Some(node) => slots.push(Some(node)),
                None if field.default => slots.push(None),
                None => {
                    let count = shape.fields.len();
                    let plural = if count == 1 { "" } else { "s" };
                    let expected = format!("struct {} with {count} element{plural}", shape.name);
                    return Err(A::Error::invalid_length(at, &expected.as_str()));
                }
            }
        }
        Ok(Node::Fields(slots, None))
    }
}

/// 没写的格：照字段的先后报第一个缺了的必填格（`Option` 的不算缺）。
fn missing<E: Error>(shape: &Shape, slots: &[Option<Node>]) -> Result<(), E> {
    let lost = shape.fields.iter().zip(slots).position(|(field, slot)| {
        slot.is_none() && !field.default && !matches!(field.kind, Kind::Opt(_))
    });
    match lost {
        Some(at) => Err(E::missing_field(shape.keys[at])),
        None => Ok(()),
    }
}

/// 摊平进来的结构体：从外面不认识的那些键里拿自己的（照读到的先后），同 serde 的 `FlatMapDeserializer`。值照 JSON 值读，
/// 报的话同它（不带位置，外面那一层补上）。
fn flattened<E: Error>(shape: &Shape, rest: &mut [Option<(String, Value)>]) -> Result<Node, E> {
    let mut slots: Vec<Option<Node>> = shape.keys.iter().map(|_| None).collect();
    for entry in rest.iter_mut() {
        let Some(at) = entry
            .as_ref()
            .and_then(|(name, _)| shape.keys.iter().position(|k| k == name))
        else {
            continue;
        };
        let Some((_, value)) = entry.take() else {
            continue;
        };
        if slots[at].is_some() {
            return Err(E::duplicate_field(shape.keys[at]));
        }
        let node = Seed(shape.fields[at].kind)
            .deserialize(value)
            .map_err(E::custom)?;
        slots[at] = Some(node);
    }
    missing(shape, &slots)?;
    Ok(Node::Fields(slots, None))
}

/// 结构体的键，照派生的 `__Field` 读（`deserialize_identifier`）。
struct FieldKey(&'static Shape);

impl<'de> DeserializeSeed<'de> for FieldKey {
    type Value = Slot;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Slot, D::Error> {
        d.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for FieldKey {
    type Value = Slot;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("field identifier")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<Slot, E> {
        let shape = self.0;
        match shape.keys.iter().position(|k| *k == name) {
            Some(at) => Ok(Slot::Field(at)),
            None if shape.flatten.is_some() => Ok(Slot::Other(name.to_string())),
            None if shape.deny => Err(E::unknown_field(name, shape.keys)),
            None => Ok(Slot::Ignored),
        }
    }
}

/// 枚举：照派生的 `visit_enum` 走。
struct Choice(&'static Variants);

impl<'de> Visitor<'de> for Choice {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "enum {}", self.0.name)
    }

    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Node, A::Error> {
        let (at, access) = data.variant_seed(VariantKey(self.0))?;
        match self.0.payloads[at] {
            None => {
                access.unit_variant()?;
                Ok(Node::Variant(at, None))
            }
            Some(kind) => {
                let node = access.newtype_variant_seed(Seed(kind))?;
                Ok(Node::Variant(at, Some(Box::new(node))))
            }
        }
    }
}

/// 枚举的哪一种，照派生的 `__Field` 读。
struct VariantKey(&'static Variants);

impl<'de> DeserializeSeed<'de> for VariantKey {
    type Value = usize;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<usize, D::Error> {
        d.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for VariantKey {
    type Value = usize;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("variant identifier")
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<usize, E> {
        let names = self.0.names;
        names
            .iter()
            .position(|n| *n == name)
            .ok_or_else(|| E::unknown_variant(name, names))
    }
}
