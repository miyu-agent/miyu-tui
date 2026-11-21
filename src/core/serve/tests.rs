//! 回答里传上去的图放进 `session.answer` 的参数（核心 Q-A）；订阅子会话的参数（LM-24）。

use serde_json::json;

use super::{FRESH, place_answer_files, watch_params};

/// LM-24（2026-11-21，LiteMiyu 撞见、终端这边同样有）：新订的子会话要从头补，订阅以前已经开始的那一步（正在跑的命令）才补得上，
/// 子代理状态行写得出它正在做什么；重连的照读到的那一条往后补。
#[test]
fn a_new_child_subscription_catches_up_from_the_start() {
    assert_eq!(
        watch_params("s-child", FRESH),
        json!({"session": "s-child", "stream": "events", "after": 0})
    );
    assert_eq!(watch_params("s-child", Some(41))["after"], 41);
}

#[test]
fn answer_images_go_to_their_question_and_deny_images_go_outside() {
    let mut params = json!({"answers": [{"picked": ["甲"]}, {}]});
    let blob = |b: &str| json!({"blob": b, "name": "a.png", "media_type": "image/png"});
    place_answer_files(
        &mut params,
        vec![(Some(1), blob("b1")), (Some(1), blob("b2"))],
    );
    assert_eq!(
        params["answers"][0].get("attachments"),
        None,
        "没贴的那道不写"
    );
    assert_eq!(
        params["answers"][1]["attachments"],
        json!([blob("b1"), blob("b2")])
    );
    let mut deny = json!({"decision": "deny"});
    place_answer_files(&mut deny, vec![(None, blob("b3"))]);
    assert_eq!(deny["attachments"], json!([blob("b3")]));
}
