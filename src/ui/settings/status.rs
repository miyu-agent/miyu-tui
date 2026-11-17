//! 配置页的状态行（蓝图 `tui.md`「配置页」第 4 条）：细线上面那一行。有话的（回执、警告、出错）一行放不下时往上长，最多
//! [`MOST`] 行，再多的最后一行截掉加 `…`（2026-11-17 项目主人定：存设置的警告、出错的原话长了被截掉）；在筛的字、页里的
//! 数目照旧一行。几页的内容照多出来的行让出地方（[`extra`]）。

use ratatui::style::Style;

use super::fit;
use crate::settings::nav::Page;
use crate::settings::{Settings, Texts, Tone};
use crate::theme;

/// 有话的最多占几行。
const MOST: usize = 3;

/// 状态行的几行（照 `width` 列排，至少一行）和颜色。
pub(super) fn lines(page: &Settings, texts: &Texts, width: u16) -> (Vec<String>, Style) {
    let (text, style, wraps) = status(page, texts);
    if !wraps {
        return (vec![text], style);
    }
    let mut rows: Vec<String> = crate::input::pieces(&text, width.max(1))
        .into_iter()
        .map(|(piece, _)| piece.trim_end().to_string())
        .collect();
    if rows.len() > MOST {
        let cut = format!("{}{}", rows[MOST - 1], rows[MOST]);
        rows.truncate(MOST - 1);
        rows.push(fit(&cut, usize::from(width)));
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    (rows, style)
}

/// 状态行比一行多出几行：几页的内容照它让出地方。
pub(crate) fn extra(page: &Settings, texts: &Texts, width: u16) -> u16 {
    let (rows, _) = lines(page, texts, width);
    u16::try_from(rows.len().saturating_sub(1)).unwrap_or(0)
}

/// 状态行写什么、什么颜色、放不下时折不折行：在筛的写筛的字；有话的写话（折行）；没有的照这一页写模型、池有几个。
fn status(page: &Settings, texts: &Texts) -> (String, Style, bool) {
    if let Some(search) = &page.nav.search {
        let text = format!("/ {}", search.text);
        let style = if search.typing {
            Style::new()
        } else {
            theme::dim()
        };
        return (text, style, false);
    }
    if let Some((text, tone)) = &page.status {
        let style = match tone {
            Tone::Note => theme::warn(),
            Tone::Good => theme::good(),
            Tone::Bad => theme::error(),
            Tone::Busy => theme::dim(),
        };
        return (text.clone(), style, true);
    }
    // 主菜单照记下的清单已经画全了的不写「正在加载配置…」，免得读回来时底下那一行闪一下（2026-10-21）。
    if page.loading() && !(page.on_menu && page.more.schema.is_some()) {
        return (texts.status("loading"), theme::dim(), false);
    }
    if page.on_menu {
        return (String::new(), Style::new(), false);
    }
    let text = match page.nav.page {
        Page::Providers => match page.nav.provider(&page.view).map(|p| p.models.len()) {
            Some(0) => texts.status("no_models"),
            Some(n) => texts.status("fetched").replace("{n}", &n.to_string()),
            None => String::new(),
        },
        Page::Pools => texts
            .status("pools")
            .replace("{n}", &page.view.pools.len().to_string()),
        Page::Defaults => String::new(),
    };
    (text, theme::warn(), false)
}
