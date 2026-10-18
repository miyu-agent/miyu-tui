//! 扩展工具交的三种通用形状的 `detail`（核心 W-11 补）：点开换掉结果的原文；出错的、形状不认得的照原文。

use std::path::PathBuf;
use std::time::Instant;

use ratatui_image::picker::{Picker, ProtocolType};
use serde_json::{Value, json};

use super::{rows, segment, step, text};
use crate::core::ToolStatus;
use crate::figures::{Figures, Graphics};
use crate::theme;
use crate::transcript::{Step, StepKind, ToolState};
use crate::ui::test_support::Fixture;

fn tool(status: ToolStatus, detail: Value) -> Step {
    let mut s = step(
        StepKind::Tool {
            name: "web_search".into(),
            args: String::new(),
            parsed: json!({"query": "芒果"}),
            state: ToolState::Done(status),
            output: "1. Mango - https://a.example/mango".into(),
            said: None,
        },
        Instant::now(),
        0,
        1,
    );
    s.detail = Some(detail);
    s.open = Some(true);
    s
}

fn opened(f: &Fixture, step: Step) -> Vec<crate::ui::rows::Row> {
    rows(0, &segment(vec![step], Some(true)), &f.ctx())
}

#[test]
fn links_show_a_clickable_title_then_site_and_date_then_the_snippet() {
    let detail = json!({"kind": "links", "links": [
        {"title": "芒果的种类", "url": "https://a.example/mango", "site": "a.example", "date": "2026-10-01", "snippet": "芒果有很多种"},
        {"title": "荔枝", "url": "https://b.example/lychee"}
    ]});
    let f = Fixture::new();
    let got = opened(&f, tool(ToolStatus::Ok, detail));
    let shown = text(&got);
    let at = |s: &str| {
        shown
            .iter()
            .position(|l| l.contains(s))
            .unwrap_or_else(|| panic!("{s}：{shown:?}"))
    };
    assert!(at("芒果的种类") < at("a.example · 2026-10-01"), "{shown:?}");
    assert!(
        at("a.example · 2026-10-01") < at("芒果有很多种"),
        "{shown:?}"
    );
    assert!(
        shown[at("芒果有很多种") + 1].trim().is_empty(),
        "两条之间空一行：{shown:?}"
    );
    assert!(
        shown.iter().all(|l| !l.contains("1. Mango")),
        "不画结果的原文：{shown:?}"
    );
    let title = &got[at("芒果的种类")];
    let x = title.plain.find("芒果").unwrap();
    assert_eq!(x, 0, "标题从内容开头起");
    assert_eq!(
        title.links,
        [(0, 10, "https://a.example/mango".to_string())],
        "标题整截能点"
    );
    assert!(
        title
            .line
            .spans
            .iter()
            .any(|s| s.content.contains("芒果的种类") && s.style == theme::md_link()),
        "标题是链接色"
    );
    let site = &got[at("a.example · 2026-10-01")];
    assert!(site.links.is_empty(), "站点那行不能点");
    assert!(
        site.line
            .spans
            .iter()
            .any(|s| s.content.contains("a.example") && s.style == theme::dim()),
        "站点、日期是暗字"
    );
    assert_eq!(got[at("荔枝")].links.len(), 1, "没有站点、摘要的只有标题");
}

#[test]
fn a_table_is_drawn_like_a_markdown_table_in_an_answer() {
    let detail = json!({"kind": "table", "columns": ["水果", "价格", "有货"], "rows": [
        ["芒果", 12.5, true],
        ["荔枝|妃子笑", null, false]
    ]});
    let f = Fixture::new();
    let shown = text(&opened(&f, tool(ToolStatus::Ok, detail)));
    let joined = shown.join("\n");
    for cell in [
        "水果",
        "价格",
        "芒果",
        "12.5",
        "true",
        "荔枝|妃子笑",
        "false",
    ] {
        assert!(joined.contains(cell), "{cell}：{shown:?}");
    }
    assert!(
        joined.contains('─') || joined.contains('┌') || joined.contains('│'),
        "画成表格：{shown:?}"
    );
    assert!(!joined.contains("| ---"), "不露 Markdown 记号：{shown:?}");
}

#[test]
fn images_go_under_the_step_with_their_caption_in_dim_text_below() {
    let detail = json!({"kind": "images", "images": [
        {"blob": "sha256:cc", "media_type": "image/png", "caption": "一只芒果"}
    ]});
    let f = Fixture::new();
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(ProtocolType::Kitty);
    *f.ctx().figures.borrow_mut() =
        Figures::start(Some(Graphics { picker }), &f.config.figures, None, |_| true);
    f.cards.borrow_mut().saved(
        "sha256:cc".into(),
        Some(PathBuf::from("/cache/blobs/cc.png")),
    );
    let got = opened(&f, tool(ToolStatus::Ok, detail));
    let shown = text(&got);
    let figure = got
        .iter()
        .position(|r| r.figure_pending || r.figure.is_some())
        .unwrap_or_else(|| panic!("画了图：{shown:?}"));
    let caption = got
        .iter()
        .position(|r| r.plain.contains("一只芒果"))
        .unwrap_or_else(|| panic!("{shown:?}"));
    assert!(caption > figure, "说明在图下面：{shown:?}");
    assert!(
        got[caption].shade && caption + 1 < got.len(),
        "说明在点开的那一块里：{shown:?}"
    );
    assert!(
        got[caption]
            .line
            .spans
            .iter()
            .any(|s| s.content.contains("一只芒果") && s.style == theme::dim()),
        "说明是暗字"
    );
    assert!(
        shown.iter().all(|l| !l.contains("1. Mango")),
        "不画结果的原文：{shown:?}"
    );
}

#[test]
fn a_failed_step_or_an_unknown_kind_shows_the_result_text() {
    let links = json!({"kind": "links", "links": [{"title": "芒果的种类", "url": "https://a.example/mango"}]});
    let f = Fixture::new();
    let shown = text(&opened(&f, tool(ToolStatus::Error, links)));
    assert!(
        shown.iter().any(|l| l.contains("1. Mango")),
        "出错的照原文：{shown:?}"
    );
    let odd = json!({"kind": "chart", "points": [1, 2]});
    let shown = text(&opened(&f, tool(ToolStatus::Ok, odd)));
    assert!(
        shown.iter().any(|l| l.contains("1. Mango")),
        "不认得的照原文：{shown:?}"
    );
    let empty = json!({"kind": "links", "links": []});
    let shown = text(&opened(&f, tool(ToolStatus::Ok, empty)));
    assert!(
        shown.iter().any(|l| l.contains("1. Mango")),
        "空的照原文：{shown:?}"
    );
}

#[test]
fn site_and_date_stay_dim_when_the_step_is_lit() {
    let detail = json!({"kind": "links", "links": [
        {"title": "芒果", "url": "https://a.example/mango", "site": "a.example", "snippet": "摘要"}
    ]});
    let f = Fixture::new();
    let lit = theme::hover();
    let each = |text: &str, style: ratatui::style::Style| {
        vec![super::super::step::Piece {
            lead: None,
            content: vec![ratatui::text::Span::styled(text.to_string(), style)],
            joined: false,
            links: Vec::new(),
        }]
    };
    let got = super::super::shapes::pieces(&detail, lit, 40, &f.ctx(), &each).unwrap();
    let style_of = |s: &str| {
        got.iter()
            .flat_map(|p| &p.content)
            .find(|span| span.content.contains(s))
            .map(|span| span.style)
    };
    assert_eq!(style_of("a.example"), Some(theme::dim()), "站点暗字");
    assert!(style_of("摘要").is_some(), "摘要也在");
}

#[test]
fn a_snippet_is_drawn_as_markdown_in_full() {
    // 2026-10-19 项目主人：网络搜索的摘要是整页正文，带着 `#`、列表；照 Markdown 画，不截（她拿到的照旧是全文）。
    let tail = "最后一句在这里。";
    let page = format!(
        "# Rust 1.90.0\n\n- **Tag**: 1.90.0\n\n{}{tail}",
        "Split up the lint into four finer-grained lints. ".repeat(20)
    );
    let detail = json!({"kind": "links", "links": [
        {"title": "Rust 1.90.0 发布", "url": "https://github.com/rust-lang/rust/releases", "snippet": page}
    ]});
    let f = Fixture::new();
    let shown = text(&opened(&f, tool(ToolStatus::Ok, detail)));
    assert!(
        shown.iter().all(|l| !l.contains('#') && !l.contains("**")),
        "不露记号：{shown:?}"
    );
    assert!(
        shown.iter().any(|l| l.contains("Tag: 1.90.0")),
        "列表照列表画：{shown:?}"
    );
    assert!(shown.iter().any(|l| l.contains(tail)), "不截：{shown:?}");
}
