//! 模型池的窗（蓝图 `tui.md`「配置页」第 19 条）：名称、调用方式、子代理能选，下面是成员。成员先列已选的、再列没选的
//! （2026-11-14 主人报：要取消一个成员得在整张模型列表里找它），两段之间各一行暗字；先后开窗时排定，窗开着时勾、取消不挪行。

use super::{Field, Form, Row, Target, Val, values};
use crate::settings::data::Data;

/// 编辑一个池（`None` 是新建）：已选的照池里写的先后在前（已经不存在的照旧在它原来的位置、红字），没选的模型照供应商、模型的先后在后。
pub fn pool(view: &Data, name: Option<&str>) -> Form {
    let pool = name.and_then(|n| view.pool(n));
    let members = pool.map(|p| p.members.clone()).unwrap_or_default();
    // 照先后：主力加备用（核心 Q-4上）。
    let strategies = ["rotate", "pin", "ordered"].map(str::to_string).to_vec();
    let at = pool
        .and_then(|p| p.strategy.as_deref())
        .and_then(|s| strategies.iter().position(|x| x == s))
        .or(Some(0));
    let mut rows = vec![
        match name {
            Some(n) => Row::new(Field::Pool, Val::Fixed(n.to_string())),
            None => Row::text(Field::Pool, ""),
        },
        Row::new(
            Field::Strategy,
            Val::Choice {
                options: strategies,
                at,
            },
        ),
        values::subagent(pool.is_some_and(|p| p.subagent)),
        Row::new(Field::Members, Val::Section),
    ];
    let others: Vec<_> = view
        .providers
        .iter()
        .flat_map(|p| &p.models)
        .filter(|m| !members.contains(&m.reference))
        .collect();
    if !members.is_empty() {
        rows.push(Row::new(Field::Chosen, Val::Section));
        for reference in &members {
            let model = view.model(reference).map(|(_, m)| m);
            rows.push(member(
                reference,
                model.is_some_and(|m| m.sees()),
                model.is_none(),
            ));
        }
        if !others.is_empty() {
            rows.push(Row::new(Field::Others, Val::Section));
        }
    }
    for m in others {
        rows.push(member(&m.reference, m.sees(), false));
    }
    Form::new(Target::Pool(name.map(str::to_string)), rows, members)
}

fn member(reference: &str, sees: bool, gone: bool) -> Row {
    Row::new(
        Field::Member,
        Val::Member {
            reference: reference.to_string(),
            sees,
            gone,
        },
    )
}
