//! 出错那一句带上还要等多久（核心 3-5 四补的 `retry_after_ms`）。

use super::{RetryAfter, reason, retry_line, wait};
use crate::config::Config;
use crate::core::CallError;

fn error(class: &str, message: &str, status: Option<u16>, retry: Option<u64>) -> CallError {
    CallError {
        class: class.into(),
        message: message.into(),
        status,
        retry_after_ms: retry,
    }
}

#[test]
fn how_long_reads_in_seconds_minutes_hours_or_days() {
    let texts = Config::builtin().unwrap().text;
    let words = &texts.retry_after;
    assert_eq!(wait(1_500, words), "2 秒", "不到 1 分钟写秒，向上取整");
    assert_eq!(wait(59_000, words), "59 秒");
    assert_eq!(wait(61_000, words), "2 分钟", "分钟向上取整");
    assert_eq!(wait(598_000, words), "10 分钟");
    assert_eq!(wait(3_600_000, words), "1 小时");
    assert_eq!(wait(47_137_000, words), "13 小时", "小时四舍五入");
    assert_eq!(wait(90_000_000, words), "1 天");
    let english = RetryAfter {
        seconds: ["{n} second".into(), "{n} seconds".into()],
        hours: ["{n} hour".into(), "{n} hours".into()],
        ..words.clone()
    };
    assert_eq!(wait(1_000, &english), "1 second");
    assert_eq!(wait(3_600_000, &english), "1 hour", "一个用单数");
    assert_eq!(wait(47_137_000, &english), "13 hours");
}

#[test]
fn a_known_wait_goes_into_the_sentence_and_without_it_nothing_changes() {
    // 2026-10-22 项目主人交给核心的：Zen 免费档 429，retry-after 约 13 小时。
    let texts = Config::builtin().unwrap().text;
    let zen = "HTTP 429: Free usage limit reached.";
    assert_eq!(
        reason(&error("auth", zen, Some(429), Some(47_137_000)), &texts),
        format!("请求受限或额度不足，约 13 小时后恢复：{zen}")
    );
    assert_eq!(
        reason(&error("auth", zen, Some(429), None), &texts),
        format!("请求受限或额度不足，请稍后重试：{zen}"),
        "没有这一格的照原来那句"
    );
    let cooling = "all candidates cooling: a1/m rate_limited until 2026-10-22T09:41:00Z";
    assert_eq!(
        reason(&error("cooling", cooling, None, Some(598_000)), &texts),
        "模型都在冷却，约 10 分钟后恢复",
        "内核自己的分类照旧不接英文原话"
    );
    assert_eq!(
        reason(&error("cooling", cooling, None, None), &texts),
        "模型都在冷却"
    );
}

#[test]
fn running_out_of_quota_has_its_own_words() {
    // 核心 Q-4中：额度、余额用完单独一类（以前归在 auth）。
    let texts = Config::builtin().unwrap().text;
    assert_eq!(reason(&error("quota", "", None, None), &texts), "额度用完");
    assert_eq!(
        reason(
            &error("quota", "insufficient_quota", Some(402), None),
            &texts
        ),
        "额度已用完：insufficient_quota",
        "有状态码、原话的照旧接原话"
    );
}

/// 模型连上了却一直不出字（核心 YC-3 的 `stalled`）：收尾行、重试那一行都写「模型没有响应」，照规矩接原话（空闲超时多久）；
/// 没有原话的只写人话。别的分类的重试照旧只写原话。
#[test]
fn a_stalled_model_is_named_and_keeps_the_idle_timeout_words() {
    let texts = Config::builtin().unwrap().text;
    let said = "空闲超时：180 秒没有收到新的内容";
    assert_eq!(
        reason(&error("stalled", said, None, None), &texts),
        format!("模型没有响应：{said}")
    );
    assert_eq!(
        reason(&error("stalled", "", None, None), &texts),
        "模型没有响应"
    );
    assert_eq!(
        retry_line(1, 5, "stalled", said, &texts),
        format!("重试 1/5：模型没有响应：{said}")
    );
    assert_eq!(
        retry_line(2, 5, "retryable", "connection reset", &texts),
        "重试 2/5：connection reset"
    );
}
