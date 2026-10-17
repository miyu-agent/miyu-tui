//! 她给人看的文件（蓝图「展示」第 3–5 条）。

use super::{rows, size};
use crate::core::Shown;
use crate::transcript::{Entry, Kind};
use crate::ui::test_support::Fixture;

fn shown(id: u64, name: &str, media_type: &str, size: u64, title: Option<&str>) -> Entry {
    let mut entry = Entry::new(id, Kind::Shown, String::new());
    entry.shown = Some(Shown {
        path: format!("/w/{name}"),
        blob: format!("sha256:{id:02}"),
        name: name.into(),
        media_type: media_type.into(),
        size,
        title: title.map(Into::into),
        language: None,
    });
    entry
}

fn texts(entry: &Entry, f: &Fixture, replaced: bool) -> Vec<String> {
    let mut ctx = f.ctx();
    if replaced {
        ctx.replaced.insert(entry.id);
    }
    rows(entry, &ctx)
        .iter()
        .map(|r| r.line.to_string().trim().to_string())
        .collect()
}

#[test]
fn a_file_is_a_card_with_its_kind_size_and_lines() {
    let f = Fixture::new();
    // 读回来以前不写几行。
    let page = shown(1, "index.html", "text/html", 4300, Some("首页"));
    assert_eq!(
        texts(&page, &f, false),
        ["首页  index.html", "HTML · 4.2 KB", "点击打开"]
    );
    // 读回来以后数几行。
    let dir = std::env::temp_dir().join(format!("miyu-shown-rows-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let saved = dir.join("blob");
    std::fs::write(&saved, "# 标题\n\n一段\n").unwrap();
    let note = shown(2, "notes.md", "text/markdown", 20, None);
    f.cards
        .borrow_mut()
        .saved("sha256:02".into(), Some(saved.clone()));
    assert_eq!(
        texts(&note, &f, false),
        ["notes.md", "Markdown · 20 B · 3 行", "点击打开"]
    );
    let same = shown(4, "page.html", "text/html", 10, Some("page.html"));
    assert_eq!(
        texts(&same, &f, false)[0],
        "page.html",
        "标题就是文件名的不重复"
    );
    let pdf = shown(3, "a.pdf", "application/pdf", 2 * 1024 * 1024, None);
    assert_eq!(texts(&pdf, &f, false)[1], "PDF · 2.0 MB", "PDF 不数行");
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn an_earlier_showing_of_the_same_path_is_one_dim_line() {
    let f = Fixture::new();
    let page = shown(1, "index.html", "text/html", 4300, None);
    assert_eq!(texts(&page, &f, true), ["index.html · 已更新"]);
}

#[test]
fn sizes_read_like_files() {
    assert_eq!(size(0), "0 B");
    assert_eq!(size(1023), "1023 B");
    assert_eq!(size(1536), "1.5 KB");
    assert_eq!(size(3 * 1024 * 1024), "3.0 MB");
}
