//! 「MCP」那一页的服务列表（蓝图「配置页」第 43b 条，2026-10-22 项目主人定）：一个服务一行，名字、暗色的状态（在跑的接着
//! 写几件工具）；要登录的状态用警示色，右边一个「登录」，同底栏的按钮：强调色、悬停加粗，点得到；登录过的右边是「登出」。
//! 名字照最长的对齐。

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::pages::Painted;
use crate::settings::pages::mcp::Server;
use crate::settings::{Hit, Settings, Texts};
use crate::theme;

/// 一行的状态写什么：要登录的写「要登录」，在跑的接着写几件工具（知道的话）。
fn state(server: &Server, texts: &Texts) -> String {
    let words = &texts.more.mcp;
    let key = if server.needs_login() {
        "needs_login"
    } else {
        server.state.as_str()
    };
    let name = words
        .states
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string());
    match server.tools.filter(|_| server.state == "running") {
        Some(n) => format!(
            "{name}{}{}",
            crate::sheet::sep(),
            words.tools.replace("{n}", &n.to_string())
        ),
        None => name,
    }
}

/// 排成的行：字、是不是选中的那一行。「登录」不在字里，画完再照 [`buttons`] 叠上去。
pub(super) fn lines(list: &[Server], at: usize, texts: &Texts, width: u16) -> Vec<Painted> {
    let name_w = list.iter().map(|s| s.name.width()).max().unwrap_or(0) + 2;
    let words = &texts.more.mcp;
    let button_w = words.login.width().max(words.logout.width()) + 2;
    list.iter()
        .enumerate()
        .map(|(i, server)| {
            // 左边空两格画选中的竖条（`paint`）；右边给「登录」留出地方。
            let room = usize::from(width).saturating_sub(2 + name_w + button_w);
            let shown = super::fit(&state(server, texts), room);
            let style = if server.needs_login() {
                theme::warn()
            } else {
                theme::dim()
            };
            let spans = vec![
                Span::raw(format!(
                    "{}{}",
                    server.name,
                    " ".repeat(name_w - server.name.width())
                )),
                Span::styled(shown, style),
            ];
            (Line::from(spans), i == at)
        })
        .collect()
}

/// 要登录的那几行右边叠上「登录」、登录过的叠上「登出」，记下点得到的地方（[`Hit::Login`]）。和 `paint` 照同一个起点算
/// 哪几行看得见。
pub(super) fn buttons(frame: &mut Frame, body: Rect, page: &mut Settings, texts: &Texts) {
    let list = page.more.servers();
    let words = &texts.more.mcp;
    let height = usize::from(body.height);
    let first = page.page_top.1;
    for (n, (i, server)) in list.iter().enumerate().skip(first).take(height).enumerate() {
        let label = if server.needs_login() {
            &words.login
        } else if server.logged_in {
            &words.logout
        } else {
            continue;
        };
        let w = u16::try_from(label.width()).unwrap_or(0);
        if body.width < w + 4 {
            continue;
        }
        let y = body.y + u16::try_from(n).unwrap_or(0);
        let area = Rect::new(body.right().saturating_sub(w + 2), y, w, 1);
        let hovered = page.hover.is_some_and(|at| area.contains(at));
        let style = if hovered {
            theme::accent().add_modifier(Modifier::BOLD)
        } else {
            theme::accent()
        };
        frame
            .buffer_mut()
            .set_line(area.x, y, &Line::styled(label.clone(), style), w);
        page.hits.push((area, Hit::Login(i)));
    }
}

#[cfg(test)]
mod tests;
