//! 留言（`send_message`，蓝图「时间线」第 8 条，2026-10-18 项目主人）：标题写「留言 · 发给谁 · 消息开头」，点开是完整的
//! 消息和附件。原来标题是「留言 · 会话 xxxx · 送到了：xxxx」，点开只有「Message sent to 35106d22.」。

use ratatui::style::Style;

use super::super::rows::Ctx;
use super::step::Piece;
use crate::session_list::{looks_like_session, short};
use crate::theme;
use crate::transcript::Step;

/// 发给谁：别的会话「会话 短编号 · 标题」，子代理「任务编号 · 名字」，父会话「父会话」。
fn target(step: &Step, ctx: &Ctx) -> String {
    let texts = &ctx.config.text;
    let to = step.arg("to").unwrap_or_default().trim();
    if to == "parent" {
        return texts.to_parent.clone();
    }
    if looks_like_session(to) {
        let id = texts.on_session.replace("{id}", &short(to));
        let title = ctx
            .sessions
            .iter()
            .find(|s| s.session == to)
            .and_then(|s| s.title.as_deref())
            .filter(|t| !t.trim().is_empty());
        return title.map_or(id.clone(), |t| format!("{id} · {}", t.trim()));
    }
    let name = ctx
        .board
        .and_then(|b| b.jobs.iter().find(|j| j.job == to))
        .map(|j| j.title.clone());
    name.map_or(to.to_string(), |n| format!("{to} · {n}"))
}

/// 标题里名字后面那一截（2026-10-18 项目主人：「留言 · 会话 xxxx · 会话名称 · 消息开头」）：发给谁，再接消息的第一行，
/// 太长的由那一行截掉加 `…`。不再接结果那一句（「送到了：xxxx」和前面重了）。
pub fn title(step: &Step, ctx: &Ctx) -> String {
    let message = step.arg("message").unwrap_or_default();
    let first = message.lines().map(str::trim).find(|l| !l.is_empty());
    match first {
        Some(line) => format!("{} · {line}", target(step, ctx)),
        None => target(step, ctx),
    }
}

/// 点开：完整的消息，带了附件的再一行写文件名；`each` 把一段字照宽度折成行。
pub fn pieces(
    step: &Step,
    style: Style,
    ctx: &Ctx,
    each: &dyn Fn(&str, Style) -> Vec<Piece>,
) -> Vec<Piece> {
    let texts = &ctx.config.text;
    let mut out = each(step.arg("message").unwrap_or_default().trim(), style);
    let files: Vec<String> = step_files(step);
    if !files.is_empty() {
        let names = files.join("、");
        out.extend(each(
            &texts.attached.replace("{names}", &names),
            theme::dim(),
        ));
    }
    out
}

/// 带的附件：只写文件名。
fn step_files(step: &Step) -> Vec<String> {
    let crate::transcript::StepKind::Tool { parsed, .. } = &step.kind else {
        return Vec::new();
    };
    parsed["attachments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p.as_str())
        .map(|p| {
            std::path::Path::new(p)
                .file_name()
                .map_or(p.to_string(), |n| n.to_string_lossy().into_owned())
        })
        .collect()
}
