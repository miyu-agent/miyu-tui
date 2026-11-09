//! `$` 点名技能（蓝图 `tui.md`「`$` 点名技能」，发版前 check 26，2026-11-09 项目主人：技能不出现在斜杠命令列表里，输入框里
//! 用 `$` 点名，一句话里能点好几个）：光标前面是 `$` 打头的词时开列表，照名字筛 `command.catalog` 的 `skills`（核心 SK-1）；
//! 发出去时从字里认出点了哪几个，交 `session.send` 的 `skills`；正文里照字里的位置给 `$名字` 上色。开没开、选中哪一条、
//! `Esc` 关掉以后什么时候再开记在 [`Picker`]；按键在 `app/skill_word.rs`，画在 `ui/skill_list.rs`。

#[cfg(test)]
mod tests;

use crate::core::CoreSkill;
use crate::mention::Mode;

/// 光标前面的 `$` 词：`$` 在第几个字节、`$` 后面打的字。`$` 在最前面或者前面是空白，到光标为止没有空白（同 `@`，和网页
/// W-1124 一个认法）。不是的交回 `None`。
pub fn token(text: &str, cursor: usize) -> Option<(usize, String)> {
    let before = text.get(..cursor)?;
    let at = before.rfind('$')?;
    let lead = before[..at].chars().next_back();
    if lead.is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    let query = &before[at + 1..];
    (!query.chars().any(char::is_whitespace)).then(|| (at, query.to_string()))
}

/// 名字里能有的字：碰到别的（空白、标点）名字就到头了。
fn name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/')
}

/// 字里点了的技能：`$名字` 的字节范围、名字。`$` 在最前面或者前面是空白；名字照 `skills` 里有的认，后面紧跟的不能还是
/// 名字里的字（`$pdf2` 不算点了 `pdf`），句末的标点不算（`$pdf，` 算）。几个名字都对得上的取最长的。
pub fn marks(text: &str, skills: &[CoreSkill]) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut prev: Option<char> = None;
    for (at, c) in text.char_indices() {
        let starts = c == '$' && prev.is_none_or(char::is_whitespace);
        prev = Some(c);
        if !starts {
            continue;
        }
        let rest = &text[at + 1..];
        let best = skills
            .iter()
            .map(|s| s.name.as_str())
            .filter(|name| {
                !name.is_empty()
                    && rest.starts_with(name)
                    && !rest[name.len()..].chars().next().is_some_and(name_char)
            })
            .max_by_key(|name| name.len());
        if let Some(name) = best {
            out.push((at, at + 1 + name.len(), name.to_string()));
        }
    }
    out
}

/// 发出去时交 `session.send` 的 `skills`：字里点了的，照先后，重复的只算一次。
pub fn named(text: &str, skills: &[CoreSkill]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (_, _, name) in marks(text, skills) {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// 这一次列的：`$` 在第几个字节、打的字、对上的技能和对上的是名字里第几个字。
#[derive(Debug, Clone)]
pub struct Found {
    /// `$` 在输入框的字里第几个字节：选定时从这里换到光标。
    pub start: usize,
    /// `$` 后面打的字。
    pub query: String,
    /// 对上的，照对得多好排；空着打的照清单的先后。
    pub items: Vec<(CoreSkill, Vec<usize>)>,
}

/// 列表的状态。
#[derive(Debug, Default)]
pub struct Picker {
    /// 选中的是第几条。
    pub selected: usize,
    /// 鼠标钉住的露出来的那一段（`menu::pin`）；按键选、词变了放开。
    pub pinned: Option<usize>,
    /// 按键模式是 Vim 时在哪一步（同 `@` 列表，发版前 check 16）；列表关了回到打字。
    pub mode: Mode,
    /// 现在的词：变了，选中回到第一条。
    typed: Option<(usize, String)>,
    /// `Esc` 关掉时的词：没变就一直关着。
    dismissed: Option<(usize, String)>,
}

impl Picker {
    /// 忘掉现在的词：没有词了、换了会话；回到打字。
    pub fn forget(&mut self) {
        self.typed = None;
        self.dismissed = None;
        self.mode = Mode::Typing;
    }

    /// 照输入框里的字和光标，定开不开、列哪些；没有技能可点（旧核心、这个会话的预设一个都没开）的不开。
    pub fn find(&mut self, text: &str, cursor: usize, skills: &[CoreSkill]) -> Option<Found> {
        let Some(word) = token(text, cursor).filter(|_| !skills.is_empty()) else {
            self.forget();
            return None;
        };
        if self.typed.as_ref() != Some(&word) {
            self.typed = Some(word.clone());
            self.selected = 0;
            self.pinned = None;
        }
        if self.dismissed.as_ref() == Some(&word) {
            return None;
        }
        self.dismissed = None;
        let (start, query) = word;
        let ranked = crate::fuzzy::rank(&query, skills.iter().collect(), |s| vec![s.name.as_str()]);
        let items: Vec<(CoreSkill, Vec<usize>)> = ranked
            .into_iter()
            .map(|s| (s.clone(), crate::fuzzy::marks(&query, &s.name)))
            .collect();
        self.selected = self.selected.min(items.len().saturating_sub(1));
        Some(Found {
            start,
            query,
            items,
        })
    }

    /// `↑`、`↓`（Vim 的 `k`、`j`）：上下挪，到头绕回来；放开鼠标钉住的那一段。
    pub fn step(&mut self, down: bool, count: usize) {
        if count == 0 {
            return;
        }
        self.selected = if down {
            (self.selected + 1) % count
        } else {
            (self.selected + count - 1) % count
        };
        self.pinned = None;
    }

    /// `Esc`：关掉，这个词没变就一直关着。
    pub fn dismiss(&mut self, found: &Found) {
        self.dismissed = Some((found.start, found.query.clone()));
        self.mode = Mode::Typing;
    }
}
