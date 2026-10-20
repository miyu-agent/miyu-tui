//! 「更多供应商…」浮窗里的一块（「第一次打开的引导」第 16a 条）：最上面一行搜索（打的字、光标；空着暗色写说明），空一行，
//! 下面一家一行（接不上的暗着），放不下的照模型列表开窗、上下写还有几个。

use ratatui::text::{Line, Span};

use super::content::Content;
use crate::oobe::Texts;
use crate::oobe::model::more::More;
use crate::theme;

/// 拼浮窗：标题、里面的一块、按键提示。`size` 是窗里写字那一块的宽、高。
pub fn popup(more: &More, texts: &Texts, (width, height): (u16, u16)) -> (String, Content, String) {
    let words = &texts.model;
    let mut c = Content::sized(width, height);
    let query = more.filter.text();
    let lead = "/ ";
    let editing = more.filter.editing();
    if query.is_empty() {
        // 按键模式是 Vim 时没在搜：写「/ 搜索」（按 `/` 才进去）。
        let cue = if editing {
            vec![
                Span::styled(lead, theme::faint()),
                Span::styled(words.more_search.clone(), theme::faint()),
            ]
        } else {
            vec![Span::styled(words.search_cue.clone(), theme::faint())]
        };
        c.lines.push(Line::from(cue));
    } else {
        let style = if editing {
            ratatui::style::Style::new()
        } else {
            theme::dim()
        };
        c.lines.push(Line::from(vec![
            Span::styled(lead, theme::faint()),
            Span::styled(query.to_string(), style),
        ]));
    }
    if editing {
        let col = unicode_width::UnicodeWidthStr::width(
            &query[..more.filter.editor.cursor().min(query.len())],
        );
        c.caret = Some((0, u16::try_from(lead.len() + col).unwrap_or(u16::MAX)));
    }
    c.blank();
    match &more.all {
        None => c.note(&words.loading, theme::dim()),
        Some(_) => {
            let found = more.matches();
            if found.is_empty() {
                c.note(&words.no_match, theme::dim());
            } else {
                let rows = found
                    .iter()
                    .map(|p| {
                        let style = if p.supported {
                            ratatui::style::Style::new()
                        } else {
                            theme::faint()
                        };
                        vec![Span::styled(p.name.clone(), style)]
                    })
                    .collect();
                c.window(rows, more.cursor, 0, (&words.more_above, &words.more_below));
            }
        }
    }
    (words.more_title.clone(), c, keys(more, texts).clone())
}

/// 浮窗的按键提示：按键模式是方向键的「输入以搜索」，开着的「/ 搜索 · j/k」，按了 `/` 在搜的「Ctrl+J/K」。
pub fn keys<'a>(more: &More, texts: &'a Texts) -> &'a String {
    match (more.vim, more.filter.editing()) {
        (false, _) => &texts.keys.more,
        (true, false) => &texts.keys.more_vim,
        (true, true) => &texts.keys.searching,
    }
}
