//! 出错那一句的原因（蓝图 `tui.md`「正文」第 4 条）：供应商的原话照样写，不加分类（各家的错误码不一样，原话大多
//! 已经说清了为什么）；几种 HTTP 状态码前面加一句人话；内核自己查出来的写分类的人话。收尾那一行、压缩失败那一行
//! 都照它。

use std::collections::HashMap;

use serde::Deserialize;

use crate::config::Texts;
use crate::core::CallError;

/// 知道还要等多久的出错（核心 3-5 四补的 `retry_after_ms`，2026-10-22 项目主人交给核心的：429 那句带上还要等多久）：
/// 状态码那句换成带 `{wait}` 的，模型都在冷却的也接一句；不写绝对的时刻。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryAfter {
    /// 状态码 → 带 `{wait}` 的人话；没写的状态码照 `status_hints`。
    pub hints: HashMap<String, String>,
    /// 模型都在冷却，`{wait}`。
    pub cooling: String,
    /// 几秒、几分钟、几小时、几天：一个、几个两种写法（英文分单复数），`{n}`。
    pub seconds: [String; 2],
    pub minutes: [String; 2],
    pub hours: [String; 2],
    pub days: [String; 2],
}

/// 还要等多久写成字：不到 1 分钟写秒，不到 1 小时写分钟（向上取整），不到 1 天写小时（四舍五入，至少 1），再长写天
/// （四舍五入，至少 1）。
pub(super) fn wait(ms: u64, words: &RetryAfter) -> String {
    const MINUTE: u64 = 60_000;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let (n, forms) = if ms < MINUTE {
        (ms.div_ceil(1000).max(1), &words.seconds)
    } else if ms < HOUR {
        (ms.div_ceil(MINUTE), &words.minutes)
    } else if ms < DAY {
        ((ms + HOUR / 2) / HOUR, &words.hours)
    } else {
        ((ms + DAY / 2) / DAY, &words.days)
    };
    let n = n.max(1);
    let form = if n == 1 { &forms[0] } else { &forms[1] };
    form.replace("{n}", &n.to_string())
}

/// 内核自己查出来的分类：原话不是供应商的，只写分类的人话（`03-事件模型.md`「出错」）。
const KERNEL_CLASSES: [&str; 7] = [
    // 一轮里连着几次写到输出上限（核心 3-10）。
    "output_limit",
    "no_model",
    "cooling",
    "bad_stream",
    "empty_reply",
    "bad_summary",
    "compaction_paused",
];

/// 限速的分类：核心还没给状态码的，照它当 429（核心把 429 都分成它）。
const RATE_LIMITED: (&str, u16) = ("rate_limited", 429);

/// 出错的原因。
pub(super) fn reason(error: &CallError, texts: &Texts) -> String {
    let said = error.message.trim();
    let class = error.class.as_str();
    let waiting = error.retry_after_ms.map(|ms| wait(ms, &texts.retry_after));
    // 内核自己查出来的：原话是给运行日志的英文诊断，只写人话（2026-10-01 项目主人：有中文了后面不要再接英文）。
    if KERNEL_CLASSES.contains(&class) {
        return match waiting {
            Some(wait) if class == "cooling" => texts.retry_after.cooling.replace("{wait}", &wait),
            _ => class_name(class, texts),
        };
    }
    let status = error
        .status
        .or((class == RATE_LIMITED.0).then_some(RATE_LIMITED.1));
    let timed = status.zip(waiting).and_then(|(s, wait)| {
        Some(
            texts
                .retry_after
                .hints
                .get(&s.to_string())?
                .replace("{wait}", &wait),
        )
    });
    if let Some(hint) = timed {
        return with(&hint, said, texts);
    }
    match status.and_then(|s| texts.status_hints.get(&s.to_string())) {
        Some(hint) => with(hint, said, texts),
        None if said.is_empty() => class_name(class, texts),
        None => said.to_string(),
    }
}

/// 人话后面接原话；没有原话的只写人话。
fn with(head: &str, said: &str, texts: &Texts) -> String {
    match said {
        "" => head.to_string(),
        said => texts
            .reason_with
            .replace("{head}", head)
            .replace("{message}", said),
    }
}

/// 出错的分类写成人话（`text/zh.json` 的 `error_classes`）；认不得的照原样。
pub(super) fn class_name(class: &str, texts: &Texts) -> String {
    texts
        .error_classes
        .get(class)
        .cloned()
        .unwrap_or_else(|| class.to_string())
}

#[cfg(test)]
mod tests;
