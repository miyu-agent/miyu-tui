//! 配置页的「开机启动」：通用页最后一组一行，`Enter` 当场装上、拿掉，跑完照装没装画、状态行说一声。

use serde_json::{Value, json};

use super::{KEY, add_row};
use crate::config::Config;
use crate::service::{Reply, Verb};
use crate::settings::pages::schema::Schema;
use crate::settings::test_support::{config_all, model_list};
use crate::settings::{Settings, Texts, Tone};

fn texts() -> Texts {
    Config::builtin().unwrap().text.settings
}

fn opened() -> Settings {
    let mut page = Settings::open(false, true);
    for (tag, method, _) in page.take_asks() {
        let got = match method {
            "model.list" => model_list(),
            "config.get" => config_all(),
            _ => json!({"secrets": []}),
        };
        page.answer(tag, Ok(got), &texts());
    }
    page
}

#[test]
fn the_row_goes_last_on_the_general_page_once() {
    let mut schema = Schema::read(&json!({
        "pages": [{"id": "general", "name": "通用"}, {"id": "advanced", "name": "高级"}],
        "groups": [
            {"id": "display", "name": "显示", "page": "general"},
            {"id": "logs", "name": "日志", "page": "advanced"}
        ],
        "items": [{"key": "display.language", "control": "select", "layers": ["personal"],
                   "name": "语言", "page": "general", "group": "display"}]
    }));
    let words = &texts().more.service;
    add_row(&mut schema, words, false);
    assert!(
        schema.items.iter().all(|i| i.key != KEY),
        "不知道核心程序在哪的不加"
    );
    add_row(&mut schema, words, true);
    add_row(&mut schema, words, true);
    let ids: Vec<&str> = schema.groups.iter().map(|g| g.id.as_str()).collect();
    assert_eq!(ids, ["display", KEY, "logs"]);
    let rows: Vec<&str> = schema.items.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(rows, ["display.language", KEY], "加一次");
    assert_eq!(schema.items[1].name, "开机启动核心");
    // 没有通用页的（核心旧）不加。
    let mut bare = Schema::default();
    add_row(&mut bare, words, true);
    assert!(bare.items.is_empty());
}

#[test]
fn enter_runs_it_now_and_the_reply_sets_the_switch_and_the_status_line() {
    let texts = texts();
    let mut page = opened();
    page.toggle_service(false, &texts);
    assert_eq!(page.take_service_runs(), [Verb::Enable]);
    assert_eq!(page.status.as_ref().map(|s| s.1), Some(Tone::Busy));
    assert!(page.take_asks().is_empty(), "不发 config.set");
    let file = "/home/a/.config/systemd/user/miyu.service".to_string();
    page.service_replied(
        &Reply::Done {
            verb: Verb::Enable,
            installed: true,
            on: Some(true),
            file: file.clone(),
        },
        &texts,
    );
    let shown = |page: &Settings| page.view.values.get(KEY).map(|(v, _)| v.clone());
    assert_eq!(shown(&page), Some(Value::Bool(true)));
    let (said, tone) = page.status.clone().unwrap();
    assert!(said.contains(&file) && tone == Tone::Good, "{said}");
    page.toggle_service(true, &texts);
    assert_eq!(page.take_service_runs(), [Verb::Disable]);
    // 没成：说哪一步、系统的原话；开关不动。
    page.service_replied(
        &Reply::Failed {
            verb: Verb::Disable,
            code: "failed".into(),
            step: "systemctl --user disable".into(),
            said: "Failed to connect to bus".into(),
        },
        &texts,
    );
    let (said, tone) = page.status.clone().unwrap();
    assert_eq!(
        said,
        "没成：systemctl --user disable：Failed to connect to bus"
    );
    assert_eq!(tone, Tone::Bad);
    assert_eq!(shown(&page), Some(Value::Bool(true)));
    // 问状态没成的（核心旧）不说。
    page.say("别的".into(), Tone::Note);
    page.service_replied(
        &Reply::Failed {
            verb: Verb::Status,
            code: "failed".into(),
            step: String::new(),
            said: "?".into(),
        },
        &texts,
    );
    assert_eq!(page.status.clone().unwrap().0, "别的");
}
