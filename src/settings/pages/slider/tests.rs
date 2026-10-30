use serde_json::{Value, json};

use super::{fraction, nudge};
use crate::settings::pages::schema::Schema;

/// 两个滑块：小数的温度（0～2，一步 0.1）、整数的步数（1～50，一步 5）。
fn items() -> Schema {
    let item = |key: &str, kind: &str, min: i64, max: i64, step: Value| {
        json!({"key": key, "type": kind, "control": "slider", "layers": ["system"], "name": key,
               "page": "packages", "group": "draw", "min": min, "max": max, "step": step})
    };
    Schema::read(&json!({"pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "draw", "name": "画图", "page": "packages"}],
        "items": [item("t", "float", 0, 2, json!(0.1)), item("s", "int", 1, 50, json!(5)),
                  item("n", "float", 0, 4, Value::Null)]}))
}

#[test]
fn a_step_snaps_to_the_grid_stays_in_range_and_keeps_the_type() {
    let s = items();
    let (t, n) = (&s.items[0], &s.items[1]);
    assert_eq!(
        nudge(t, &json!(0.2), true),
        Some(json!(0.3)),
        "不写出 0.30000000000000004"
    );
    assert_eq!(nudge(t, &json!(2.0), true), Some(json!(2.0)), "到顶停");
    assert_eq!(nudge(t, &json!(0.0), false), Some(json!(0.0)), "到底停");
    assert_eq!(
        nudge(t, &Value::Null, true),
        Some(json!(0.1)),
        "没值的从最小起"
    );
    assert_eq!(nudge(n, &json!(1), true), Some(json!(6)), "整数存整数");
    assert_eq!(
        nudge(n, &json!(7), true),
        Some(json!(11)),
        "对齐到从最小起的格子上"
    );
    assert_eq!(nudge(n, &json!(48), true), Some(json!(50)));
    let default_step = &s.items[2];
    assert_eq!(
        default_step.slide,
        Some((0.0, 4.0, 0.04)),
        "没写一步的小数是范围的百分之一"
    );
}

#[test]
fn the_fraction_places_the_dot() {
    let s = items();
    assert_eq!(fraction(&s.items[0], &json!(1.0)), Some(0.5));
    assert_eq!(fraction(&s.items[0], &json!(9)), Some(1.0));
    assert_eq!(fraction(&s.items[0], &json!("x")), None);
}
