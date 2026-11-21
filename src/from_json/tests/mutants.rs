//! 照一份好的 JSON 造写坏的样本：少一个键、多一个不认识的键、换成别的类型、空对象、一串多一个少一个、同一个键写两次，
//! 还有两处一起坏的（看先报哪一处）。大的资源照固定的步长挑一部分格子，每次跑挑的都一样。

use serde_json::{Map, Value, json};

/// 每份资源最多挑几个格子换类型。
const PICKS: usize = 40;

/// 换类型用的几种值：数、字、`null`、真假、小数、负数、放不进 u16 的数、空的和不空的一串、空表。
fn replacements() -> [Value; 11] {
    [
        json!(5),
        json!("x"),
        Value::Null,
        json!(true),
        json!(1.5),
        json!(-1),
        json!(70000),
        json!([]),
        json!(["a", "b", "c"]),
        json!(["a"]),
        json!({}),
    ]
}

/// 每一格的位置（JSON Pointer），照读的先后。
fn pointers(value: &Value, at: &str, out: &mut Vec<String>) {
    out.push(at.to_string());
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                pointers(
                    v,
                    &format!("{at}/{}", key.replace('~', "~0").replace('/', "~1")),
                    out,
                );
            }
        }
        Value::Array(list) => {
            for (i, v) in list.iter().enumerate() {
                pointers(v, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

fn with(value: &Value, at: &str, change: impl FnOnce(&mut Value)) -> Value {
    let mut out = value.clone();
    if let Some(slot) = out.pointer_mut(at) {
        change(slot);
    }
    out
}

fn object(value: &mut Value) -> Option<&mut Map<String, Value>> {
    value.as_object_mut()
}

/// 写坏的样本：说明、值。
pub fn of(value: &Value) -> Vec<(String, Value)> {
    let mut all = Vec::new();
    pointers(value, "", &mut all);
    let mut out = Vec::new();
    let step = all.len().div_ceil(PICKS).max(1);
    for (n, at) in all.iter().enumerate() {
        let here = value.pointer(at).unwrap_or(&Value::Null);
        if n % step == 0 {
            for (k, v) in replacements().into_iter().enumerate() {
                out.push((
                    format!("{at} 换成第 {k} 种"),
                    with(value, at, |slot| *slot = v),
                ));
            }
            // 两处一起坏：这一格换成数，另加一个排在最前、一个排在最后的不认识的键。
            let parent = at.rsplit_once('/').map_or("", |(p, _)| p);
            if !at.is_empty() {
                let mut two = with(value, at, |slot| *slot = json!(5));
                if let Some(map) = two.pointer_mut(parent).and_then(object) {
                    map.insert("_a".into(), json!(1));
                    map.insert("zz_unknown".into(), json!(1));
                }
                out.push((format!("{at} 换成数、上一层多两个键"), two));
            }
        }
        match here {
            Value::Object(map) if !map.is_empty() => {
                let pick = map.keys().nth(n % map.len()).cloned().unwrap_or_default();
                out.push((
                    format!("{at} 少了 {pick}"),
                    with(value, at, |slot| {
                        object(slot).map(|m| m.remove(&pick));
                    }),
                ));
                let last = map.keys().next_back().cloned().unwrap_or_default();
                let first = map.keys().next().cloned().unwrap_or_default();
                out.push((
                    format!("{at} 少了头尾两个"),
                    with(value, at, |slot| {
                        if let Some(m) = object(slot) {
                            m.remove(&first);
                            m.remove(&last);
                        }
                    }),
                ));
                out.push((
                    format!("{at} 多一个键"),
                    with(value, at, |slot| {
                        object(slot).map(|m| m.insert("zz_unknown".into(), json!(1)));
                    }),
                ));
                out.push((
                    format!("{at} 多一个排在前面的键"),
                    with(value, at, |slot| {
                        object(slot).map(|m| m.insert("_a".into(), json!({"x": 1})));
                    }),
                ));
                out.push((
                    format!("{at} 空对象"),
                    with(value, at, |slot| *slot = json!({})),
                ));
                out.push((
                    format!("{at} 写成一串"),
                    with(value, at, |slot| {
                        *slot = Value::Array(map.values().cloned().collect())
                    }),
                ));
            }
            Value::Array(list) if !list.is_empty() => {
                out.push((
                    format!("{at} 少最后一个"),
                    with(value, at, |slot| {
                        slot.as_array_mut().map(Vec::pop);
                    }),
                ));
                let first = list[0].clone();
                out.push((
                    format!("{at} 多一个"),
                    with(value, at, |slot| {
                        if let Some(list) = slot.as_array_mut() {
                            list.push(first);
                        }
                    }),
                ));
            }
            _ => {}
        }
    }
    out
}

/// 原文里同一个键写两次：挑几行单行的 `"键": 值,` 原样再写一行。
pub fn doubled(json: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = json.lines().collect();
    let single = |l: &&str| {
        let t = l.trim_start();
        t.starts_with('"') && t.ends_with(',') && t.contains("\": ") && !t.ends_with("{,")
    };
    let picks: Vec<usize> = (0..lines.len()).filter(|&i| single(&lines[i])).collect();
    let step = picks.len().div_ceil(4).max(1);
    picks
        .iter()
        .step_by(step)
        .map(|&i| {
            let mut copy = lines.clone();
            copy.insert(i, lines[i]);
            (format!("第 {} 行写两次", i + 1), copy.join("\n"))
        })
        .collect()
}
