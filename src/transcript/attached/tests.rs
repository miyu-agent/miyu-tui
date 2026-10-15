//! 附的图片、文件画成块（核心 C-9）。

use crate::config::Config;
use crate::core::{Attached, Push, Update};
use crate::transcript::{Kind, Transcript};

fn image(path: Option<&str>) -> Attached {
    Attached {
        media_type: "image/png".into(),
        name: Some("晚霞.png".into()),
        path: path.map(str::to_string),
    }
}

#[test]
fn a_replayed_message_keeps_the_labels_it_was_sent_with() {
    // 补发来的你说的话：字里已经写着 `[图片 1]`，块照先后对上，不再写一遍。
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.update(
        Update::Push(Push::Said {
            seq: 4,
            text: "看看 [图片 1] 和 [图片 2]".into(),
            attached: vec![image(Some("/home/me/a.png")), image(None)],
        }),
        &config.text,
    );
    let entry = &t.entries[0];
    assert_eq!(entry.text, "看看 [图片 1] 和 [图片 2]");
    let labels: Vec<&str> = entry.pasted.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["[图片 1]", "[图片 2]"]);
    assert_eq!(
        entry.pasted[0].file.as_deref(),
        Some(std::path::Path::new("/home/me/a.png")),
        "点块打开原来那个文件"
    );
    assert_eq!(entry.pasted[1].file, None, "没给位置的");
}

#[test]
fn attachments_from_another_session_follow_the_text_and_count_on() {
    // 核心 C-9：别的会话发来的话只有字和块，字里没有块上的字：接在后面，编号照正文里这一种已有的接着数。
    let config = Config::builtin().unwrap();
    let words = &config.text.attach;
    let mut t = Transcript::default();
    t.update(
        Update::Push(Push::Said {
            seq: 1,
            text: "[图片 1]".into(),
            attached: vec![image(None)],
        }),
        &config.text,
    );
    t.foreign(Some(2), "从 B 收到消息".into(), "看这些".into());
    let zip = Attached {
        media_type: "application/zip".into(),
        name: Some("data.zip".into()),
        path: Some("/tmp/data.zip".into()),
    };
    t.attach_to(2, &[image(Some("/tmp/b.png")), zip], words);
    let entry = t.entries.iter().find(|e| e.seq == Some(2)).unwrap();
    assert_eq!(entry.kind, Kind::User);
    assert_eq!(entry.text, "看这些 [图片 2] [data.zip]");
    let kinds: Vec<Option<&str>> = entry.pasted.iter().map(|c| c.kind.as_deref()).collect();
    assert_eq!(kinds, [Some("image"), Some("file")]);
    assert!(
        entry.pasted.iter().all(|c| !c.expandable()),
        "附件不展开，点块打开文件"
    );
    assert_eq!(t.attachment_counts()["image"], 2);
}
