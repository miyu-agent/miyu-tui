//! 新几页的按键和回应（蓝图「配置页」第 5、33 到 36 条）：主菜单上下挪、进页；一页里上下挪、`Enter` 照控件改、`d`
//! 恢复默认；人格页 `Enter` 看详情；选择窗、编辑窗、详情窗；`config.schema`、`persona.list`、`persona.get` 回来了记下。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::choose::{Choose, LineEdit, PersonaView, PresetView, Pressed};
use super::packages::PACKAGES;
use super::schema::{Row, Schema};
use super::{Entry, Section, choices, current, is_named, personas, presets};
use crate::core::Refusal;
use crate::settings::popup::{Confirm, Delete, Popup};
use crate::settings::{Outcome, Settings, Texts, Tone, Waiting};

impl Settings {
    /// 重读的时候顺手要清单和人格（不算进「供应商和模型」那三样，没回来也不挡那一页）。
    pub(in crate::settings) fn load_more(&mut self) {
        self.ask("config.schema", json!({}), Waiting::Schema);
        self.ask("persona.list", json!({}), Waiting::Personas);
        self.ask("preset.list", json!({}), Waiting::Presets);
        self.ask("package.list", json!({}), Waiting::Mascots);
    }

    /// 清单、人格、人格详情回来了：记下；核心不认的（旧核心）当没有，菜单里不列那几页。
    pub(in crate::settings) fn more_answer(
        &mut self,
        waiting: Waiting,
        result: Result<Value, Refusal>,
        texts: &super::Texts,
    ) {
        match (waiting, result) {
            (Waiting::Schema, Ok(got)) => {
                let mut schema = Schema::read(&got);
                schema.split_own(&texts.own_groups, &texts.appearance);
                self.more.schema = Some(schema);
            }
            (Waiting::Personas, Ok(got)) => self.more.personas = Some(personas::read(&got)),
            (Waiting::Presets, Ok(got)) => {
                self.more.presets = Some(crate::core::read_presets(&got));
            }
            (Waiting::Mascots, Ok(got)) => {
                self.more.summaries = super::packages::summaries(&got);
                self.more.mascots = Some(super::mascots::read(&got));
            }
            (Waiting::PresetDetail(id), result) => {
                if let Some(Popup::Preset(view)) = &mut self.popup
                    && view.id == id
                {
                    match result {
                        Ok(got) => view.detail = Some(presets::detail(&got)),
                        Err(refusal) => view.error = Some(refusal.message),
                    }
                }
            }
            (Waiting::PersonaDetail(id), result) => {
                if let Some(Popup::Persona(view)) = &mut self.popup
                    && view.id == id
                {
                    match result {
                        Ok(got) => view.detail = Some(personas::detail(&got)),
                        Err(refusal) => view.error = Some(refusal.message),
                    }
                }
            }
            _ => {}
        }
    }

    /// 主菜单的按键。
    pub(in crate::settings) fn menu_key(&mut self, key: KeyEvent) -> Outcome {
        let count = self.more.entries().len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.more.menu_at = (self.more.menu_at + 1).min(count.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.more.menu_at = self.more.menu_at.saturating_sub(1)
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                self.on_menu = false;
                self.status = None;
                self.more.section = match self.more.selected() {
                    Entry::Models => None,
                    // 「软件包」先列包，进去一包一页（第 43a 条）。
                    Entry::Page(id) if id == PACKAGES => Some(Section::Packages(0)),
                    Entry::Page(id) => Some(Section::Page(id, 0)),
                    Entry::Personas => Some(Section::Personas(0)),
                    Entry::Presets => Some(Section::Presets(0)),
                };
            }
            KeyCode::Esc | KeyCode::Char('q') => return self.back(),
            _ => {}
        }
        Outcome::Stay
    }

    /// 进了新几页里的一页时的按键。
    pub(in crate::settings) fn section_key(&mut self, key: KeyEvent, texts: &Texts) -> Outcome {
        let Some(section) = self.more.section.clone() else {
            return Outcome::Stay;
        };
        self.status = self.status.take().filter(|(_, t)| *t == Tone::Busy);
        let count = self.section_count(&section);
        let (at, rebuild): (usize, fn(usize, &Section) -> Section) = match &section {
            Section::Page(_, at) => (*at, |n, s| match s {
                Section::Page(id, _) => Section::Page(id.clone(), n),
                other => other.clone(),
            }),
            Section::Personas(at) => (*at, |n, _| Section::Personas(n)),
            Section::Presets(at) => (*at, |n, _| Section::Presets(n)),
            Section::Packages(at) => (*at, |n, _| Section::Packages(n)),
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q' | 'h') | KeyCode::Left => self.leave_section(&section),
            KeyCode::Char('j') | KeyCode::Down => {
                let n = (at + 1).min(count.saturating_sub(1));
                self.more.section = Some(rebuild(n, &section));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.more.section = Some(rebuild(at.saturating_sub(1), &section));
            }
            KeyCode::Enter => match &section {
                Section::Packages(at) => self.open_package(*at),
                Section::Page(id, at) => self.change(id, *at, texts),
                Section::Personas(at) => self.open_persona(*at),
                Section::Presets(at) => self.open_preset(*at),
            },
            KeyCode::Char('d') => match &section {
                Section::Page(id, at) => self.ask_unset(id, *at, texts),
                // 预设页 `d`：删选中的那个的个人那一层，先问（第 39 条）。
                Section::Presets(at) => {
                    let id = self
                        .more
                        .presets
                        .as_ref()
                        .and_then(|l| l.get(*at))
                        .map(|p| p.id.clone());
                    if let Some(id) = id {
                        self.ask_delete_preset(&id, texts);
                    }
                }
                // 人格页 `d`：同预设（第 40 条）。
                Section::Personas(at) => {
                    let id = self
                        .more
                        .personas
                        .as_ref()
                        .and_then(|l| l.get(*at))
                        .map(|p| p.id.clone());
                    if let Some(id) = id {
                        self.ask_delete_persona(&id, texts);
                    }
                }
                Section::Packages(_) => {}
            },
            // 预设页、人格页 `a`：新建（第 37、39 条，同旧版）。
            KeyCode::Char('a') if matches!(section, Section::Presets(_)) => self.new_preset(texts),
            KeyCode::Char('a') if matches!(section, Section::Personas(_)) => {
                self.new_persona(texts)
            }
            _ => {}
        }
        Outcome::Stay
    }

    /// 一页里有几行能选（项、人格）。
    fn section_count(&self, section: &Section) -> usize {
        match section {
            Section::Page(id, _) => self.page_items(id).len(),
            Section::Personas(_) => self.more.personas.as_ref().map_or(0, Vec::len),
            Section::Presets(_) => self.more.presets.as_ref().map_or(0, Vec::len),
            Section::Packages(_) => self.more.packages().len(),
        }
    }

    /// 从一页退出来：包自己那一页回到包的列表、停在它上面；别的回主菜单。
    fn leave_section(&mut self, section: &Section) {
        let back = match section {
            Section::Page(page, _) => super::packages::package_of(page).map(|id| {
                let at = self.more.packages().iter().position(|p| p.id == id);
                Section::Packages(at.unwrap_or(0))
            }),
            _ => None,
        };
        self.on_menu = back.is_none();
        self.more.section = back;
    }

    /// 包的列表里 `Enter`：进这个包自己那一页。
    fn open_package(&mut self, at: usize) {
        if let Some(listed) = self.more.packages().get(at) {
            let page = super::packages::page_of(&listed.id);
            self.more.section = Some(Section::Page(page, 0));
        }
    }

    /// 一页里的项，照画的先后（第几项）。
    pub fn page_items(&self, page: &str) -> Vec<usize> {
        self.more
            .rows(page)
            .into_iter()
            .filter_map(|r| match r {
                Row::Item(i) => Some(i),
                Row::Group(_) => None,
            })
            .collect()
    }

    /// `Enter` 改一项：两层都写不了的、终端还不能改的弹一句；下拉开选择窗，开关当场切，数、字开编辑窗。只有系统配置这一层
    /// 的写系统配置（第 33 条）。
    fn change(&mut self, page: &str, at: usize, texts: &Texts) {
        let (Some(schema), Some(data)) = (self.more.schema.as_ref(), self.data.as_ref()) else {
            return;
        };
        let Some(item) = self
            .page_items(page)
            .get(at)
            .and_then(|&i| schema.items.get(i))
            .cloned()
        else {
            return;
        };
        let words = &texts.more;
        if !item.writable() {
            self.say(words.no_editor.clone(), Tone::Note);
            return;
        }
        // 主色：候选的色块列表，不开编辑窗（2026-10-14 项目主人定）。
        if item.key == crate::theme::choice::SEED_KEY {
            let choose = self.more.seed_choose(&item.name, &words.default_mark);
            self.popup = Some(Popup::Choose(choose));
            return;
        }
        let (value, _) = current(&item, data);
        let value = value.clone();
        // 默认人格不管清单写什么控件（核心写的是填名字），都照 `persona.list` 开列表（2026-10-07 项目主人报）。
        let control = if is_named(&item.key) {
            "select"
        } else {
            item.control.as_str()
        };
        if super::secret::is_secret(&item) {
            return self.open_secret(&item);
        }
        match control {
            "select" => {
                let list = choices(&item, self.more.named(&item.key), &texts.more);
                let list = self.more.offered(&item.key, list);
                self.popup = Some(Popup::Choose(Choose::open(
                    &item.key, &item.name, list, &value,
                )));
            }
            "toggle" => {
                let on = value.as_bool().unwrap_or(false);
                self.draft.system = item.system_only();
                self.draft.set(&item.key, Value::Bool(!on), data);
                self.refresh();
                self.save(false, texts);
            }
            "combo" => self.open_combo(&item, &value, texts),
            "textarea" => self.open_textarea(&item, &value),
            "list" if super::list::editable(&item) => self.open_list(&item, &value),
            "list" => self.list_not_editable(texts),
            "number" | "text" => {
                let number = item.control == "number";
                self.popup = Some(Popup::Line(LineEdit::open(
                    &item.key, &item.name, number, &value,
                )));
            }
            _ => self.say(words.no_editor.clone(), Tone::Note),
        }
    }

    /// `d`：个人设置里写了这一项的（只有系统配置这一层的：系统配置里写了的），问一句再去掉，回到下面一层的值。
    fn ask_unset(&mut self, page: &str, at: usize, texts: &Texts) {
        let (Some(schema), Some(data)) = (self.more.schema.as_ref(), self.data.as_ref()) else {
            return;
        };
        let Some(item) = self
            .page_items(page)
            .get(at)
            .and_then(|&i| schema.items.get(i))
            .cloned()
        else {
            return;
        };
        let words = &texts.more;
        // 跟随头像的主色存在人格上：恢复默认是清掉人格的主题色。
        if let Some(persona) = self
            .more
            .seed_persona
            .clone()
            .filter(|_| item.key == crate::theme::choice::SEED_KEY)
        {
            self.popup = Some(Popup::Confirm(Confirm {
                title: words.unset[0].replace("{name}", &item.name),
                lines: vec![(words.seed_unset.clone(), true)],
                buttons: vec![
                    (words.unset[2].clone(), true),
                    (words.unset[3].clone(), false),
                ],
                sel: 1,
                ask: Delete::PersonaSeed(persona),
            }));
            return;
        }
        let system = item.system_only();
        let written = if system { &data.system } else { &data.personal };
        if !written.contains_key(&item.key) {
            self.say(words.nothing_to_unset.clone(), Tone::Note);
            return;
        }
        let below = if !system && data.system.contains_key(&item.key) {
            "system"
        } else {
            "default"
        };
        let from = words.origins.get(below).cloned().unwrap_or_default();
        self.popup = Some(Popup::Confirm(Confirm {
            title: words.unset[0].replace("{name}", &item.name),
            lines: vec![(words.unset[1].replace("{from}", &from), true)],
            buttons: vec![
                (words.unset[2].clone(), true),
                (words.unset[3].clone(), false),
            ],
            sel: 1,
            ask: Delete::Setting(item.key),
        }));
    }

    /// 预设页 `Enter`：开详情窗，去读 `preset.get`。
    fn open_preset(&mut self, at: usize) {
        let Some(id) = self
            .more
            .presets
            .as_ref()
            .and_then(|l| l.get(at))
            .map(|p| p.id.clone())
        else {
            return;
        };
        self.ask(
            "preset.get",
            json!({"preset": id}),
            Waiting::PresetDetail(id.clone()),
        );
        self.popup = Some(Popup::Preset(PresetView {
            id,
            detail: None,
            error: None,
            cursor: 0,
        }));
    }

    /// 人格页 `Enter`：开详情窗，去读 `persona.get`。
    fn open_persona(&mut self, at: usize) {
        let Some(id) = self
            .more
            .personas
            .as_ref()
            .and_then(|l| l.get(at))
            .map(|p| p.id.clone())
        else {
            return;
        };
        self.ask(
            "persona.get",
            json!({"persona": id}),
            Waiting::PersonaDetail(id.clone()),
        );
        self.popup = Some(Popup::Persona(PersonaView {
            id,
            detail: None,
            error: None,
            cursor: 0,
        }));
    }

    /// 选择窗、编辑窗、详情窗的按键。
    pub(in crate::settings) fn more_popup_key(
        &mut self,
        popup: Popup,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match popup {
            Popup::Choose(mut choose) => match choose.press(key) {
                Pressed::Picked(value) => self.write_to(choose.target, &choose.key, value, texts),
                Pressed::Cancelled => self.edit_cancel(choose.target),
                Pressed::Moved | Pressed::Ignored => self.popup = Some(Popup::Choose(choose)),
            },
            Popup::Line(mut edit) => {
                match key.code {
                    KeyCode::Esc => return self.edit_cancel(edit.target),
                    KeyCode::Enter => {
                        // 列表的一条（第 47 条）：清空了是去掉这一条，整数的写了别的照整数说。
                        let item = matches!(edit.target, super::target::Target::ListItem(..));
                        let blank = edit.editor.text().trim().is_empty();
                        match edit.value() {
                            _ if item && blank => {
                                let value = Value::String(String::new());
                                return self.write_to(edit.target, &edit.key, value, texts);
                            }
                            Some(value) => {
                                return self.write_to(edit.target, &edit.key, value, texts);
                            }
                            None if item => edit.error = Some(texts.more.list.whole.clone()),
                            None => edit.error = Some(texts.more.bad_number.clone()),
                        }
                    }
                    _ => {}
                }
                let editor = &mut edit.editor;
                match key.code {
                    KeyCode::Backspace => editor.backspace(),
                    KeyCode::Delete => editor.delete(),
                    KeyCode::Left => editor.left(false),
                    KeyCode::Right => editor.right(false),
                    KeyCode::Home => editor.move_to(0, false),
                    KeyCode::End => editor.move_to(editor.text().len(), false),
                    KeyCode::Char(c) => editor.insert(&c.to_string()),
                    _ => {}
                }
                self.popup = Some(Popup::Line(edit));
            }
            Popup::Sheet(edit) => self.sheet_key(edit, key, texts),
            Popup::List(edit) => self.list_key(edit, key, texts),
            // 人格的窗（第 36 条）、示范对话列表和两格窗（第 41 条）。
            Popup::Persona(view) => self.persona_key(view, key, texts),
            Popup::Dialogs(view) => self.dialogs_key(view, key),
            Popup::Pair(edit) => self.pair_key(edit, key, texts),
            // 能改的预设详情窗（第 39 条）。
            Popup::Preset(view) => self.preset_key(view, key, texts),
            other => self.popup = Some(other),
        }
    }

    /// 选择窗、编辑窗选好了：改预设的照它办（`preset_edit.rs`），别的写个人设置。
    fn write_to(&mut self, target: super::target::Target, key: &str, value: Value, texts: &Texts) {
        // 跟随头像的主色写在人格上（`persona.set` 的 `seed`）。
        if let super::target::Target::PersonaSeed(persona) = &target {
            let params = json!({"persona": persona, "seed": value});
            self.ask("persona.set", params, Waiting::PersonaSeed);
            return;
        }
        if !self.model_or_write(&target, key, &value)
            && !self.list_write(&target, &value, texts)
            && !self.secret_write(&target, key, &value, texts)
            && !self.combo_write(&target, key, &value)
            && !self.preset_write(target.clone(), key, value.clone())
            && !self.avatar_write(target.clone(), &value)
            && !self.persona_write(target, key, value.clone())
        {
            self.write_setting(key, value, texts);
        }
    }

    /// 写一项进个人设置（只有系统配置这一层的写系统配置），当场存（关窗；没存成的原因写在状态行）。
    pub(in crate::settings) fn write_setting(&mut self, key: &str, value: Value, texts: &Texts) {
        self.draft.system = self.system_only(key);
        let Some(data) = self.data.as_ref() else {
            return;
        };
        // 选了「无人格」这类空的：从个人设置里去掉这一项。
        if value.is_null() {
            self.draft.unset(key, data);
        } else {
            self.draft.set(key, value, data);
        }
        self.refresh();
        self.save(false, texts);
    }

    /// 清单里这一项只有系统配置这一层（「配置页」第 33 条）。
    pub(in crate::settings) fn system_only(&self, key: &str) -> bool {
        self.more
            .schema
            .as_ref()
            .and_then(|s| s.items.iter().find(|i| i.key == key))
            .is_some_and(super::schema::Item::system_only)
    }
}
