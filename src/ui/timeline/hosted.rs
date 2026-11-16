//! 一步工具的名字那一截、对象：供应商侧工具（模型在供应商那边自己跑的，`Step::hosted`）照核心 `view.md`「供应商侧工具」写
//! ——对上的核心工具（`timeline.json` 的 `hosted`）的显示名和对象，对不上的写原名、没有对象，谁跑的写供应商的名字，接在
//! 名字后面的括号里：「网络搜索（agy）」（2026-11-16 项目主人定）。核心执行的照调用那一刻记下的显示名，结果那一句带了来源
//! （`human.fields.via`，核心 2026-10-19）的同样接在后面：「网络搜索（Exa）」。

use miyu_client::texts::Face;

use super::super::rows::Ctx;
use crate::config::Hosted;
use crate::transcript::{Step, StepKind};

/// 名字那一截，带着经谁做的。不是工具的是空的。
pub fn name(step: &Step, ctx: &Ctx) -> String {
    let StepKind::Tool { name, said, .. } = &step.kind else {
        return String::new();
    };
    let (shown, via) = match &step.hosted {
        Some(provider) => (
            mapped(name, ctx).map_or_else(|| name.clone(), |m| ctx.human.tool_name(&m.core)),
            Some(ctx.human.provider(provider)),
        ),
        None => (
            ctx.human
                .step_name(name, &step.display, ctx.config.language.code()),
            said.as_ref()
                .and_then(|s| s.fields.iter().find(|(f, _)| f.as_str() == "via"))
                .map(|(_, v)| v.as_str()),
        ),
    };
    match via.map(str::trim).filter(|v| !v.is_empty()) {
        Some(via) => ctx
            .config
            .text
            .via
            .replace("{name}", &shown)
            .replace("{via}", via),
        None => shown,
    }
}

/// 对象：供应商侧工具照对上的那一条说的参数（对不上的、没说的没有）；核心执行的照工具自己说的 `subject`（`face`）。
/// 供应商侧工具空着的参数不算对象（同核心）。
pub fn subject<'a>(step: &'a Step, face: Option<&Face>, ctx: &Ctx) -> Option<&'a str> {
    match (&step.hosted, &step.kind) {
        (Some(_), StepKind::Tool { name, .. }) => {
            let key = mapped(name, ctx)?.subject.as_deref()?;
            step.arg(key).filter(|v| !v.is_empty())
        }
        _ => step.arg(face?.subject.as_deref()?),
    }
}

/// 供应商的工具名对上的核心工具。
fn mapped<'a>(name: &str, ctx: &'a Ctx) -> Option<&'a Hosted> {
    ctx.config.timeline.hosted.get(name)
}
