//! 这个会话改过的文件（蓝图 `tui.md`「文件修改」，发版前 check 9，2026-11-13 主人 1113 初验）：照正文里编辑、写入成了的那几步
//! 按文件归在一起（同网页右侧栏的「文件修改」卡：只算编辑、写入，命令行改的不算；先改的在前），一个文件记下改它的每一步
//! （调用编号、照参数比的差异）和加减的行数。侧边栏那一块、点开以后的那一页都照这一份。

use std::rc::Rc;

use crate::config::{Timeline, ToolKind};
use crate::core::ToolStatus;
use crate::diff::Diff;
use crate::transcript::{StepKind, ToolState, Transcript};

/// 改一个文件的一步。
#[derive(Debug, Clone)]
pub struct Change {
    /// 调用编号：点开时照它问核心完整的差异（`view.detail`）；还没拿到的是 `None`。
    pub call: Option<String>,
    /// 照参数比的差异：核心的完整差异回来以前先画它（编辑的片段不带行号）。
    pub diff: Rc<Diff>,
}

/// 改过的一个文件。
#[derive(Debug, Clone)]
pub struct Changed {
    /// 参数里写的路径（`file_path`）。
    pub path: String,
    /// 改它的几步，照先后。
    pub changes: Vec<Change>,
    /// 一共加了几行。
    pub added: usize,
    /// 一共删了几行。
    pub removed: usize,
}

/// 这个会话改过的文件：撤销掉的那几轮（藏起来的条目）不算；没成的（出错、被拒、打断）、还没结果的不算。
pub fn of(t: &Transcript, timeline: &Timeline) -> Vec<Changed> {
    let mut out: Vec<Changed> = Vec::new();
    let steps = t
        .entries
        .iter()
        .filter(|e| !e.hidden)
        .filter_map(|e| e.segment.as_ref())
        .flat_map(|s| s.steps.iter());
    for step in steps {
        let StepKind::Tool {
            name,
            parsed,
            state,
            ..
        } = &step.kind
        else {
            continue;
        };
        let done = matches!(state, ToolState::Done(ToolStatus::Ok | ToolStatus::Other));
        if !done || timeline.kinds.get(name) != Some(&ToolKind::Edit) {
            continue;
        }
        let Some(path) = parsed["file_path"].as_str().filter(|p| !p.is_empty()) else {
            continue;
        };
        let Some(diff) = crate::diff::from_args(parsed) else {
            continue;
        };
        let change = Change {
            call: step.call_id.clone(),
            diff: diff.clone(),
        };
        match out.iter_mut().find(|c| c.path == path) {
            Some(file) => {
                file.added += diff.added;
                file.removed += diff.removed;
                file.changes.push(change);
            }
            None => out.push(Changed {
                path: path.to_string(),
                added: diff.added,
                removed: diff.removed,
                changes: vec![change],
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests;
