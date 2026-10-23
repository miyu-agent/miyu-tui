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
    rows(Some(data), &config, &money, width)
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
    assert!(
        got[0].ends_with("已用 45.2k / 128k · 压缩线 102.4k"),
        "{}",
        got[0]
    );
    assert_eq!(got[1], "");
    assert_eq!(got[2], "系统提示           6,299   13.9%");
    assert_eq!(got[3], "  技能目录           929    2.1%");
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
    assert!(
        got.last().is_some_and(|l| l.starts_with("  工具结果")),
        "{got:#?}"
    );
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
    assert_eq!(got[0], "已用 45.2k · 估算");
    assert_eq!(got[2], "系统提示           6,299   13.9%", "占已用");
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
    let at = wide.iter().position(|l| l == "本会话累计").unwrap();
    assert_eq!(
        &wide[at + 1..],
        [
            "          次数  输入  缓存读  缓存写  输出   花费",
            "主对话      12    9k    300k      1k    7k  $0.42",
            "压缩         1    2k     30k       0  1.5k  $0.42",
            "合计        13   11k    330k      1k  8.5k  $0.42",
        ]
    );
    // 窄的先去掉缓存写，再去掉缓存读。
    let narrow = text(&data, 44);
    let head = &narrow[narrow.iter().position(|l| l == "本会话累计").unwrap() + 1];
    assert!(
        !head.contains("缓存写") && head.contains("缓存读"),
        "{head}"
    );
    let narrower = text(&data, 36);
    let head = &narrower[narrower.iter().position(|l| l == "本会话累计").unwrap() + 1];
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
    assert_eq!(got[0], "已用 3.4k · 估算");
    assert!(got.iter().any(|l| l.starts_with("  核心说明")), "{got:#?}");
    assert!(!got.iter().any(|l| l.contains("本会话累计")));
    assert_eq!(text(&ContextData::default(), 80), ["正在读…"]);
}
