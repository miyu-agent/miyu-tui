//! 说法的模板原文（`human.get` 的 `said`）：只留原文，用到哪句解哪句（蓝图 `tui.md`「在哪」`src/human.rs` 那一行）。
//!
//! 一种语言一千多句，一个界面一次用到的只有几句：原来一拿到回应就全解成模板，连键、原文的拷贝住着约 470 KB。现在所有的
//! 键和原文接成一整串，另记每一句到哪为止，键照字节的先后排好，查的时候对半找；解模板留到要说那一句的时候。
//!
//! 单数那一句（`X/one`，核心补的）和 `X` 一字不差的（中文、日文全是）不另存，`X` 那一句记一笔「单数同它」，要 `X/one` 时
//! 照它答；和 `X` 不一样的（英文写了单数的）照常存。

use serde_json::Value;

/// 单数那一句的编号的结尾。
const ONE: &str = "/one";
/// 记在键的结尾上的一笔：这一句的单数那一句和它一字不差。
const TWIN: u32 = 1 << 31;

/// 每一句说法的模板原文。
#[derive(Debug, Clone, Default)]
pub(super) struct Phrases {
    /// 一句接一句：每一句的键后面紧跟着它的原文。
    text: String,
    /// 每一句的键到哪为止（也是原文的开头，最高一位是 [`TWIN`]）、原文到哪为止（也是下一句键的开头），在 `text` 里的字节
    /// 位置；照键排好。
    ends: Vec<[u32; 2]>,
}

impl Phrases {
    /// 照回应的 `said`（键到模板原文）记下：不是字的那一句不要；整串长过 2 GiB 的，放不下的那几句不要。
    pub(super) fn from_said(said: &Value) -> Self {
        let all = said.as_object();
        let source = |key: &str| all.and_then(|all| all.get(key)).and_then(Value::as_str);
        let mut entries: Vec<(&str, &str, bool)> = all
            .into_iter()
            .flatten()
            .filter_map(|(key, value)| {
                let text = value.as_str()?;
                // 和 `X` 一字不差的 `X/one`：记在 `X` 上，不另存。
                if key
                    .strip_suffix(ONE)
                    .is_some_and(|base| source(base) == Some(text))
                {
                    return None;
                }
                let twin = source(&format!("{key}{ONE}")) == Some(text);
                Some((key.as_str(), text, twin))
            })
            .collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let mut text =
            String::with_capacity(entries.iter().map(|(k, s, _)| k.len() + s.len()).sum());
        let mut ends = Vec::with_capacity(entries.len());
        for (key, source, twin) in entries {
            let key_end = text.len() + key.len();
            let end = key_end + source.len();
            let (Ok(key_end), Ok(end)) = (u32::try_from(key_end), u32::try_from(end)) else {
                break;
            };
            if end >= TWIN {
                break;
            }
            text.push_str(key);
            text.push_str(source);
            ends.push([if twin { key_end | TWIN } else { key_end }, end]);
        }
        Self { text, ends }
    }

    /// 键是 `key` 的那一句的模板原文；没有的是空的。
    pub(super) fn get(&self, key: &str) -> Option<&str> {
        if let Some(i) = self.find(key) {
            return Some(self.entry(i).1);
        }
        let i = self.find(key.strip_suffix(ONE)?)?;
        (self.ends[i][0] & TWIN != 0).then(|| self.entry(i).1)
    }

    /// 键是 `key` 的那一句排第几；对半找。
    fn find(&self, key: &str) -> Option<usize> {
        let (mut low, mut high) = (0, self.ends.len());
        while low < high {
            let middle = low + (high - low) / 2;
            match self.entry(middle).0.cmp(key) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Some(middle),
            }
        }
        None
    }

    /// 第 `i` 句的键和原文。
    fn entry(&self, i: usize) -> (&str, &str) {
        let at = |n: u32| (n & !TWIN) as usize;
        let start = i
            .checked_sub(1)
            .map_or(0, |before| at(self.ends[before][1]));
        let [key_end, end] = self.ends[i];
        let (key_end, end) = (at(key_end), at(end));
        (&self.text[start..key_end], &self.text[key_end..end])
    }

    /// 另存了几句（和 `X` 一字不差的 `X/one` 不算）。
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.ends.len()
    }
}

#[cfg(test)]
mod tests;
