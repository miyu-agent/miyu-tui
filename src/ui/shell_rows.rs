//! 你用 `!` 打的命令（蓝图 `tui.md`「人自己跑命令 `!`」第 4 条，排期第 21 条）：竖线照你说的话，「! 命令行」强调色；下面一块
//! 输出（暗色，长的只留头尾，中间写省了几行）；跑着时末尾转圈，跑完一行暗字「退出码 0 · 1.2s」，非 0 的、被信号杀的、超时的
//! 红字，被打断、核心重启没跑完的暗字。

use ratatui::style::Style;
use ratatui::text::Span;

use super::rows::{Ctx, Row};
use crate::core::ShellRan;
use crate::input::wrap_words;
use crate::theme;
use crate::transcript::{Entry, Shell};

/// 输出留头尾各几行。
const KEEP: usize = 10;

/// 排成的行。
pub fn rows(entry: &Entry, shell: &Shell, ctx: &Ctx) -> Vec<Row> {
    let bar = Span::styled(
        ctx.config.layout.user_bar.clone(),
        theme::user_bar(entry.level.unwrap_or(ctx.level)),
    );
    let line = |content: Vec<Span<'static>>| ctx.row(bar.clone(), content);
    let mut out = vec![line(Vec::new())];
    let width = ctx.width.saturating_sub(2).max(1);
    let command = entry.text.trim_matches('\n');
    for (n, l) in wrap_words(command, width).iter().enumerate() {
        let lead = if n == 0 { "! " } else { "  " };
        out.push(line(vec![
            Span::styled(lead, theme::accent()),
            Span::styled(command[l.start..l.end].to_string(), theme::accent()),
        ]));
    }
    match &shell.ran {
        Some(ran) => {
            out.extend(output(ran, ctx).into_iter().map(|s| line(vec![s])));
            let (text, style) = status(ran, ctx);
            out.push(line(vec![Span::styled(text, style)]));
        }
        None if !entry.queued => {
            let tl = &ctx.config.timeline;
            let spin = tl.spinner[ctx.frame % tl.spinner.len().max(1)].clone();
            out.push(line(vec![Span::styled(
                format!("{spin} {}", ctx.config.text.shell.running),
                theme::dim(),
            )]));
        }
        None => {}
    }
    out.push(line(Vec::new()));
    out
}

/// 输出，一行一段：制表符展开、照宽度折行；多于头尾各 [`KEEP`] 行的中间写省了几行。
fn output(ran: &ShellRan, ctx: &Ctx) -> Vec<Span<'static>> {
    let text = crate::tabs::expand(
        ran.text.trim_end_matches('\n'),
        ctx.config.layout.output.tab,
    );
    if text.is_empty() {
        return Vec::new();
    }
    let width = ctx.width.max(1);
    let lines: Vec<String> = wrap_words(&text, width)
        .iter()
        .map(|l| text[l.start..l.end].to_string())
        .collect();
    let dim = |s: String| Span::styled(s, theme::dim());
    if lines.len() <= 2 * KEEP {
        return lines.into_iter().map(dim).collect();
    }
    let skipped = lines.len() - 2 * KEEP;
    let note = ctx
        .config
        .text
        .omitted
        .replace("{count}", &skipped.to_string());
    let mut out: Vec<Span<'static>> = lines[..KEEP].iter().cloned().map(dim).collect();
    out.push(dim(note));
    out.extend(lines[lines.len() - KEEP..].iter().cloned().map(dim));
    out
}

/// 跑完那一行：写什么、什么颜色。
fn status(ran: &ShellRan, ctx: &Ctx) -> (String, Style) {
    let words = &ctx.config.text.shell;
    let elapsed = crate::meter::seconds(std::time::Duration::from_millis(
        ran.duration_ms.unwrap_or_default(),
    ));
    match (ran.reason.as_deref(), ran.exit, ran.signal) {
        (Some("timed_out"), ..) => (words.timed_out.clone(), theme::error()),
        (Some("interrupted"), ..) => (words.interrupted.clone(), theme::dim()),
        (Some("aborted"), ..) => (words.aborted.clone(), theme::dim()),
        (_, Some(code), _) => (
            words
                .exited
                .replace("{code}", &code.to_string())
                .replace("{elapsed}", &elapsed),
            if code == 0 {
                theme::dim()
            } else {
                theme::error()
            },
        ),
        (_, None, Some(signal)) => (
            words
                .signal
                .replace("{signal}", &signal.to_string())
                .replace("{elapsed}", &elapsed),
            theme::error(),
        ),
        _ => (words.aborted.clone(), theme::dim()),
    }
}
