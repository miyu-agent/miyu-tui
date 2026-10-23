//! 了结以后正文里留什么（蓝图「确认和提问的抽屉」第 7 条），和抽屉、「确认」页共用的几样写法：
//! 一道的回答写成一句、确认的问题行、碰到的路径。

use super::{Answer, Ask, Decision, Drawer, Outcome, Texts};
use crate::local::home_short;

/// 了结以后留什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// 提问答了：一道一行「问题：回答」，接在时间线里「提问」那一步下面（第 6 条，2026-10-11 项目主人）。
    Answers(Vec<String>),
    /// 不允许：红 `✗` 一行、点开看的全文。理由有几行的，那一行只写第一行接「…」，全文点开看（2026-10-22 项目主人：
    /// 换行写成一串「↵」露在那一行里）；只有一行的全文是空的。
    Denied(String, String),
    /// 什么都不留：允许了、取消了（取消一定顺带打断这一轮，那一步的结果和「已中断」已经说了，2026-10-11 项目主人）。
    Nothing,
}

impl Drawer {
    /// 了结以后留什么。
    pub fn report(&self, outcome: &Outcome, texts: &Texts) -> Report {
        match outcome {
            Outcome::Answered(a) => {
                let lines = a.answers.iter().enumerate().map(|(p, answer)| {
                    let said = if empty(answer) {
                        texts.unanswered.clone()
                    } else {
                        said(answer)
                    };
                    let mut line = texts
                        .answered_line
                        .replace("{label}", &self.label(p))
                        .replace("{answer}", &said);
                    if let Some(notes) = &answer.notes {
                        line.push_str(&texts.notes_suffix.replace("{notes}", &inline(notes)));
                    }
                    line
                });
                Report::Answers(lines.collect())
            }
            Outcome::Cancelled => Report::Nothing,
            Outcome::Decided(d) if d.decision != Decision::Deny => Report::Nothing,
            Outcome::Decided(d) => match with_images(d.reason.as_deref(), &d.files).as_deref() {
                Some(reason) if !reason.is_empty() => {
                    let mut lines = reason.lines();
                    let first = lines.next().unwrap_or_default().trim_end();
                    let more = lines.next().is_some();
                    let head = if more {
                        format!("{first} {}", crate::sheet::ellipsis())
                    } else {
                        first.to_string()
                    };
                    let full = if more {
                        reason.to_string()
                    } else {
                        String::new()
                    };
                    Report::Denied(texts.denied_with.replace("{reason}", &head), full)
                }
                _ => Report::Denied(texts.decisions[2].clone(), String::new()),
            },
        }
    }

    /// 确认的问题行：「要写 1 个文件」这类，照 `access` 找（第 3 条）。
    pub fn title(&self, texts: &Texts) -> String {
        let Ask::Approval(a) = &self.ask else {
            return String::new();
        };
        // 跑命令的：她写的短标题当问题行（2026-10-07 项目主人：不写「要用 shell」）。
        if let Some((Some(title), _)) = &self.command {
            return title.clone();
        }
        // 装、卸软件包，打开扩展的：「安装 weather 1.2.0」这类（`package.rs`）。
        if let Some(plan) = self.package_plan(texts) {
            return plan.title;
        }
        let detail = a.detail.clone().unwrap_or_default();
        let template = texts.access.get(&a.access).unwrap_or(&texts.access_other);
        template
            .replace("{count}", &detail.paths.len().to_string())
            .replace("{tool}", detail.tool.as_deref().unwrap_or(&a.access))
    }

    /// 装、卸软件包，打开扩展的确认写成的样子（核心 F-10 下，`package.rs`）；别的确认、提问没有。
    pub fn package_plan(&self, texts: &Texts) -> Option<super::Plan> {
        let Ask::Approval(a) = &self.ask else {
            return None;
        };
        super::package::plan(a.detail.as_ref()?, &texts.package)
    }

    /// 确认碰到的路径：家目录写 `~`，工作区外的、别的程序的私密目录（核心 5-13）、受保护的（核心 5-17 下上）带上标记。
    pub fn paths(&self, texts: &Texts) -> Vec<(String, Option<String>)> {
        let Ask::Approval(a) = &self.ask else {
            return Vec::new();
        };
        a.detail
            .iter()
            .flat_map(|d| &d.paths)
            .map(|p| {
                let mark = match p.zone.as_deref() {
                    Some("outside") => Some(texts.outside.clone()),
                    Some("private") => Some(texts.private.clone()),
                    Some("read_only") => Some(texts.protected.clone()),
                    _ => None,
                };
                (home_short(&p.path), mark)
            })
            .collect()
    }
}

/// 什么都没答（补充不算）。
/// 拒绝的理由接上贴的图的块（`[图片 1]`），都没有的是 `None`。
fn with_images(reason: Option<&str>, files: &[super::Pasted]) -> Option<String> {
    let mut text = reason.map(str::trim).unwrap_or_default().to_string();
    for file in files {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(&file.label);
    }
    (!text.is_empty()).then_some(text)
}

fn empty(answer: &Answer) -> bool {
    answer.picked.is_empty() && answer.text.is_none() && answer.files.is_empty()
}

/// 一道的回答写成一句：选了的用「、」接起来，再接自己写的话。
pub fn said(answer: &Answer) -> String {
    let mut parts = answer.picked.clone();
    parts.extend(answer.text.as_deref().map(inline));
    let mut said = parts.join("、");
    // 贴的图接在后面写块上的字（`[图片 1]`）。
    for file in &answer.files {
        if !said.is_empty() {
            said.push(' ');
        }
        said.push_str(&file.label);
    }
    said
}

/// 写成一行：换行写成 `↵`（照旧版；记号照主题的 `marks.enter`）。
pub fn inline(text: &str) -> String {
    text.trim()
        .replace(['\r', '\n'], &crate::sheet::mark(|m| &m.enter))
}
