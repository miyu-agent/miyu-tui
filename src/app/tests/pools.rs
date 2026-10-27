//! `/model` 里固定的三个池写给人看的名字（2026-10-26 项目主人：旗舰、日常、轻量，`pools.rs`）。

use super::app;
use crate::core::{Choice, ChoiceState};

fn choice(reference: &str) -> Choice {
    Choice {
        reference: reference.into(),
        name: reference.into(),
        detail: "pin · 2".into(),
        state: ChoiceState::Ok,
        shared: None,
    }
}

#[test]
fn the_fixed_pools_show_their_names_and_keep_the_reference_in_the_detail() {
    let (mut app, _sent) = app();
    app.boxes.model_list = Some(crate::model_list::ModelList::default());
    app.models_listed(vec![choice("@flagship"), choice("@duo"), choice("dev/m1")]);
    let all = &app.boxes.model_list.as_ref().unwrap().all;
    assert_eq!(all[0].name, "旗舰");
    assert_eq!(
        all[0].detail, "@flagship · pin · 2",
        "引用挪到暗色那一截前面"
    );
    assert_eq!(all[0].reference, "@flagship", "交给核心的照旧");
    assert_eq!(all[1].name, "@duo", "人建的池照旧");
    assert_eq!(all[2].name, "dev/m1");
}
