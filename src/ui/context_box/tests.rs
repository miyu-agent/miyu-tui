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

/// 写着的宽。
fn width(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// 发版前 check 32（2026-11-09 项目主人从三个样板里挑的「每行一根小条」）：段头右边写用了多少，一块一行一根小条，大块之间
/// 不空行；段头右边那一格和下面的百分比在同一列收尾（第 19 条）。
#[test]
fn the_composition_is_one_bar_a_part_under_a_head_that_says_how_much_is_used() {
    let data = ContextData {
        parts: Some(Ok(read_parts(&sample()))),
        uses: None,
    };
    let got = text(&data, 80);
    assert!(got[0].starts_with("上下文 "), "第 7 条：段头");
    assert!(
        got[0].ends_with("已用 45.2k / 128k · 压缩线 102.4k"),
        "{}",
        got[0]
    );
    assert_eq!(got[1], "系统提示          ██▎               6.3k  13.9%");
    assert_eq!(got[2], "  技能目录        ▍                  929   2.1%");
    assert!(
        got.iter()
            .any(|l| l == "    常驻记忆      ▎                  826   1.8%"),
        "{got:#?}"
    );
    assert_eq!(width(&got[0]), width(&got[1]), "{got:#?}");
    // 大块之间不再空行；不写空余，标题只写「上下文明细」。
    assert!(!got.iter().any(String::is_empty), "{got:#?}");
    assert!(!got.iter().any(|l| l.starts_with("空余")), "{got:#?}");
    // 各块最后是工具结果，接着一行「已压缩 N 次 · 缓存断裂 N 次」（没开会话的这里是 0）。
    let done = got.len() - 1;
    assert!(got[done - 1].starts_with("  工具结果"), "{got:#?}");
    assert_eq!(got[done], "已压缩 0 次 · 缓存断裂 0 次", "没卡住不写原因");
    let config = Config::builtin().unwrap();
    assert_eq!(title(&config.text.context_box, Some(&data)), "上下文明细");
}

#[test]
fn a_bar_is_drawn_in_eighths_and_a_little_still_shows() {
    let bar = super::bars::bar;
    assert_eq!(
        bar(500, 1000, 16),
        format!("{}{}", "█".repeat(8), " ".repeat(8))
    );
    assert_eq!(bar(3, 8, 1), "▍");
    assert_eq!(bar(1000, 1000, 4), "████");
    assert_eq!(bar(1, 1_000_000, 4), "▏   ", "不是 0 的至少八分之一格");
    assert_eq!(bar(0, 1000, 4), "    ");
    assert_eq!(bar(5, 0, 2), "▏ ", "整个是 0 的不除");
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
    assert!(got[0].ends_with("  已用 45.2k · 估算"), "{}", got[0]);
    assert!(got[1].ends_with("6.3k  13.9%"), "占已用：{}", got[1]);
    assert!(!got[0].contains("压缩线"), "没压缩线的不写");
}

/// 按用途：次数、token（四种加起来）、花费三列，照 token 多少排，不止一行的最后合计；窄得放不下时去掉花费（发版前 check 32）。
#[test]
fn usage_by_purpose_is_a_three_column_table_sorted_by_tokens() {
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
        uses: Some(Ok(uses.clone())),
    };
    let wide = text(&data, 90);
    let at = wide
        .iter()
        .position(|l| l.starts_with("按用途"))
        .unwrap_or_else(|| panic!("{wide:#?}"));
    assert_eq!(
        &wide[at..],
        [
            "按用途    次数   token   花费",
            "主对话      12    317k  $0.42",
            "压缩         1   33.5k  $0.42",
            "合计        13  350.5k  $0.42",
        ]
    );
    let narrow = text(&data, 30);
    let at = narrow.iter().position(|l| l.starts_with("按用途")).unwrap();
    assert_eq!(narrow[at], "按用途    次数   token", "放不下去掉花费");
    // 只有一行的不写合计。
    let data = ContextData {
        parts: None,
        uses: Some(Ok(uses[1..].to_vec())),
    };
    let one = text(&data, 90);
    assert!(!one.iter().any(|l| l.starts_with("合计")), "{one:#?}");
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
    assert!(got[0].ends_with("已用 3.4k · 估算"), "{}", got[0]);
    assert!(got.iter().any(|l| l.starts_with("  核心说明")), "{got:#?}");
    assert!(
        !got.iter()
            .any(|l| l.contains("按用途") || l.contains("累计消耗"))
    );
    assert_eq!(text(&ContextData::default(), 80), ["正在读…"]);
}

/// 第 7 条、发版前 check 32：累计消耗的段头写命中率，四种 token 一种一根条、多的在上；单独的「花费」那一段不再写。
#[test]
fn spent_kinds_are_bars_biggest_first_and_there_is_no_cost_section() {
    let uses = read_uses(&json!({"rows": [
        {"use": "main", "requests": 2,
         "usage": {"uncached": 1000, "cache_read": 8000, "cache_write": 1000, "output": 0},
         "amounts": [{"currency": "USD", "amount": 0.3}]},
        {"use": "subagent", "requests": 1,
         "usage": {"uncached": 0, "cache_read": 0, "cache_write": 0, "output": 10000},
         "amounts": [{"currency": "USD", "amount": 0.1}]}
    ]}));
    let data = ContextData {
        parts: None,
        uses: Some(Ok(uses)),
    };
    let got = text(&data, 60);
    let spent = got
        .iter()
        .position(|l| l.starts_with("累计消耗 20k"))
        .unwrap_or_else(|| panic!("{got:#?}"));
    assert!(got[spent].ends_with("缓存命中 80%"), "{got:#?}");
    assert_eq!(
        width(&got[spent]),
        width(&got[spent + 1]),
        "命中率和百分比一列"
    );
    let names: Vec<&str> = got[spent + 1..spent + 5]
        .iter()
        .map(|l| l.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(names, ["输出", "缓存读", "缓存写", "输入"], "多的在上");
    assert_eq!(
        got[spent + 1],
        format!("输出    {}         10k    50%", "█".repeat(8))
    );
    assert!(!got.iter().any(|l| l.starts_with("花费")), "{got:#?}");
    assert!(got.last().unwrap().starts_with("合计"), "{got:#?}");
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
    let before = laid[laid.len() - 2].to_string();
    assert_eq!(before, "已压缩 2 次 · 缓存断裂 1 次");
}

/// 设了会话的上下文上限（核心 CTX-1，设计 68）：段头右边压缩线后面接「会话上限 128k」。
#[test]
fn a_session_cap_is_named_in_the_head() {
    let mut reply = sample();
    reply["context"] = json!(128000);
    let data = ContextData {
        parts: Some(Ok(read_parts(&reply))),
        uses: None,
    };
    let got = text(&data, 100);
    assert!(
        got[0].ends_with("已用 45.2k / 128k · 压缩线 102.4k · 会话上限 128k"),
        "{}",
        got[0]
    );
}
