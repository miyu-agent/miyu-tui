//! 照字段表给结构体、枚举写 [`super::Read`]：一个字段一行，规矩照原来的 serde 属性写在括号里。
//!
//! ```text
//! record! {
//!     Icons, deny {                       // deny：不认识的键报错（deny_unknown_fields）
//!         tools,                          // 必填；Option 的没写是 None
//!         lucide(default),                // 没写用 Default
//!         warn(default = warn_mark),      // 没写用这个函数
//!         loop_mark("loop", default = loop_mark),   // JSON 里的键另起名（rename）
//!     }
//!     skip { name }                       // 不从 JSON 读，用 Default（skip）
//!     flatten head                        // 这一格摊平读（flatten）
//! }
//! choice! { Level { Full = "full", ReadOnly = "read_only" } }
//! choice! { Body { None = "none", Arg(String) = "arg" } }       // 带值的写成 {"arg": "..."}
//! ```

/// 给结构体写 [`super::Read`]，见本文件开头。
macro_rules! record {
    (@deny) => { false };
    (@deny deny) => { true };
    (@key $f:ident) => { stringify!($f) };
    (@key $f:ident default $($rest:tt)*) => { stringify!($f) };
    (@key $f:ident $key:literal $($rest:tt)*) => { $key };
    (@default) => { false };
    (@default default $($rest:tt)*) => { true };
    (@default $key:literal) => { false };
    (@default $key:literal, $($rest:tt)*) => { $crate::from_json::record!(@default $($rest)*) };
    (@take $slots:ident) => { $crate::from_json::take(&mut $slots) };
    (@take $slots:ident, default) => {
        $crate::from_json::take_or(&mut $slots, ::core::default::Default::default)
    };
    (@take $slots:ident, default = $make:path) => { $crate::from_json::take_or(&mut $slots, $make) };
    (@take $slots:ident, $key:literal) => { $crate::from_json::record!(@take $slots) };
    (@take $slots:ident, $key:literal, $($rest:tt)*) => {
        $crate::from_json::record!(@take $slots, $($rest)*)
    };
    (@flatten $ty:ident) => { None };
    (@flatten $ty:ident $flat:ident) => {
        Some($crate::from_json::shape_of($crate::from_json::kind_of(|s: &$ty| &s.$flat)))
    };
    (
        $ty:ident $(, $deny:ident)? { $($f:ident $(($($opt:tt)*))?),* $(,)? }
        $(skip { $($skip:ident),* $(,)? })?
        $(flatten $flat:ident)?
    ) => {
        impl $crate::from_json::Read for $ty {
            const KIND: &'static $crate::from_json::Kind =
                &$crate::from_json::Kind::Struct(&$crate::from_json::Shape {
                    name: stringify!($ty),
                    deny: $crate::from_json::record!(@deny $($deny)?),
                    keys: &[$($crate::from_json::record!(@key $f $($($opt)*)?)),*],
                    fields: &[$($crate::from_json::Field {
                        kind: $crate::from_json::kind_of(|s: &$ty| &s.$f),
                        default: $crate::from_json::record!(@default $($($opt)*)?),
                    }),*],
                    flatten: $crate::from_json::record!(@flatten $ty $($flat)?),
                });

            fn build(node: $crate::from_json::Node) -> Self {
                let $crate::from_json::Node::Fields(slots, _flat) = node else {
                    $crate::from_json::mismatch()
                };
                let mut slots = slots.into_iter();
                Self {
                    $($f: $crate::from_json::record!(@take slots $(, $($opt)*)?),)*
                    $($($skip: ::core::default::Default::default(),)*)?
                    $($flat: match _flat {
                        Some(node) => $crate::from_json::Read::build(*node),
                        None => $crate::from_json::mismatch(),
                    },)?
                }
            }
        }
    };
}

/// 给枚举写 [`super::Read`]（也能做表的键），见本文件开头。
macro_rules! choice {
    (@payload) => { None };
    (@payload $payload:ty) => { Some(<$payload as $crate::from_json::Read>::KIND) };
    (@make $ty:ident $v:ident) => { |_| $ty::$v };
    (@make $ty:ident $v:ident $payload:ty) => {
        |node: Option<Box<$crate::from_json::Node>>| match node {
            Some(node) => $ty::$v(<$payload as $crate::from_json::Read>::build(*node)),
            None => $crate::from_json::mismatch(),
        }
    };
    ($ty:ident { $($v:ident $(($payload:ty))? = $name:literal),* $(,)? }) => {
        impl $crate::from_json::Read for $ty {
            const KIND: &'static $crate::from_json::Kind =
                &$crate::from_json::Kind::Enum(&$crate::from_json::Variants {
                    name: stringify!($ty),
                    names: &[$($name),*],
                    payloads: &[$($crate::from_json::choice!(@payload $($payload)?)),*],
                });

            fn build(node: $crate::from_json::Node) -> Self {
                let $crate::from_json::Node::Variant(at, payload) = node else {
                    $crate::from_json::mismatch()
                };
                let make: &[fn(Option<Box<$crate::from_json::Node>>) -> Self] =
                    &[$($crate::from_json::choice!(@make $ty $v $($payload)?)),*];
                match make.get(at) {
                    Some(make) => make(payload),
                    None => $crate::from_json::mismatch(),
                }
            }
        }

        impl $crate::from_json::Key for $ty {}
    };
}

pub(crate) use {choice, record};
