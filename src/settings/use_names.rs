//! 默认模型页那几项的名字、用途一句（蓝图「配置页」第 21 条）：照核心 `config.schema` 里那一项的 `name`、`description`，照
//! 连接的语言（ACC-1121-#3：两个头都照核心的字表，核心 FX-9 把 `models.vision` 改名「多模态模型」；对话那一行也照核心，网页
//! 同）。名字三行都照核心；用途一句只有对话那一行照核心，多模态、语义模型两行的用途是主人定的说法，照终端自己的字。旧核心的
//! 清单里没有这一项、清单还没读回来、核心没写的照终端自己的字。

use super::nav::{EMBEDDING, USES, Use};
use super::pages::schema::Item;
use super::{Settings, Texts};

impl Settings {
    /// 默认模型页第 `at` 项的名字。
    pub fn use_name(&self, at: usize, texts: &Texts) -> String {
        let named = self.use_item(at).filter(|i| !i.name.trim().is_empty());
        named.map_or_else(|| texts.uses[at].clone(), |i| i.name.clone())
    }

    /// 默认模型页第 `at` 项的用途一句。
    pub fn use_note(&self, at: usize, texts: &Texts) -> String {
        let said = (USES.get(at) == Some(&Use::Chat))
            .then(|| self.use_item(at)?.description.clone())
            .flatten();
        said.unwrap_or_else(|| texts.use_notes[at].clone())
    }

    /// 核心清单里第 `at` 项那一条。
    fn use_item(&self, at: usize) -> Option<&Item> {
        let key = match USES.get(at)? {
            Use::Chat => "models.chat",
            Use::Vision => "models.vision",
            Use::Embedding => EMBEDDING,
        };
        let schema = self.more.schema.as_ref()?;
        schema.items.iter().find(|i| i.key == key)
    }
}
