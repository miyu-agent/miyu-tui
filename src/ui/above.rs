//! 输入框上面那一块（蓝图「挂载位」第 4 条 SL-2、「连核心」第 8 条）：连不上核心时整句原因红字折行写全（2026-11-03 项目
//! 主人：旧版数据根那句在左下角被截掉了后半句，选「首页中间红字折行写全」；底栏只写前面那一截），下面空一行接挂着的几件
//! （出厂只有待办，第 4a 条）。

use ratatui::text::Line;

use crate::app::App;
use crate::theme;
use crate::transcript::Link;

/// 这一块的几行，照 `width` 列排（首页照首页框里的字宽）。
pub(super) fn lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let todo = super::todos::todo_lines(&app.talk.board, &app.config, width, app.talk.todo_full);
    let parts = super::layout::slot::column(vec![("todo", todo)], (&app.config.layout.above, 1));
    let Link::Down(reason) = &app.talk.transcript.link else {
        return parts;
    };
    let mut out: Vec<Line<'static>> = crate::input::pieces(reason, width.max(1))
        .into_iter()
        .map(|(piece, _)| Line::styled(piece.trim_end().to_string(), theme::error()))
        .collect();
    if !parts.is_empty() {
        out.push(Line::default());
        out.extend(parts);
    }
    out
}

/// 连不上时底栏写的那一截：原因前面「连不上核心」这样的头（到第一个冒号）；没有冒号的照全句。
pub(super) fn head(reason: &str) -> &str {
    reason
        .split_once('：')
        .or_else(|| reason.split_once(": "))
        .map_or(reason, |(head, _)| head)
}

#[cfg(test)]
mod tests {
    use super::head;

    #[test]
    fn the_status_line_keeps_only_the_head_of_a_reason() {
        assert_eq!(
            head("连不上核心：/tmp/x 是旧版 Miyu 的数据根"),
            "连不上核心"
        );
        assert_eq!(
            head("Can't connect to the core: old root"),
            "Can't connect to the core"
        );
        assert_eq!(head("没有冒号的一句"), "没有冒号的一句");
    }
}
