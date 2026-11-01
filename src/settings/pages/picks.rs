//! 选 Miyu 自己的东西（蓝图「配置页」第 33 条子规则，核心设计 53 第三档 PC-3）：`persona`、`knowledge` 类型的项照人格、
//! 知识库列表开选择窗（列表在 [`super::More::named_item`]），`reference` 开选默认模型的那个窗、选了写回这一项。

use serde_json::Value;

use super::target::Target;
use crate::core::Persona;
use crate::settings::Settings;
use crate::settings::nav::Use;
use crate::settings::popup::{Pick, Popup};

/// 读 `kb.list`：一个知识库一项，名字照它的名字（没有的照编号），说明照它的说明。
pub fn bases(got: &Value) -> Vec<Persona> {
    got["bases"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| {
            let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_string);
            Some(Persona {
                id: b["id"].as_str()?.to_string(),
                name: text(&b["name"]),
                summary: text(&b["description"]),
                ..Persona::default()
            })
        })
        .collect()
}

impl Settings {
    /// 开选模型的窗：停在现在的值上，选了照 `target` 写回 `key`，`Esc` 回到来处。
    pub(in crate::settings) fn open_reference(&mut self, target: Target, key: &str, value: &Value) {
        let pick = Pick::open(Use::Chat, &self.view)
            .at(value.as_str().unwrap_or_default(), &self.view)
            .to(target, key);
        self.popup = Some(Popup::Pick(pick));
    }
}
