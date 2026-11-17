//! 选人格、预设的选择窗里按 `e` 编辑（蓝图「配置页」第 33 条的子条目，2026-11-17 项目主人验收第 2 条：`/config` 里默认人格、默认
//! 预设没有像 `/new` 的框那样能跳去编辑；同「新会话：人格、工作区」第 2b 条）：光标那一行直接开它的详情窗，不算选；详情窗关了
//! 回到选择窗，光标还在那一行，名字照改过的写。「无人格」那一行、写错了读不出来的不开，说一句不能编辑。

use serde_json::Value;

use super::choose::Choose;
use super::target::Target;
use super::values::{choices, current};
use super::{DEFAULT_PERSONA, DEFAULT_PRESET};
use crate::settings::in_use::Item;
use crate::settings::{Settings, Texts, Tone};

impl super::More {
    /// 这个选择窗选的是人格还是预设（默认人格、默认预设，和别的选人格、预设的项）；别的是 `None`。
    pub fn edits(&self, choose: &Choose) -> Option<Item> {
        if choose.target != Target::Setting {
            return None;
        }
        let item = self
            .schema
            .as_ref()?
            .items
            .iter()
            .find(|i| i.key == choose.key)?;
        match (item.key.as_str(), item.kind.as_str()) {
            (DEFAULT_PERSONA, _) | (_, "persona") => Some(Item::Persona),
            (DEFAULT_PRESET, _) | (_, "preset") => Some(Item::Preset),
            _ => None,
        }
    }
}

impl Settings {
    /// 选择窗上按了 `e`：选人格、预设的开光标那一行的详情窗，记下回哪个选择窗；交回 `false` 的不是这种窗，照常按。
    pub(in crate::settings) fn edit_from_choose(&mut self, choose: &Choose, texts: &Texts) -> bool {
        let Some(kind) = self.more.edits(choose) else {
            return false;
        };
        let row = choose.choices.get(choose.sel).filter(|c| c.off.is_none());
        let Some(id) = row
            .and_then(|c| c.value.as_str())
            .filter(|id| !id.is_empty())
        else {
            self.popup = Some(crate::settings::popup::Popup::Choose(choose.clone()));
            self.say(texts.more.cant_edit.clone(), Tone::Note);
            return true;
        };
        self.more.back_to = Some((choose.key.clone(), Value::from(id)));
        match kind {
            Item::Persona => self.show_persona(id.to_string()),
            Item::Preset => self.show_preset(id.to_string()),
        }
        true
    }

    /// 从选择窗进去的详情窗关了：重开那个选择窗，光标停回那一行（删掉了的停在现在的值上）。
    pub(in crate::settings) fn back_to_choose(&mut self, texts: &Texts) {
        if self.more.back_to.is_none() || !self.detail_closed() {
            return;
        }
        let Some((key, at)) = self.more.back_to.take() else {
            return;
        };
        let item = self
            .more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
            .cloned();
        let (Some(item), Some(data)) = (item, self.data.as_ref()) else {
            return;
        };
        let value = current(&item, data).0.clone();
        let list = choices(&item, self.more.named_item(&item), &texts.more);
        let list = self.more.offered(&item.key, list);
        let mut choose = Choose::open(&item.key, &item.name, list, &value);
        if let Some(i) = choose
            .choices
            .iter()
            .position(|c| c.value == at && c.off.is_none())
        {
            choose.sel = i;
        }
        self.popup = Some(crate::settings::popup::Popup::Choose(choose));
    }
}

#[cfg(test)]
mod tests;
