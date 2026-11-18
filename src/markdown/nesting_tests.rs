//! 套得很深的 `<details>`、列表、引用（LiteMiyu 照这份代码查到的：`<details>` 套 4000 层调试构建 8.8 秒 2.3 GB，
//! 列表套 2000 层 6.7 秒 0.9 GB，照层数的平方涨）。量尺看耗时和内存，回归测试数引子的段数、不断言耗时。

use super::render;
use super::tests::draw;
use crate::config::Config;

/// `<details open>` 套 `n` 层，每层一个标题，最里面一句，收尾以后再一句。
fn details(n: usize) -> String {
    let open: String = (0..n)
        .map(|i| format!("<details open>\n<summary>第{i}层</summary>\n\n"))
        .collect();
    format!("{open}最里面的字\n\n{}后面的字", "</details>\n\n".repeat(n))
}

/// 无序列表套 `n` 层，一层一行（第 `i` 层缩进 `2i` 格）。
fn list(n: usize) -> String {
    let items: String = (0..n)
        .map(|i| format!("{}- 第{i}层\n", "  ".repeat(i)))
        .collect();
    format!("{items}\n后面的字")
}

/// 无序列表套 `n` 层，写在一行里（`- - - … 字`）。
fn list_one_line(n: usize) -> String {
    format!("{}最里面的字\n\n后面的字", "- ".repeat(n))
}

/// 引用套 `n` 层，一层一行。
fn quote(n: usize) -> String {
    let lines: String = (0..n)
        .map(|i| format!("{} 第{i}层\n", ">".repeat(i + 1)))
        .collect();
    format!("{lines}\n后面的字")
}

/// 排出来所有行的引子一共几段。
fn lead_spans(text: &str) -> (usize, usize) {
    let drawn = draw(text, 80);
    let spans = drawn.iter().map(|l| l.lead.len()).sum();
    (drawn.len(), spans)
}

/// 所有行的字（不带引子）连起来，去掉空白。
fn plain(text: &str) -> String {
    draw(text, 80)
        .iter()
        .map(|l| super::inline::plain(&l.folded))
        .collect::<String>()
        .split_whitespace()
        .collect()
}

#[test]
fn nesting_deeper_than_the_cap_does_not_grow_the_lead_and_keeps_every_word() {
    // 改以前每一行的引子照层数一层一段，一千层的每行上千段（排出来的段数照层数的平方涨）。
    for (name, text) in [
        ("details", details(1000)),
        ("list", list(1000)),
        ("list_one_line", list_one_line(1000)),
        ("quote", quote(1000)),
    ] {
        let (lines, spans) = lead_spans(&text);
        assert!(
            spans <= lines * (super::MAX_NESTING + 1),
            "{name}：{lines} 行、引子 {spans} 段"
        );
        let all = plain(&text);
        assert!(all.ends_with("后面的字"), "{name}：后面的字还在");
        if name != "list_one_line" {
            for i in [0, 15, 16, 17, 500, 999] {
                assert!(
                    all.contains(&format!("第{i}层")),
                    "{name}：第 {i} 层的字还在"
                );
            }
        }
        if name == "details" || name == "list_one_line" {
            assert!(all.contains("最里面的字"), "{name}：最里面的字还在");
        }
    }
}

#[test]
fn nesting_within_the_cap_draws_as_before() {
    let drawn = super::tests::lines("- a\n  - b\n    - c", 30);
    assert_eq!(drawn, ["▪ a", "  • b", "    ◦ c"]);
    let deep = list(super::MAX_NESTING);
    let drawn = draw(&deep, 200);
    let last = drawn
        .iter()
        .find(|l| super::inline::plain(&l.folded) == format!("第{}层", super::MAX_NESTING - 1))
        .unwrap();
    assert_eq!(last.lead.len(), super::MAX_NESTING, "上限以内一层一段");
}

/// 量尺：套 1000～4000 层的 `<details>`、500～2000 层的列表，排一次要多久、峰值内存多少
/// （`MD_NEST=details:4000 cargo test --bin miyu-tui nesting_ruler -- --ignored --nocapture`，一个进程只量一种，
/// 峰值内存读 cgroup 的 `memory.peak`，在 `systemd-run --user --scope -p MemoryMax=8G` 里跑；没设 `MD_NEST` 的
/// 从小到大量一遍，峰值照最大的那一个算）。
#[test]
#[ignore = "量尺：看耗时和内存用"]
fn nesting_ruler() {
    let cases: Vec<(String, usize)> = match std::env::var("MD_NEST") {
        Ok(one) => {
            let (kind, n) = one.split_once(':').expect("种类:层数");
            vec![(kind.to_string(), n.parse().expect("层数"))]
        }
        Err(_) => [
            ("details", [1000, 2000, 4000]),
            ("list", [500, 1000, 2000]),
            ("list_one_line", [500, 1000, 2000]),
            ("quote", [500, 1000, 2000]),
        ]
        .iter()
        .flat_map(|(k, ns)| ns.iter().map(|n| (k.to_string(), *n)))
        .collect(),
    };
    let config = Config::builtin().unwrap();
    let kit = super::Kit {
        languages: &config.languages,
        math: &config.math,
        labels: &config.text.markdown,
        tab: config.layout.output.tab,
    };
    for (kind, n) in cases {
        let text = match kind.as_str() {
            "details" => details(n),
            "list" => list(n),
            "list_one_line" => list_one_line(n),
            "quote" => quote(n),
            other => panic!("认不得 {other}"),
        };
        let start = std::time::Instant::now();
        let drawn = render(&text, 80, &kit, &[]);
        let took = start.elapsed();
        let spans: usize = drawn.iter().map(|l| l.lead.len()).sum();
        println!(
            "{kind} {n} 层：原文 {} KB，{} 行，引子 {spans} 段，{took:?}，峰值 {}（cgroup {}）",
            text.len() / 1024,
            drawn.len(),
            status("VmHWM:"),
            cgroup_peak(),
        );
        drop(drawn);
    }
}

/// `/proc/self/status` 里的一项。
fn status(field: &str) -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(field))
                .map(|l| l[field.len()..].trim().to_string())
        })
        .unwrap_or_else(|| "?".into())
}

/// 这个进程所在 cgroup 的 `memory.peak`，换成 MiB。
fn cgroup_peak() -> String {
    let path = std::fs::read_to_string("/proc/self/cgroup")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("0::").map(str::to_string))
        });
    path.and_then(|p| std::fs::read_to_string(format!("/sys/fs/cgroup{p}/memory.peak")).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or_else(|| "?".into(), |b| format!("{} MiB", b / 1024 / 1024))
}
