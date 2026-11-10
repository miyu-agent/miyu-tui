//! 量尺（`cargo test ruler_huge -- --ignored --nocapture --test-threads=1`）：一条几十 MB 的回答，排成行以后常驻多少（压测报告
//! O17，施工 YC-11）。只打数字，不断言。

use crate::transcript::{Entry, Kind};
use crate::ui::test_support::{Fixture, fresh_rows};

/// 这个进程现在常驻多少 MB（`/proc/self/statm` 第二格，按 4 KiB 一页）。
fn rss_mb() -> f64 {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: f64 = statm
        .split_whitespace()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(0.0);
    pages * 4096.0 / 1048576.0
}

/// 一行回答：中英混着、偶尔一个行内代码，照真模型的长日志样子。
fn reply(mb: usize) -> String {
    let line = "第一步先看 `upstream` 的日志：2026-11-10 10:00:00 ERROR connection reset by peer，重试三次都失败。\n";
    line.repeat(mb * 1048576 / line.len())
}

#[test]
#[ignore]
fn ruler_huge_reply() {
    for mb in [5usize, 20, 50] {
        let start = rss_mb();
        let text = reply(mb);
        let with_text = rss_mb();
        let entries = vec![Entry::new(1, Kind::Reply, text)];
        let f = Fixture::new();
        let t = std::time::Instant::now();
        let rows = fresh_rows(&entries, &f.ctx());
        let took = t.elapsed();
        let laid = rss_mb();
        let plain: usize = rows.iter().map(|r| r.plain().len()).sum();
        let spans: usize = rows.iter().map(|r| r.line.spans.len()).sum();
        let span_bytes: usize = rows
            .iter()
            .flat_map(|r| r.line.spans.iter())
            .map(|s| s.content.len())
            .sum();
        let owned = rows
            .iter()
            .flat_map(|r| r.line.spans.iter())
            .filter(|s| matches!(s.content, std::borrow::Cow::Owned(_)))
            .count();
        println!(
            "{mb} MB 回答：原文 +{:.0} MB；排成 {} 行 +{:.0} MB，{:.1}s；复制的字（现拼，不另存）{:.0} MB，span {spans} 个（自有 {owned}）字 {:.0} MB，Row {} 字节/个",
            with_text - start,
            rows.len(),
            laid - with_text,
            took.as_secs_f64(),
            plain as f64 / 1048576.0,
            span_bytes as f64 / 1048576.0,
            std::mem::size_of::<crate::ui::rows::Row>(),
        );
        drop(rows);
        let no_rows = rss_mb();
        drop(f);
        let no_md = rss_mb();
        drop(entries);
        println!(
            "  拿掉排好的行 -{:.0} MB，再拿掉 Markdown 缓存 -{:.0} MB，再拿掉原文 -{:.0} MB",
            laid - no_rows,
            no_rows - no_md,
            no_md - rss_mb()
        );
    }
}
