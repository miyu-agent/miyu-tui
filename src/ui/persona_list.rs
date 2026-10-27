//! 新会话的人格框、预设框（蓝图 `tui.md`「新会话：人格、工作区」第 1、3 条）：和 `/language`、`/model` 一个位置、一个样子。一个
//! 人格一行，只写名字（没有的写编号）；默认的那个右边暗色写「默认」；写错的暗着、后面黄字写原因，选不了。

use ratatui::text::Span;
use serde::Deserialize;

use super::panel::{self, Chrome, Row};
use crate::app::new_session::Kind;
use crate::core::Persona;
use crate::theme;

/// 人格框和人格那一行的字。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Texts {
    /// 上边框写的。
    pub title: String,
    /// 新会话开头、列表还没读回来时，输入框的位置写的（2026-10-21）。
    pub loading: String,
    /// 下边框的按键提示。
    pub hint: String,
    /// 默认人格右边写的。
    pub default_mark: String,
    /// 不带人格那一行的名字（2026-10-08 项目主人定叫「无人格」）。
    pub none: String,
    /// 老会话的「无工具」（核心 P-17 以前开的，订阅回应里 `preset: null`）：列表里已经没有它，查不到名字时写这个。
    pub no_tools: String,
    /// 它后面暗色写的。
    pub none_note: String,
    /// 开了会话再打 `/persona`、`/preset`：框最上面暗色写的（第 2a 条，项目主人原话）。
    pub switch_note: String,
    /// 在回答时打 `/persona`、`/preset`。
    pub busy: String,
    /// 正文里换了人格那一行，`{name}`。
    pub switched: String,
    /// 正文里换了预设那一行，`{name}`。
    pub preset_switched: String,
    /// 首页框下面那一行、侧边栏：`{name}` 是选的人格。
    pub chosen: String,
    /// 预设框上边框写的。
    pub preset_title: String,
    /// 默认预设用不了时按 `Esc`（开会话一定要有预设，核心 Y12）。
    pub preset_need: String,
    /// 「无人格」、写错了的那一行上按 `e`（第 2b 条）。
    pub cant_edit: String,
    /// 首页框下面那一行、侧边栏：`{name}` 是选的预设。
    pub preset_chosen: String,
}

/// 框（标题照是人格还是预设、提示）、排好的行、每一行是第几个。`current` 是现在用的那一个（无人格是空的编号）：开着会话的
/// 是会话用的，还没开会话、已经选过的是选的（2026-10-20 项目主人报：空会话选过再打开没有圆点），还没选的是 `None`；它行首写
/// `dot`（选中了也照旧），不写「默认」。`switching` 是开着会话、中途换（「新会话：人格、工作区」第 2a 条）：最上面一行暗色
/// 写切换前先压缩。
#[allow(clippy::too_many_arguments)]
pub fn lines(
    list: &[Persona],
    default: Option<&str>,
    (current, switching): (Option<&str>, bool),
    (dot, warn): (&str, &str),
    kind: Kind,
    selected: usize,
    texts: &Texts,
    width: u16,
    max: usize,
) -> (
    Chrome,
    Vec<ratatui::text::Line<'static>>,
    Vec<Option<usize>>,
) {
    let title = match kind {
        Kind::Persona => &texts.title,
        Kind::Preset => &texts.preset_title,
    };
    let chrome = Chrome::new(title, Vec::new()).hint(&texts.hint);
    let rows: Vec<Row> = list
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (content, right) = match &p.problem {
                Some(problem) => (
                    vec![
                        Span::styled(p.label().to_string(), theme::dim()),
                        Span::styled(format!("  {problem}"), theme::warn()),
                    ],
                    None,
                ),
                None => {
                    // 中途换的：现在用的那一个行首写 `●`（下面），右边不写；还没开会话的：默认的右边写「默认」。
                    let mark = (current.is_none() && default == Some(p.id.as_str()))
                        .then(|| Span::styled(texts.default_mark.clone(), theme::dim()));
                    let mut content = vec![Span::raw(p.label().to_string())];
                    // 预设后面暗色写它的描述（核心交回的 `summary`，照连接的语言挑好；2026-10-23 项目主人定，没有的不写；
                    // 编号空着的「无工具」也照它）；人格框「无人格」那一行（编号空着）写它意味着什么。
                    let note = if kind == Kind::Preset {
                        p.summary.as_deref().filter(|s| !s.trim().is_empty())
                    } else if p.id.is_empty() {
                        Some(texts.none_note.as_str())
                    } else {
                        None
                    };
                    if let Some(note) = note {
                        content.push(Span::styled(format!("  {note}"), theme::dim()));
                    }
                    (content, mark)
                }
            };
            let on = current == Some(p.id.as_str());
            let lead = panel::current_lead(dot, on);
            (
                Some(i),
                panel::item_with(i == selected, lead, content, right, width),
            )
        })
        .collect();
    let mut rows = rows;
    if switching {
        // 黄字、前面一个警示记号（2026-10-23 项目主人：「加个什么符号警醒一下会不会好一些？或者这句话应该高亮？」）。
        let note = vec![Span::styled(
            format!("{warn}{}", texts.switch_note),
            theme::warn(),
        )];
        rows.insert(0, (None, panel::item(false, note, None, width)));
        // 那一句和下面的列表之间空一行（2026-10-23 项目主人，人格框、预设框一样）。
        rows.insert(1, (None, panel::item(false, Vec::new(), None, width)));
    }
    let rows = panel::fit(rows, Some(selected), max);
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

/// 从框进去改了正在用的、改成了，问要不要压缩（第 2b 条）：框不写标题（同网页），第一行是项目主人那句话（放不下的折行），
/// 下面「是」「否」。
pub fn changed_lines(
    kind: Kind,
    yes: bool,
    words: &crate::settings::in_use::InUseTexts,
    width: u16,
    max: usize,
) -> (
    Chrome,
    Vec<ratatui::text::Line<'static>>,
    Vec<Option<usize>>,
) {
    let line = match kind {
        Kind::Persona => &words.persona,
        Kind::Preset => &words.preset,
    };
    let chrome = Chrome::default();
    let room = usize::from(width).saturating_sub(2).max(1);
    let mut rows: Vec<Row> = crate::cells::wrap(line, room)
        .into_iter()
        .map(|part| (None, panel::item(false, vec![Span::raw(part)], None, width)))
        .collect();
    for (i, button) in words.buttons.iter().enumerate() {
        let picked = (i == 0) == yes;
        let line = panel::item(picked, vec![Span::raw(button.clone())], None, width);
        rows.push((Some(i), line));
    }
    let rows = panel::fit(rows, Some(usize::from(!yes)), max);
    let (map, lines) = rows.into_iter().unzip();
    (chrome, lines, map)
}

#[cfg(test)]
mod tests;
