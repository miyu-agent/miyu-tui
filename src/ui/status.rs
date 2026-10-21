//! 输入框上面的一块（蓝图 `tui.md`「运行状态行和排队的消息」「提示」）：回答进行中是一个词（从词库挑，`pulse.rs`）
//! 连着一到三个点（跟着流光轮换），一起被流光扫，再是这一轮的用时和重试；下面是排队的消息；提示是浮在上面的小框。

use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use unicode_width::UnicodeWidthStr;

use super::rows::clip;
use crate::app::{MascotState, Seen};
use crate::config::Dots;
use crate::theme;

/// 画那一行：从词库挑的词连着一到三个点，一起被流光扫（流光明暗），补到最宽的词那么宽，再写这一轮的用时、重试。
pub fn draw(frame: &mut Frame, area: Rect, seen: &Seen, mascot: &mut MascotState) {
    let Some(start) = seen.talk.transcript.running else {
        return;
    };
    let now = Instant::now();
    let beat = seen.talk.transcript.beat();
    let word = mascot
        .pulse
        .word(start, beat, now, &seen.config.pulse)
        .to_string();
    let layout = &seen.config.layout;
    // 流光一直照工作区那一档的颜色，不跟着权限级别变（2026-10-17 项目主人：开放权限时整行变红不对）；照级别的颜色算、
    // 不写死强调色，终端配色时和模式字对得上（2026-10-14 项目主人定）。
    let base = theme::level_color(crate::core::Level::Workspace);
    // 换了词，亮光从头扫（`tui.md`「运行状态行和排队的消息」第 1 条）。
    let t = mascot
        .pulse
        .shown()
        .map_or(0.0, |s| now.saturating_duration_since(s).as_secs_f64());
    let chars: Vec<char> = swept(&word, &layout.dots, t, layout.shimmer.sweep_seconds)
        .chars()
        .collect();
    let mut spans: Vec<Span> = chars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            Span::styled(
                c.to_string(),
                theme::shimmer_on(base, i, chars.len(), t, &layout.shimmer),
            )
        })
        .collect();
    // 补到词库里最宽的词那么宽，用时的位置不跟着词长短跳。
    let pad = seen.config.pulse.widest().saturating_sub(word.width()) + 1;
    spans.push(Span::raw(" ".repeat(pad)));
    let rest = theme::shimmer_rest_on(base, &layout.shimmer);
    spans.push(Span::styled(
        crate::meter::clock(start.elapsed().as_secs()),
        rest,
    ));
    if let Some(retry) = &seen.talk.transcript.retry {
        spans.push(Span::styled(crate::sheet::sep(), theme::dim()));
        spans.push(Span::styled(retry.clone(), theme::warn()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// 被流光扫的那一截：词后面接着点，同一道亮光一路扫过去；点的个数跟着流光轮换，一趟亮光轮一圈，
/// 没写满的格子留空，长度不变（`tui.md`「运行状态行和排队的消息」第 2 条）。`t` 是这个词换上以后过了几秒。
pub(super) fn swept(word: &str, dots: &Dots, t: f64, sweep: f64) -> String {
    let count = dots.count.max(1);
    let phase = if sweep > 0.0 {
        t.rem_euclid(sweep) / sweep
    } else {
        0.0
    };
    // 一趟分成 `count` 段，第几段就写几个点。
    let shown = ((phase * count as f64) as usize).min(count - 1) + 1;
    let gap = " ".repeat((count - shown) * dots.mark.width());
    format!("{word}{}{gap}", dots.mark.repeat(shown))
}

/// 提示：圆角细边框的小框，底边压在 `bottom` 那一行，框里的字从 `text_x` 起（和输入框里的字左对齐），
/// 左右各空一格。浮着，先把底下的格子清掉再画（`tui.md`「提示」）。字有好几行的（扩展办的斜杠命令的回执，核心 X-14
/// 下）照行数长高，一行一行各自截，宽照最宽的一行；上面放不下的只露最后几行。
pub fn toast(frame: &mut Frame, text_x: u16, bottom: u16, max_width: u16, seen: &Seen) {
    let Some(notice) = &seen.typing.notice else {
        return;
    };
    let room = max_width.saturating_sub(4);
    let raw: Vec<&str> = notice.text.lines().collect();
    let last = raw.len().saturating_sub(1);
    let lines: Vec<String> = raw
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let tail = if i == last { notice.tail.as_str() } else { "" };
            fitted(line, tail, room)
        })
        .collect();
    let lines = if lines.is_empty() {
        vec![fitted("", &notice.tail, room)]
    } else {
        lines
    };
    let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
    let width = u16::try_from(width).unwrap_or(0) + 4;
    let wanted = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let Some(area) = toast_area(text_x, bottom, width, wanted, frame.area()) else {
        return;
    };
    let shown = usize::from(area.height.saturating_sub(2));
    let lines: Vec<Line> = lines[lines.len() - shown.min(lines.len())..]
        .iter()
        .map(|l| Line::raw(l.clone()))
        .collect();
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_set(crate::sheet::read(|s| s.frame.toast.set()))
        .border_style(theme::notice_border(notice.good));
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    )
    .intersection(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

/// 提示框里写的：放得进 `width` 列的照原样；放不下的截 `text` 加 `…`，后面的 `tail` 整个留着（`/goal` 看进度的
/// 「 · 进行中 · 第 1 轮」，「长期目标」第 1 条）；`tail` 自己都放不下的整句截。
fn fitted(text: &str, tail: &str, width: u16) -> String {
    let room = usize::from(width);
    let tail_width = tail.width();
    if tail_width >= room {
        return clip(&format!("{text}{tail}"), width);
    }
    let rest = u16::try_from(room - tail_width).unwrap_or(width);
    format!("{}{tail}", clip(text, rest))
}

/// 提示的框放在哪：`lines` 行字加上下边框高，底边在 `bottom` 那一行；上面放不下的少露几行，一行都放不下就不画，
/// 不往下盖到输入框上（`tui.md`「窗口小的时候」第 2 条）。
fn toast_area(text_x: u16, bottom: u16, width: u16, lines: u16, screen: Rect) -> Option<Rect> {
    let above = bottom.checked_sub(screen.y)?;
    let lines = lines.max(1).min(above.checked_sub(1)?);
    if lines == 0 {
        return None;
    }
    let top = bottom - lines - 1;
    Some(Rect::new(text_x.saturating_sub(2), top, width, lines + 2).intersection(screen))
}

/// 排队的消息：一条一行，暗色的记号加这句话，放不下截掉加 `…`（`tui.md`「运行状态行和排队的消息」第 5 条）。
pub fn queued(frame: &mut Frame, area: Rect, texts: &[String], mark: &str) {
    let lines: Vec<Line> = texts
        .iter()
        .take(usize::from(area.height))
        .map(|t| Line::styled(clip(&format!("{mark}{t}"), area.width), theme::dim()))
        .collect();
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{fitted, swept, toast_area};
    use crate::config::Config;

    #[test]
    fn a_long_toast_keeps_its_tail_and_clips_the_words_before_it() {
        // 2026-10-12 项目主人：`/goal` 看进度，原文长了截原文，后面的「 · 进行中 · 第 1 轮」整个留着。
        let text = "用 write 工具在当前目录写 count.txt，内容是 3";
        let tail = " · 进行中 · 第 1 轮";
        let shown = fitted(text, tail, 30);
        assert!(shown.ends_with("… · 进行中 · 第 1 轮"), "{shown}");
        assert!(unicode_width::UnicodeWidthStr::width(shown.as_str()) <= 30);
        assert_eq!(fitted("短的", tail, 30), "短的 · 进行中 · 第 1 轮");
        assert_eq!(fitted("没有结尾的长长长长长长", "", 8), "没有结…");
        assert_eq!(fitted("结尾自己就放不下", tail, 6), "结尾…", "整句截");
    }

    #[test]
    fn a_toast_without_three_rows_above_the_box_is_not_drawn() {
        let screen = Rect::new(0, 0, 30, 7);
        // 输入框在第 4 行起：底边压在第 3 行，框占第 1 到 3 行。
        assert_eq!(
            toast_area(5, 3, 12, 1, screen),
            Some(Rect::new(3, 1, 12, 3))
        );
        assert_eq!(
            toast_area(5, 2, 12, 1, screen),
            Some(Rect::new(3, 0, 12, 3))
        );
        // 30×7 的首页：输入框在最上面，框上面没有地方。
        assert_eq!(toast_area(5, 1, 12, 1, screen), None, "不往下盖到输入框上");
        assert_eq!(toast_area(5, 0, 12, 1, screen), None);
        // 好几行的照行数长高；上面放不下的少露几行（扩展办的斜杠命令的回执，核心 X-14 下）。
        assert_eq!(
            toast_area(5, 6, 12, 3, screen),
            Some(Rect::new(3, 2, 12, 5))
        );
        assert_eq!(
            toast_area(5, 3, 12, 4, screen),
            Some(Rect::new(3, 0, 12, 4))
        );
    }

    #[test]
    fn the_dots_are_swept_with_the_word_and_count_up_with_the_sweep() {
        let layout = Config::builtin().unwrap().layout;
        let (dots, sweep) = (&layout.dots, layout.shimmer.sweep_seconds);
        // 一趟亮光轮一圈：一个、两个、三个，再回到一个；没写满的格子留空，长度不变。
        assert_eq!(swept("思考中", dots, 0.0, sweep), "思考中.  ");
        assert_eq!(
            swept("思考中", dots, sweep / 3.0 + 0.01, sweep),
            "思考中.. "
        );
        assert_eq!(
            swept("思考中", dots, sweep * 2.0 / 3.0 + 0.01, sweep),
            "思考中..."
        );
        assert_eq!(swept("思考中", dots, sweep + 0.01, sweep), "思考中.  ");
    }
}
