//! `/context` 框排出来的样子（蓝图「上下文 `/context`」）。

use serde_json::json;

use super::{ContextData, read_parts, read_uses, rows, title};
use crate::config::Config;
use crate::core::Bill;

/// 核心 CX-1 上的样本（核心会话 2026-10-23 发来的那一份）。
fn sample() -> serde_json::Value {
    json!({"window":128000,"line":102400,"used":45231,"basis":"measured","anchor":41000,"estimated_total":43800,
    "parts":[
      {"part":"system","label":"系统提示","tokens":6100,"children":[
        {"part":"system.skills","label":"技能目录","tokens":900},
        {"part":"system.other","label":"人设和其余说明","tokens":5200}]},
      {"part":"tools","label":"工具说明","tokens":9800,"children":[
        {"part":"tools.builtin","label":"内置工具","tokens":8000},
        {"part":"tools.mcp:notes","label":"notes","tokens":1500},
        {"part":"tools.deferred","label":"待加载的工具","tokens":300}]},
      {"part":"summary","label":"检查点摘要","tokens":2100},
      {"part":"messages","label":"对话","tokens":25800,"children":[
        {"part":"messages.user","label":"你说的","tokens":4200},
        {"part":"messages.memory","label":"记忆","tokens":1100,"children":[
          {"part":"messages.memory.resident","label":"常驻记忆","tokens":800},
          {"part":"messages.memory.recall","label":"召回的记忆","tokens":300}]},
        {"part":"messages.assistant","label":"AI 说的","tokens":14000},
        {"part":"messages.tool_results","label":"工具结果","tokens":6500}]}]})
}

fn text(data: &ContextData, width: u16) -> Vec<String> {
    let config = Config::builtin().unwrap();
    let money = |_: &Bill| Some("$0.42".to_string());
    rows(
        Some(data),
        &config,
        (&money, &super::Tail::default()),
        width,
    )
    .iter()
    .map(|l| l.to_string().trim_end().to_string())
    .collect()
}

#[test]
fn parts_are_scaled_to_what_is_used_and_children_follow() {
    let shape = read_parts(&sample());
    assert_eq!(shape.used(), 45231);
    let scaled = shape.apportioned();
    let sum: u64 = scaled.iter().map(|p| p.tokens).sum();
    // 各块之和 43800 摊成 45231，四舍五入差一两个。
    assert!(sum.abs_diff(45231) <= 2, "{sum}");
    assert_eq!(scaled[3].children[1].children[0].label, "常驻记忆");
    // 核心摊好了的（加起来就是 used）不再动。
    let mut even = shape.clone();
    even.used = Some(43800);
    assert_eq!(even.apportioned(), even.parts);
    // 没有 used 的照原值，用了多少是各块之和。
    even.used = None;
    assert_eq!(even.used(), 43800);
    assert_eq!(even.apportioned(), even.parts);
}

#[test]
fn the_composition_is_one_row_a_part_with_a_blank_line_between_the_big_parts() {
    let data = ContextData {
        parts: Some(Ok(read_parts(&sample()))),
        uses: None,
    };
    let got = text(&data, 80);
    assert_eq!(got[0], "上下文", "第 7 条：段头");
    assert!(
        got[1].ends_with("已用 45.2k / 128k · 压缩线 102.4k"),
        "{}",
        got[1]
    );
    assert_eq!(got[2], "");
    assert_eq!(got[3], "系统提示           6,299   13.9%");
    assert_eq!(got[4], "  技能目录           929    2.1%");
    assert!(
        got.iter().any(|l| l == "    常驻记忆         826    1.8%"),
        "{got:#?}"
    );
    // 大块和大块之间空一行；不写空余（2026-10-23 项目主人：用量写三处不合理），标题只写「上下文明细」。
    let tools = got
        .iter()
        .position(|l| l.starts_with("工具说明"))
        .unwrap_or(0);
    assert_eq!(got[tools - 1], "", "{got:#?}");
    assert!(!got.iter().any(|l| l.starts_with("空余")), "{got:#?}");
    // 各块最后是工具结果，空一行接已压缩、缓存断裂（第 7 条；没开会话的这里是 0）。
    let done = got
        .iter()
        .position(|l| l.starts_with("已压缩"))
        .expect("有已压缩");
    assert!(got[done - 2].starts_with("  工具结果"), "{got:#?}");
    assert!(got[done].ends_with("0 次"), "{got:#?}");
    assert!(got[done + 1].starts_with("缓存断裂") && got[done + 1].ends_with("0 次"));
    assert_eq!(got.len(), done + 2, "没卡住不写原因：{got:#?}");
    let config = Config::builtin().unwrap();
    assert_eq!(title(&config.text.context_box, Some(&data)), "上下文明细");
}

#[test]
fn estimated_and_windowless_replies_say_so() {
    let mut reply = sample();
    reply["basis"] = json!("estimated");
    reply.as_object_mut().unwrap().remove("window");
    reply.as_object_mut().unwrap().remove("line");
    let data = ContextData {
        parts: Some(Ok(read_parts(&reply))),
        uses: None,
    };
    let got = text(&data, 80);
    assert_eq!(got[1], "已用 45.2k · 估算");
    assert_eq!(got[3], "系统提示           6,299   13.9%", "占已用");
    assert!(!got[1].contains("压缩线"), "没压缩线的不写");
}

#[test]
fn usage_by_purpose_is_a_table_sorted_by_tokens_with_a_total() {
    let uses = read_uses(&json!({"rows": [
        {"use": "compaction", "requests": 1,
         "usage": {"uncached": 2000, "cache_read": 30000, "cache_write": 0, "output": 1500}, "amounts": []},
        {"use": "main", "requests": 12,
         "usage": {"uncached": 9000, "cache_read": 300000, "cache_write": 1000, "output": 7000},
         "amounts": [{"currency": "USD", "amount": 0.4}]},
        {"use": "title", "requests": 0, "usage": {}}
    ]}));
    let data = ContextData {
        parts: None,
        uses: Some(Ok(uses)),
    };
    let wide = text(&data, 90);
    let at = wide.iter().position(|l| l == "按用途").unwrap();
    let end = wide[at..]
        .iter()
        .position(String::is_empty)
        .map_or(wide.len(), |n| at + n);
    assert_eq!(
        &wide[at + 1..end],
        [
            "          次数  输入  缓存读  缓存写  输出   花费",
            "主对话      12    9k    300k      1k    7k  $0.42",
            "压缩         1    2k     30k       0  1.5k  $0.42",
            "合计        13   11k    330k      1k  8.5k  $0.42",
        ]
    );
    // 窄的先去掉缓存写，再去掉缓存读。
    let narrow = text(&data, 44);
    let head = &narrow[narrow.iter().position(|l| l == "按用途").unwrap() + 1];
    assert!(
        !head.contains("缓存写") && head.contains("缓存读"),
        "{head}"
    );
    let narrower = text(&data, 36);
    let head = &narrower[narrower.iter().position(|l| l == "按用途").unwrap() + 1];
    assert!(!head.contains("缓存"), "{head}");
}

#[test]
fn a_new_session_preview_has_no_totals() {
    // 核心 CX-1 下的新会话预览（`session.context {}`）：全是估的，没有窗口，没有累计。
    let data = ContextData {
        parts: Some(Ok(read_parts(
            &json!({"basis":"estimated","estimated_total":3436,"used":3434,
            "parts":[{"part":"system","label":"系统提示","tokens":410,"children":[
                {"part":"system.core","label":"核心说明","tokens":137},
                {"part":"system.clicall","label":"脚本调用说明","tokens":103},
                {"part":"system.skills","label":"技能目录","tokens":170}]},
              {"part":"tools","label":"工具说明","tokens":3024,"children":[
                {"part":"tools.builtin","label":"内置工具","tokens":3024}]}]}),
        ))),
        uses: None,
    };
    let got = text(&data, 80);
    assert_eq!(got[1], "已用 3.4k · 估算");
    assert!(got.iter().any(|l| l.starts_with("  核心说明")), "{got:#?}");
    assert!(
        !got.iter()
            .any(|l| l.contains("按用途") || l.contains("累计消耗"))
    );
    assert_eq!(text(&ContextData::default(), 80), ["正在读…"]);
}

/// 第 7 条：累计消耗的段头写命中率，四种 token 一种一行；花费那一段照 `converted_parts` 加起来按种类分。
#[test]
fn spent_and_cost_sections_split_by_kind() {
    let uses = read_uses(&json!({"rows": [
        {"use": "main", "requests": 2,
         "usage": {"uncached": 1000, "cache_read": 8000, "cache_write": 1000, "output": 0},
         "amounts": [{"currency": "USD", "amount": 0.3}],
         "converted": {"currency": "USD", "amount": 0.3},
         "converted_parts": {"uncached": 0.1, "cache_read": 0.1, "cache_write": 0.1, "output": 0}},
        {"use": "subagent", "requests": 1,
         "usage": {"uncached": 0, "cache_read": 0, "cache_write": 0, "output": 10000},
         "amounts": [{"currency": "USD", "amount": 0.1}],
         "converted": {"currency": "USD", "amount": 0.1},
         "converted_parts": {"uncached": 0, "cache_read": 0, "cache_write": 0, "output": 0.1}}
    ]}));
    let data = ContextData {
        parts: None,
        uses: Some(Ok(uses)),
    };
    let config = Config::builtin().unwrap();
    let money = |bill: &Bill| {
        let c = bill.converted.as_ref()?;
        Some(format!("${:.2}", c.total.amount))
    };
    let got: Vec<String> = rows(Some(&data), &config, (&money, &super::Tail::default()), 60)
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    let at = |word: &str| {
        got.iter()
            .position(|l| l.starts_with(word))
            .unwrap_or_else(|| panic!("没有「{word}」：{got:#?}"))
    };
    let spent = at("累计消耗 20k");
    assert!(got[spent].ends_with("缓存命中 80%"), "{got:#?}");
    // 缓存读、缓存写、输出、输入（未命中），占累计（2 万）的百分比。
    assert!(
        got[spent + 1].starts_with("缓存读") && got[spent + 1].ends_with("8k     40%"),
        "{got:#?}"
    );
    assert!(got[spent + 3].starts_with("输出") && got[spent + 3].ends_with("10k     50%"));
    assert!(got[spent + 4].starts_with("输入（未命中）"));
    assert_eq!(got[spent + 5], "按用途");
    let cost = at("花费");
    assert!(got[cost].ends_with("$0.40"), "{got:#?}");
    assert!(got[cost + 3].starts_with("输出") && got[cost + 3].ends_with("$0.10     25%"));
    assert_eq!(got.len(), cost + 5, "{got:#?}");
    // 发版前 check 第 19 条（2026-11-09 项目主人）：窗口宽时段头右边的缓存命中、总钱数和下面表里的数在同一列收尾，不贴着框的
    // 右边缘。
    let wide: Vec<String> = rows(Some(&data), &config, (&money, &super::Tail::default()), 120)
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    let w = |s: &String| unicode_width::UnicodeWidthStr::width(s.as_str());
    assert_eq!(w(&wide[spent]), w(&wide[spent + 1]), "{wide:#?}");
    assert_eq!(w(&wide[cost]), w(&wide[cost + 1]), "{wide:#?}");
    assert!(w(&wide[spent]) < 60, "{wide:#?}");
    // 有一行是老账（没有 converted_parts）：按种类的不写，总数照写。
    let mut old = read_uses(&json!({"rows": [
        {"use": "main", "requests": 1, "usage": {"uncached": 10, "output": 1},
         "amounts": [{"currency": "USD", "amount": 0.2}],
         "converted": {"currency": "USD", "amount": 0.2}}]}));
    old[0].requests = 1;
    let data = ContextData {
        parts: None,
        uses: Some(Ok(old)),
    };
    let got: Vec<String> = rows(Some(&data), &config, (&money, &super::Tail::default()), 60)
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    assert!(got.last().unwrap().starts_with("花费") && got.last().unwrap().ends_with("$0.20"));
}

/// 第 7 条：卡住的，上下文那一段底下再一行红字说为什么。
#[test]
fn a_stuck_session_says_why_under_the_context_section() {
    use crate::transcript::Stuck;
    let data = ContextData {
        parts: Some(Ok(read_parts(&sample()))),
        uses: None,
    };
    let config = Config::builtin().unwrap();
    let money = |_: &Bill| None;
    let tail = super::Tail {
        compactions: 2,
        breaks: 1,
        stuck: Some(Stuck::Over {
            used: 203_000,
            window: 128_000,
        }),
    };
    let laid = rows(Some(&data), &config, (&money, &tail), 80);
    let last = laid.last().unwrap();
    assert_eq!(last.to_string(), "超出这个模型的窗口，压缩后才能继续。");
    assert_eq!(
        last.style.fg.or(last.spans[0].style.fg),
        crate::theme::error().fg
    );
    let text: Vec<String> = laid
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    assert!(text[text.len() - 3].ends_with("2 次") && text[text.len() - 2].ends_with("1 次"));
}
