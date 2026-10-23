//! 用量那一行要的小工具：token 数写短、算百分比。

/// 把 token 数写短：`950`、`12.3k`、`1.4M`、`1B`。整数不带 `.0`。
pub fn short(n: u64) -> String {
    let (value, unit) = match n {
        0..1_000 => return n.to_string(),
        1_000..1_000_000 => (n as f64 / 1e3, "k"),
        1_000_000..1_000_000_000 => (n as f64 / 1e6, "M"),
        _ => (n as f64 / 1e9, "B"),
    };
    let text = format!("{value:.1}");
    format!("{}{unit}", text.strip_suffix(".0").unwrap_or(&text))
}

/// 一段字大约多少 token。思考中那一行用：供应商一轮结束才报用量，这时只能估。不带分词表，照字分几类数：汉字 0.7 个，
/// 数字、别的非 ASCII 字各 1 个，英文词每 7 个字母 1 个，ASCII 符号半个，空白不算。
///
/// 2026-10-24 拿 DeepSeek 8 段思考（中文、日文、英文、代码、数学、逻辑）的实报量定的：8 段都在实报的 ±13% 以内，
/// 每段拿另外 7 段定的数去估，最差差 28%。核心那种 4 个字节一个（`miyu-kernel` 的 `estimate.rs`）最差差 47%：
/// 数学推理里数字、符号多，只估到一半。
pub fn estimate_tokens(text: &str) -> u64 {
    // 以十分之一个 token 为单位累加。
    let word = |letters: u64| letters.div_ceil(7) * 10;
    let (mut tenths, mut letters) = (0u64, 0u64);
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            letters += 1;
            continue;
        }
        tenths += word(letters);
        letters = 0;
        tenths += match c {
            _ if c.is_ascii_whitespace() => 0,
            _ if c.is_ascii_digit() => 10,
            _ if c.is_ascii() => 5,
            '\u{2e80}'..='\u{9fff}' | '\u{f900}'..='\u{ffef}' => 7,
            _ => 10,
        };
    }
    (tenths + word(letters)).div_ceil(10)
}

/// 三位一撇：`3120` 写成 `3,120`（压缩中收到的字数，照 `miyu ask`）。
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 百分比，保留一位小数，整数不带 `.0`：`0.2`、`4.1`、`12`。分母是 0 的是 `0`。
pub fn percent_tenths(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "0".to_string();
    }
    let text = format!("{:.1}", part as f64 * 100.0 / whole as f64);
    text.strip_suffix(".0").unwrap_or(&text).to_string()
}

/// 读秒：`12s`、`1m 05s`、`1h 02m 05s`（运行状态行的用时、收起那一行末尾的用时，照旧版的底栏）。
pub fn clock(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    match (h, m) {
        (0, 0) => format!("{s}s"),
        (0, _) => format!("{m}m {s:02}s"),
        _ => format!("{h}h {m:02}m {s:02}s"),
    }
}

/// 命中率（蓝图 `tui.md`「命中率的写法」）：99 以下写整数；99 以上带一位小数，正好 99.0 的写 `99`；
/// 四舍五入到 100.0 的写 `100`。分母是 0 的是 `0`。
pub fn hit_rate(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "0".to_string();
    }
    let exact = part as f64 * 100.0 / whole as f64;
    let tenths = (exact * 10.0).round() / 10.0;
    if tenths < 99.0 {
        return format!("{}", exact.round() as u64);
    }
    let text = format!("{tenths:.1}");
    text.strip_suffix(".0").unwrap_or(&text).to_string()
}

/// 一步、一轮用了多久：一分钟以内带一位小数 `11.8s`，再长 `1m 05s`、`1h 02m 05s`（照旧版的时间线，
/// 和运行状态行的读秒一个写法）。
pub fn seconds(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        return format!("{:.1}s", d.as_secs_f64());
    }
    clock(secs)
}

#[cfg(test)]
mod tests {
    use super::{percent_tenths, seconds, short};

    #[test]
    fn short_numbers() {
        assert_eq!(short(950), "950");
        assert_eq!(short(12_345), "12.3k");
        assert_eq!(short(1_000_000), "1M");
        assert_eq!(short(1_400_000), "1.4M");
        assert_eq!(short(1_000_000_000), "1B");
        assert_eq!(short(2_500_000_000), "2.5B");
    }

    #[test]
    fn tokens_are_estimated_by_kind_of_character() {
        use super::estimate_tokens;
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("  \n"), 0, "空白不算");
        assert_eq!(estimate_tokens("think"), 1, "英文词每 7 个字母一个");
        assert_eq!(estimate_tokens("tokenizers"), 2);
        assert_eq!(estimate_tokens("the cat sat"), 3);
        assert_eq!(estimate_tokens("1+2=3"), 4, "数字各一个，符号各半个");
        assert_eq!(estimate_tokens("思考一下"), 3, "汉字各 0.7 个，零头进一");
        assert_eq!(estimate_tokens("é"), 1, "别的非 ASCII 字各一个");
    }

    #[test]
    fn tenths_keep_one_decimal() {
        assert_eq!(percent_tenths(1_800, 1_000_000), "0.2");
        assert_eq!(percent_tenths(41_100, 1_000_000), "4.1");
        assert_eq!(percent_tenths(120_000, 1_000_000), "12");
        assert_eq!(percent_tenths(5, 0), "0");
    }

    #[test]
    fn clock_matches_the_old_footer() {
        use super::clock;
        assert_eq!(clock(12), "12s");
        assert_eq!(clock(65), "1m 05s");
        assert_eq!(clock(3725), "1h 02m 05s");
    }

    #[test]
    fn hit_rate_shows_a_decimal_only_above_99() {
        use super::hit_rate;
        assert_eq!(hit_rate(94, 100), "94");
        assert_eq!(hit_rate(9_860, 10_000), "99", "98.6 还不到 99，照整数写");
        assert_eq!(hit_rate(9_900, 10_000), "99", "99.0 不写 .0");
        assert_eq!(hit_rate(9_963, 10_000), "99.6");
        assert_eq!(hit_rate(9_990, 10_000), "99.9");
        assert_eq!(hit_rate(9_996, 10_000), "100", "四舍五入到 100.0 的写 100");
        assert_eq!(hit_rate(0, 0), "0");
    }

    #[test]
    fn step_seconds() {
        use std::time::Duration;
        assert_eq!(seconds(Duration::from_millis(11_840)), "11.8s");
        assert_eq!(seconds(Duration::from_secs(65)), "1m 05s");
        assert_eq!(seconds(Duration::from_secs(3725)), "1h 02m 05s");
    }
}
