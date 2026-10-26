//! 别的公式写法换成 `$`（蓝图「她的回答：Markdown」第 5 条，2026-10-19 项目主人）：她常写 `\[…\]`、`\(…\)`，CommonMark
//! 把 `\[` 当转义的 `[`，原来照字写出来。解析以前换成 `$$…$$`、`$…$`；代码块、行内代码里的不动；没收尾的（回答还在流）
//! 不换，照字写到收尾那一刻。链接地址 `](…)` 里的 `$` 转义成 `\$`：不然和链接文字里的 `$` 凑成一对行内公式，链接拆坏
//! （2026-10-26 项目主人报：`[…-$0.11.html](/…-$0.11.html)` 照原样露出来）。

use std::borrow::Cow;

/// 换好的字；没有这两种写法的原样交回。
pub fn dollars(text: &str) -> Cow<'_, str> {
    let in_link = text.contains("](") && text.contains('$');
    if !text.contains("\\[") && !text.contains("\\(") && !in_link {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut prose = String::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.split_inclusive('\n') {
        let mark = fence_mark(line);
        match (fence, mark) {
            (Some((c, n)), Some((m, k))) if m == c && k >= n && line.trim().len() == k => {
                fence = None;
                out.push_str(line);
            }
            (Some(_), _) => out.push_str(line),
            (None, Some(open)) => {
                out.push_str(&swap(&std::mem::take(&mut prose)));
                fence = Some(open);
                out.push_str(line);
            }
            (None, None) => prose.push_str(line),
        }
    }
    out.push_str(&swap(&prose));
    Cow::Owned(out)
}

/// 这一行是代码块的围栏：（`` ` `` 或 `~`，几个）。
fn fence_mark(line: &str) -> Option<(char, usize)> {
    let t = line.trim_start();
    let c = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = t.chars().take_while(|x| *x == c).count();
    (n >= 3).then_some((c, n))
}

/// 一段不在代码块里的字：行内代码照原样跳过，`\\` 是转义的反斜杠，`\[…\]` 换成 `$$…$$`、`\(…\)` 换成 `$…$`。
fn swap(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '`' {
            let run = rest.chars().take_while(|x| *x == '`').count();
            let ticks = &rest[..run];
            let end = rest[run..].find(ticks).map(|i| run + i + run);
            let take = end.unwrap_or(run);
            out.push_str(&rest[..take]);
            rest = &rest[take..];
            continue;
        }
        // 链接地址：到 `)` 或空白为止，`$` 转义（地址里转义照常认，打开的还是原来的路径）。
        if let Some(after) = rest.strip_prefix("](") {
            let end = after
                .find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(after.len());
            out.push_str("](");
            out.push_str(&after[..end].replace('$', "\\$"));
            rest = &after[end..];
            continue;
        }
        if let Some(after) = rest.strip_prefix("\\\\") {
            out.push_str("\\\\");
            rest = after;
            continue;
        }
        let pair = [("\\[", "\\]", "$$"), ("\\(", "\\)", "$")]
            .into_iter()
            .find(|(open, _, _)| rest.starts_with(open));
        if let Some((open, close, dollar)) = pair
            && let Some(at) = rest[open.len()..].find(close)
        {
            let inner = &rest[open.len()..open.len() + at];
            let inner = if dollar == "$" { inner.trim() } else { inner };
            out.push_str(dollar);
            out.push_str(inner);
            out.push_str(dollar);
            rest = &rest[open.len() + at + close.len()..];
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests;
