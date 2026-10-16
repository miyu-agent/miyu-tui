//! 她发出去的图（蓝图「她发的图」）。

use std::path::PathBuf;

use ratatui_image::picker::{Picker, ProtocolType};

use super::{figure, rows};
use crate::core::Sent;
use crate::figures::{Figures, Graphics};
use crate::markdown::FigureKind;
use crate::theme;
use crate::transcript::{Entry, Kind};
use crate::ui::test_support::Fixture;

fn sticker(alt: Option<&str>, name: Option<&str>) -> Entry {
    let mut entry = Entry::new(1, Kind::Media, String::new());
    entry.media = Some(Sent {
        blob: "sha256:aa".into(),
        name: name.map(Into::into),
        alt: alt.map(Into::into),
        sticker: true,
    });
    entry
}

/// 能画图的终端。
fn with_pictures(f: &Fixture) {
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(ProtocolType::Kitty);
    *f.ctx().figures.borrow_mut() =
        Figures::start(Some(Graphics { picker }), &f.config.figures, None, |_| true);
}

fn texts(entry: &Entry, f: &Fixture) -> Vec<String> {
    rows(entry, &f.ctx())
        .iter()
        .map(|r| r.line.to_string().trim().to_string())
        .collect()
}

#[test]
fn a_terminal_without_pictures_writes_the_sticker_and_its_words() {
    let f = Fixture::new();
    let entry = sticker(Some("摊手"), Some("shrug.png"));
    assert!(crate::ui::rows::shown(&entry), "没有字也画");
    let got = rows(&entry, &f.ctx());
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].line.to_string().trim(), "[表情包] 摊手");
    assert_eq!(got[0].line.spans.last().unwrap().style, theme::md_image());
    assert!(got[0].copy, "那一行字照常复制");
    assert_eq!(
        texts(&sticker(None, Some("shrug.png")), &f),
        ["[表情包] shrug.png"],
        "没有描述的写文件名"
    );
    assert_eq!(texts(&sticker(None, None), &f), ["[表情包]"]);
    let mut photo = sticker(Some("窗外"), None);
    photo.media.as_mut().unwrap().sticker = false;
    assert_eq!(texts(&photo, &f), ["[图片] 窗外"]);
    let (_, blobs) = f.cards.borrow_mut().take();
    assert!(blobs.is_empty(), "显示不了图的不去读图");
}

#[test]
fn a_picture_not_back_yet_waits_and_one_that_cannot_be_read_writes_its_words() {
    let f = Fixture::new();
    with_pictures(&f);
    let entry = sticker(Some("摊手"), None);
    let got = rows(&entry, &f.ctx());
    assert!(
        got.len() == 1 && got[0].figure_pending,
        "没回来是「正在渲染图片」那一行"
    );
    let (_, blobs) = f.cards.borrow_mut().take();
    assert_eq!(blobs, ["sha256:aa"], "记进单子照 blob.get 读");
    f.cards.borrow_mut().saved("sha256:aa".into(), None);
    assert_eq!(texts(&entry, &f), ["[表情包] 摊手"], "读不成的不一直等");
}

#[test]
fn a_picture_read_back_is_drawn_as_a_sticker_at_its_own_size() {
    let f = Fixture::new();
    with_pictures(&f);
    f.cards.borrow_mut().saved(
        "sha256:aa".into(),
        Some(PathBuf::from("/cache/blobs/aa.png")),
    );
    let entry = sticker(Some("摊手"), None);
    assert!(
        !texts(&entry, &f).iter().any(|t| t.contains("表情包")),
        "画得出图的不写那一行字"
    );
    let sent = entry.media.as_ref().unwrap();
    let got = figure(sent, "/cache/blobs/aa.png".into());
    assert_eq!(
        got.kind,
        FigureKind::Sticker,
        "表情包照 sticker_room 画，比回答里的图小"
    );
    assert_eq!(
        (got.width, got.height),
        (None, None),
        "大小照图本身，不照扩展报的"
    );
    let mut photo = sent.clone();
    photo.sticker = false;
    assert_eq!(figure(&photo, String::new()).kind, FigureKind::Image);
}
