//! 协议一行的上限（核心 `protocol.md`：一条消息一行，最多 1 MiB）：超了核心整行不认，回的错对不上是哪一条请求，界面当没发生，
//! 字就悄悄丢了（压测报告 F5，施工 YC-11）。说一句话之前照它量，放不下的不发（蓝图 `tui.md`「输入框」第 11 条）。

/// 一行最多多少字节（含换行）。
pub const LINE_LIMIT: usize = 1024 * 1024;

/// 话以外那几格（`jsonrpc`、`id`、`method`、会话编号、`cwd`、点名的技能）留的余量。
const ROOM: usize = 8 * 1024;

/// 这句话写进请求（JSON 字符串，转义以后）放不放得进一行。
#[must_use]
pub fn fits(text: &str) -> bool {
    escaped_len(text) + ROOM <= LINE_LIMIT
}

/// 照 JSON 写成字符串（不算两头的引号）有多少字节：`"`、`\`、常见的控制字符多一个，别的控制字符写成 `\u00XX` 多五个；
/// 别的照 UTF-8 原样（同 `serde_json` 的写法）。不另拷一份：一两 MB 的字只数一遍。
fn escaped_len(text: &str) -> usize {
    text.bytes()
        .map(|b| match b {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 0x08 | 0x0c => 2,
            0..0x20 => 6,
            _ => 1,
        })
        .sum()
}

/// 字有多大，给人看的写法（`1.4 MB`）。
#[must_use]
pub fn size(text: &str) -> String {
    let bytes = escaped_len(text) as f64;
    format!("{:.1} MB", bytes / LINE_LIMIT as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 数出来的和 `serde_json` 真写出来的一样长（去掉两头的引号）。
    #[test]
    fn the_count_matches_what_serde_json_writes() {
        for text in [
            "",
            "你好",
            "a\"b\\c\nd\re\tf",
            "\u{1}\u{8}\u{c}\u{1f}",
            "😀 \u{7f} \u{2028}",
        ] {
            let written = serde_json::to_string(text).unwrap();
            assert_eq!(escaped_len(text), written.len() - 2, "{text:?}");
        }
    }

    /// 1 MiB 上下：一两 MB 的放不下；换行多的照转义以后算（50 万个换行写出来 1 MB）。
    #[test]
    fn a_megabyte_or_two_does_not_fit_and_escapes_count() {
        assert!(fits(&"a".repeat(100_000)));
        assert!(!fits(&"a".repeat(2 * LINE_LIMIT)));
        assert!(!fits(&"a".repeat(LINE_LIMIT)));
        assert!(fits(&"a".repeat(LINE_LIMIT - ROOM)));
        assert!(!fits(&"\n".repeat(LINE_LIMIT / 2)));
        assert_eq!(size(&"a".repeat(LINE_LIMIT * 3 / 2)), "1.5 MB");
    }
}
