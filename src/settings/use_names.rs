//! 默认模型页那几项的名字（蓝图「配置页」第 21 条）：多模态、语义模型两项照核心 `config.schema` 里那一项的 `name`，照连接的
//! 语言（ACC-1121-#3：两个头都照核心的字表，核心 FX-9 把 `models.vision` 改名「多模态模型」）；旧核心的清单里没有这一项、
//! 清单还没读回来的照终端自己的字。默认文本模型照旧是终端的字。

use super::nav::{USES, Use};
use super::{Settings, Texts};

impl Settings {
    /// 默认模型页第 `at` 项的名字。
    pub fn use_name(&self, at: usize, texts: &Texts) -> String {
        let key = match USES.get(at) {
            Some(Use::Vision) => Some("models.vision"),
            Some(Use::Embedding) => Some(super::nav::EMBEDDING),
            _ => None,
        };
        let schema = self.more.schema.as_ref();
        let item = key.and_then(|key| schema?.items.iter().find(|i| i.key == key));
        match item.filter(|i| !i.name.trim().is_empty()) {
            Some(item) => item.name.clone(),
            None => texts.uses[at].clone(),
        }
    }
}
