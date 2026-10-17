//! 点开一条留言（`send_message`，蓝图「时间线」第 8 条，2026-10-18 项目主人：原来只有「Message sent to 35106d22.」）：先一行
//! 暗的「→ 发给谁」——别的会话写它的标题和短编号，子代理写它的名字和任务编号，父会话写「父会话」——下面接完整的消息，带了
//! 附件的再一行写文件名。

use ratatui::style::Style;

use super::super::rows::Ctx;
use super::step::Piece;
use crate::session_list::{looks_like_session, short};
use crate::theme;
use crate::transcript::Step;

/// 排成一行行；`each` 把一段字照宽度折成行。
pub fn pieces(
    step: &Step,
    style: Style,
    ctx: &Ctx,
    each: &dyn Fn(&str, Style) -> Vec<Piece>,
) -> Vec<Piece> {
    let texts = &ctx.config.text;
    let to = step.arg("to").unwrap_or_default().trim();
    let target = if to == "parent" {
        texts.to_parent.clone()
    } else if looks_like_session(to) {
        let id = texts.on_session.replace("{id}", &short(to));
        let title = ctx
            .sessions
            .iter()
            .find(|s| s.session == to)
            .and_then(|s| s.title.as_deref())
            .filter(|t| !t.trim().is_empty());
        title.map_or(id.clone(), |t| format!("{} · {id}", t.trim()))
    } else {
        let name = ctx
            .board
            .and_then(|b| b.jobs.iter().find(|j| j.job == to))
            .map(|j| j.title.clone());
        name.map_or(to.to_string(), |n| {
            format!("{} · {to}", texts.drawer.agent.replace("{name}", &n))
        })
    };
    let mut out = each(&format!("→ {target}"), theme::dim());
    out.extend(each(step.arg("message").unwrap_or_default().trim(), style));
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
