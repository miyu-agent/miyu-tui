//! 附的图片、文件画成块（核心 C-9）。

use crate::config::Config;
use crate::core::{Attached, Push, Update};
use crate::transcript::{Kind, Transcript};

fn image(path: Option<&str>) -> Attached {
    Attached {
        media_type: "image/png".into(),
        name: Some("晚霞.png".into()),
        path: path.map(str::to_string),
        path_only: false,
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
        path_only: false,
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

fn path_only(name: &str) -> Attached {
    Attached {
        media_type: String::new(),
        name: Some(name.into()),
        path: Some(format!("/home/me/{name}")),
        path_only: true,
    }
}

#[test]
fn a_path_only_attachment_is_its_kinds_block_with_a_note() {
    // 核心 AT-1（2026-11-11 项目主人：超过上传上限的只把路径交给她）：补发来的那句字里写着 `[视频 1]`，只给了路径的那一项
    // 没有媒体类型，种类照字里第几个块认；块指着那个路径，后面记一句「仅路径」。
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.update(
        Update::Push(Push::Said {
            seq: 4,
            text: "看看 [图片 1] 和 [视频 1]".into(),
            attached: vec![image(Some("/home/me/a.png")), path_only("旅行.mp4")],
        }),
        &config.text,
    );
    let entry = &t.entries[0];
    assert_eq!(entry.text, "看看 [图片 1] 和 [视频 1]", "字不动");
    let chips: Vec<(&str, Option<&str>, Option<&str>)> = entry
        .pasted
        .iter()
        .map(|c| (c.label.as_str(), c.kind.as_deref(), c.note.as_deref()))
        .collect();
    assert_eq!(
        chips,
        [
            ("[图片 1]", Some("image"), None),
            ("[视频 1]", Some("video"), Some("仅路径"))
        ]
    );
    assert_eq!(
        entry.pasted[1].file.as_deref(),
        Some(std::path::Path::new("/home/me/旅行.mp4")),
        "放回输入框、点块都照这个路径"
    );
    // 字里没写块的（别的头发的）：照文件块写名字。
    t.foreign(Some(9), "从 B 收到消息".into(), "给你".into());
    t.attach_to(9, &[path_only("disk.iso")], &config.text.attach);
    let other = t.entries.iter().find(|e| e.seq == Some(9)).unwrap();
    assert_eq!(other.text, "给你 [disk.iso]");
    assert_eq!(other.pasted[0].kind.as_deref(), Some("file"));
    assert_eq!(other.pasted[0].note.as_deref(), Some("仅路径"));
}

#[test]
fn a_block_already_drawn_gets_the_note_when_the_core_says_it_went_as_a_path() {
    // 你在这个界面发的那句：块发之前就画上了，传的时候核心说太大才改成只给路径；核心推来那一句时照先后补上那一句。
    use crate::transcript::Chip;
    let config = Config::builtin().unwrap();
    let mut t = Transcript::default();
    t.note(Kind::User, "[图片 1] [视频 1]".into());
    t.entries[0].seq = Some(7);
    let chip = |label: &str, kind: &str| Chip {
        label: label.into(),
        full: label.into(),
        kind: Some(kind.into()),
        file: Some(format!("/home/me/{kind}").into()),
        note: None,
    };
    t.entries[0].pasted = vec![chip("[图片 1]", "image"), chip("[视频 1]", "video")];
    let sent = [image(Some("/home/me/a.png")), path_only("旅行.mp4")];
    t.attach_to(7, &sent, &config.text.attach);
    let notes: Vec<Option<&str>> = t.entries[0]
        .pasted
        .iter()
        .map(|c| c.note.as_deref())
        .collect();
    assert_eq!(notes, [None, Some("仅路径")]);
    assert_eq!(t.entries[0].pasted.len(), 2, "不再画一遍");
}
