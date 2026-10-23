//! 回答里传上去的图放进 `session.answer` 的参数（核心 Q-A）。

use serde_json::json;

use super::place_answer_files;

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
