//! 菜单里打字筛的模糊搜索（2026-10-18 项目主人：模型列表搜不到，原来各处都是整串包含）：不分大小写；打的字照先后出现就算
//! 对上，不必挨着（`opus4` 对得上 `claude-opus-4`）；有空格的拆成几段，每段都要对上，可以落在不同的字段里（`cl op`）。
//! 越挨着、越落在词头、越靠前的分越高，照 fzf 的意思。照先后对不上的一段，还可以切成几截、每截至少两个字、各自原样出现，先后
//! 不限（2026-10-19 项目主人：`clinedeepseek` 搜得到 `clinepass/deepseek-v4.1-flash`，`deepseekcline` 搜不到），分数比照先后
//! 对上的低。

/// 打的字照空格拆成的几段；全是空白的是空的（什么都对得上）。
fn terms(query: &str) -> Vec<Vec<char>> {
    query
        .split_whitespace()
        .map(|t| t.chars().flat_map(char::to_lowercase).collect())
        .collect()
}

/// `c` 是不是一个词的开头：最前面，或者前一个是分隔的符号、从小写到大写、从中文到别的。
fn boundary(prev: Option<char>, c: char) -> bool {
    match prev {
        None => true,
        Some(p) => {
            matches!(
                p,
                ' ' | '-' | '_' | '/' | '.' | ':' | '·' | '(' | '[' | '@' | '#'
            ) || (p.is_lowercase() && c.is_uppercase())
                || (p.is_ascii() != c.is_ascii())
        }
    }
}

/// 一段在一个字段里的分：对不上是 `None`。
fn term_score(term: &[char], text: &[char], lower: &[char]) -> Option<i64> {
    term_match(term, text, lower).map(|(score, _)| score)
}

/// 一段在一个字段里的分和对上的那几个字（第几个字）：从每一个能起头的地方往后贪心地找，取最好的。
fn term_match(term: &[char], text: &[char], lower: &[char]) -> Option<(i64, Vec<usize>)> {
    let first = *term.first()?;
    let mut best: Option<(i64, Vec<usize>)> = None;
    for start in (0..lower.len()).filter(|&i| lower[i] == first) {
        let mut at = start;
        let mut score = 0i64;
        let mut last: Option<usize> = None;
        let mut hits = Vec::new();
        let mut ok = true;
        for &want in term {
            let Some(found) = (at..lower.len()).find(|&i| lower[i] == want) else {
                ok = false;
                break;
            };
            score += 1;
            if boundary(found.checked_sub(1).map(|i| text[i]), text[found]) {
                score += 8;
            }
            match last {
                Some(l) if found == l + 1 => score += 12,
                Some(l) => score -= i64::try_from(found - l - 1).unwrap_or(i64::MAX).min(8),
                None => {}
            }
            last = Some(found);
            hits.push(found);
            at = found + 1;
        }
        if !ok {
            break;
        }
        // 越靠前越好：起头每晚一格扣一点，最多扣 10。
        score -= i64::try_from(start).unwrap_or(i64::MAX).min(10);
        if best.as_ref().is_none_or(|(b, _)| score > *b) {
            best = Some((score, hits));
        }
    }
    best
}

/// `query` 在 `field` 这一个字段里对上的字（第几个字，从小到大）：列表里把它们换个颜色（2026-10-23 项目主人：「搜索匹配到
/// 的文字我希望颜色能不一样」）。照先后对上的照那几个字；切成几截才对上的照每截原样出现的那一处。
pub fn marks(query: &str, field: &str) -> Vec<usize> {
    let text: Vec<char> = field.chars().collect();
    let lower: Vec<char> = text
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut out = Vec::new();
    for term in terms(query) {
        match term_match(&term, &text, &lower) {
            Some((_, hits)) => out.extend(hits),
            None => out.extend(piece_marks(&term, &lower)),
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// 切开对上的一段：从前往后取最长的、原样出现在这个字段里的一截（至少 [`PIECE`] 个字），标上它出现的地方。
fn piece_marks(term: &[char], lower: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0;
    while from + PIECE <= term.len() {
        let found = (from + PIECE..=term.len()).rev().find_map(|to| {
            let part = &term[from..to];
            lower
                .windows(part.len())
                .position(|w| w == part)
                .map(|at| (to, at))
        });
        match found {
            Some((to, at)) => {
                out.extend(at..at + (to - from));
                from = to;
            }
            None => from += 1,
        }
    }
    out
}

/// `query` 和这几个字段的分：每一段取它最对得上的那个字段，加起来；有一段哪个字段都对不上是 `None`。空的 `query` 是 0。
pub fn score<'a>(query: &str, fields: impl IntoIterator<Item = &'a str>) -> Option<i64> {
    let fields: Vec<(Vec<char>, Vec<char>)> = fields
        .into_iter()
        .map(|f| {
            let text: Vec<char> = f.chars().collect();
            let lower = text
                .iter()
                .map(|c| c.to_lowercase().next().unwrap_or(*c))
                .collect();
            (text, lower)
        })
        .collect();
    terms(query).iter().try_fold(0i64, |sum, term| {
        let best = fields
            .iter()
            .filter_map(|(text, lower)| term_score(term, text, lower))
            .max()
            .or_else(|| pieces(term, &fields))?;
        Some(sum + best)
    })
}

/// 切开的一截至少几个字：一个字的不算，不然随便几个字母乱序都对得上。
const PIECE: usize = 2;
/// 每多切一刀扣的分：切开的总排在照先后对上的后面。
const SPLIT: i64 = 20;

/// 一段照先后对不上时：切成几截，每截至少 [`PIECE`] 个字、原样出现在某个字段里（先后不限），交回切法里分最高的；切不成是
/// `None`。
fn pieces(term: &[char], fields: &[(Vec<char>, Vec<char>)]) -> Option<i64> {
    let piece = |from: usize, to: usize| -> Option<i64> {
        let part = &term[from..to];
        fields
            .iter()
            .filter(|(_, lower)| lower.windows(part.len()).any(|w| w == part))
            .filter_map(|(text, lower)| term_score(part, text, lower))
            .max()
    };
    // best[i]：前 i 个字切好的最高分。
    let mut best: Vec<Option<i64>> = vec![None; term.len() + 1];
    best[0] = Some(0);
    for to in PIECE..=term.len() {
        best[to] = (0..=to - PIECE)
            .filter_map(|from| Some(best[from]? + piece(from, to)? - SPLIT))
            .max();
    }
    best[term.len()]
}

/// 筛并排：对不上的拿掉，照分数从高到低，同分的照原来的先后。空的 `query` 原样交回。
pub fn rank<'a, T>(query: &str, items: Vec<T>, fields: impl Fn(&T) -> Vec<&'a str>) -> Vec<T>
where
    T: 'a,
{
    if query.trim().is_empty() {
        return items;
    }
    let mut scored: Vec<(i64, usize, T)> = items
        .into_iter()
        .enumerate()
        .filter_map(|(i, item)| score(query, fields(&item)).map(|s| (s, i, item)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, item)| item).collect()
}

/// 对不对得上（分组的列表只筛不排时用）。
pub fn hit<'a>(query: &str, fields: impl IntoIterator<Item = &'a str>) -> bool {
    score(query, fields).is_some()
}

#[cfg(test)]
mod tests;
