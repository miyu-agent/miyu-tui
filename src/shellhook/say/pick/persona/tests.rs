//! `/persona`、`/preset` 写到哪。

use serde_json::json;

use super::PersonaPick;
use crate::app::new_session::Kind;
use crate::config::Config;
use crate::core::{Command, Persona, Update, detached};
use crate::shellhook::say::look::Look;
use crate::shellhook::say::pick::Picker;
use crate::shellhook::say::turn::Turn;
use crate::shellhook::words::Words;

#[test]
fn a_chosen_preset_is_written_to_the_system_config_without_a_layer() {
    // 核心 SU-3 上起设置只剩系统配置一份：`config.set` 不写 `layer`（写 `personal` 核心回 `bad_params`）。
    let config = Config::builtin().unwrap();
    let words = Words::of("zh").unwrap();
    let mut pick = PersonaPick::new(Kind::Preset, Some("full"), &words, &config);
    let turn = Turn::new(String::new(), None, Look::new(None));
    let preset = Persona {
        id: "full".into(),
        name: Some("全开".into()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
    };
    pick.core(&Update::Presets(vec![preset]), &turn);
    pick.core(
        &Update::Answer {
            tag: super::READ_TAG,
            result: Ok(json!({"items": {}})),
        },
        &turn,
    );
    let (core, mut sent) = detached();
    let mut turn = turn;
    pick.ready(&mut turn, &core, &config);
    let Ok(Command::Ask { method, params, .. }) = sent.try_recv() else {
        panic!("没写设置");
    };
    assert_eq!(method, "config.set");
    assert!(params.get("layer").is_none(), "不写层：{params}");
    assert_eq!(
        params["changes"],
        json!([{"key": "terminal.preset", "value": "full"}])
    );
}
