//! 她给人看的文件（蓝图「展示」第 3–5 条；核心 SW-1 起照 `path` 读，改过、没了的写「文件已变」「文件已不在」）。

use std::path::{Path, PathBuf};

use super::{rows, size};
use crate::core::Shown;
use crate::transcript::{Entry, Kind};
use crate::ui::test_support::Fixture;

/// 不在硬盘上的一份（只看卡片的样子）。
fn shown(id: u64, name: &str, media_type: &str, size: u64, title: Option<&str>) -> Entry {
    at(
        id,
        &PathBuf::from(format!("/nonexistent-miyu/{name}")),
        media_type,
        size,
        title,
    )
}

/// 硬盘上 `path` 那一份。
fn at(id: u64, path: &Path, media_type: &str, size: u64, title: Option<&str>) -> Entry {
    let mut entry = Entry::new(id, Kind::Shown, String::new());
    entry.shown = Some(Shown {
        path: path.display().to_string(),
        modified: None,
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        media_type: media_type.into(),
        size,
        title: title.map(Into::into),
        language: None,
    });
    entry
}

/// 测试用的目录：照名字和进程号分开。
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("miyu-shown-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

/// 这一套里这种文件的图标，后面空一格。
fn icon(f: &Fixture, key: &str) -> String {
    format!("{} ", f.config.icons.files[key])
}

#[test]
fn a_file_is_a_card_with_its_icon_kind_size_and_lines() {
    let f = Fixture::new();
    let dir = scratch("rows");
    let page_path = dir.join("index.html");
    std::fs::write(&page_path, vec![b'x'; 4300]).unwrap();
    let page = at(1, &page_path, "text/html", 4300, Some("首页"));
    assert_eq!(
        texts(&page, &f, false),
        [
            format!("{}首页  index.html", icon(&f, "html")),
            "HTML · 4.2 KB · 1 行".into(),
            "点击打开".into()
        ]
    );
    // 文本照 `path` 读出来数几行。
    let body = "# 标题\n\n一段\n";
    let note_path = dir.join("notes.md");
    std::fs::write(&note_path, body).unwrap();
    let note = at(2, &note_path, "text/markdown", body.len() as u64, None);
    assert_eq!(
        texts(&note, &f, false),
        [
            format!("{}notes.md", icon(&f, "markdown")),
            format!("Markdown · {} · 3 行", size(body.len() as u64)),
            "点击打开".into()
        ]
    );
    let same = shown(4, "page.html", "text/html", 10, Some("page.html"));
    assert_eq!(
        texts(&same, &f, false)[0],
        format!("{}page.html", icon(&f, "html")),
        "标题就是文件名的不重复"
    );
    let pdf = shown(3, "a.pdf", "application/pdf", 2 * 1024 * 1024, None);
    assert_eq!(texts(&pdf, &f, false)[1], "PDF · 2.0 MB", "PDF 不数行");
    drop(std::fs::remove_dir_all(&dir));
}

/// 核心 SW-1：内容不进核心，展示以后文件改了、删了，卡片最后一行写「文件已变」「文件已不在」。
#[test]
fn a_changed_or_missing_file_says_so_on_its_card() {
    let f = Fixture::new();
    let dir = scratch("changed");
    let path = dir.join("a.html");
    std::fs::write(&path, "<p>hi</p>").unwrap();
    let changed = at(1, &path, "text/html", 999, None);
    assert_eq!(texts(&changed, &f, false).last().unwrap(), "文件已变");
    let gone = shown(2, "b.html", "text/html", 9, None);
    assert_eq!(texts(&gone, &f, false).last().unwrap(), "文件已不在");
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn an_earlier_showing_of_the_same_path_keeps_its_card_and_says_updated() {
    // 2026-10-17 项目主人：不缩成一行，照常三行，最后一行换成「已更新」。
    let f = Fixture::new();
    let page = shown(1, "index.html", "text/html", 4300, None);
    assert_eq!(
        texts(&page, &f, true),
        [
            format!("{}index.html", icon(&f, "html")),
            "HTML · 4.2 KB".into(),
            "已更新".into()
        ]
    );
}

#[test]
fn every_kind_of_file_has_its_icon() {
    let f = Fixture::new();
    for (name, media, language, key) in [
        ("a.html", "text/html", None, "html"),
        ("a.md", "text/markdown", None, "markdown"),
        ("a.rs", "text/plain", Some("rust"), "code"),
        ("a.txt", "text/plain", None, "text"),
        ("a.pdf", "application/pdf", None, "pdf"),
        ("a.bin", "application/octet-stream", None, "file"),
    ] {
        let mut entry = shown(9, name, media, 1, None);
        entry.shown.as_mut().unwrap().language = language.map(Into::into);
        let first = texts(&entry, &f, false)[0].clone();
        assert!(first.starts_with(&icon(&f, key)), "{name}：{first}");
    }
}

#[test]
fn sizes_read_like_files() {
    assert_eq!(size(0), "0 B");
    assert_eq!(size(1023), "1023 B");
    assert_eq!(size(1536), "1.5 KB");
    assert_eq!(size(3 * 1024 * 1024), "3.0 MB");
}

#[test]
fn only_the_picture_itself_opens_it_not_the_blank_to_its_right() {
    // 2026-10-22 项目主人：show 出来的图点击判定是一整行，图右边的空档也是手形、点了也开。
    let f = Fixture::new();
    let (figures, jobs) = crate::figures::queued(8);
    *f.ctx().figures.borrow_mut() = figures;
    let dir = scratch("pick");
    let path = dir.join("cat.png");
    std::fs::write(&path, b"png").unwrap();
    let entry = at(7, &path, "image/png", 3, None);
    assert!(rows(&entry, &f.ctx())[0].figure_pending, "先排队做图");
    crate::figures::finish(&mut f.ctx().figures.borrow_mut(), &jobs, 100, 3);
    let got = rows(&entry, &f.ctx());
    assert_eq!(got.len(), 3, "图三行");
    for row in &got {
        assert_eq!(
            row.links(),
            [(0, 10, "miyu-shown:7".to_string())],
            "只有图那 10 列能点"
        );
    }
    drop(std::fs::remove_dir_all(&dir));
}
