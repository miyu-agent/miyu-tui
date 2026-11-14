//! 记录列表（蓝图「配置页」第 47b 条，`element` 是 `record`，核心设计 53 第二档 PC-2；2026-10-31 项目主人看了样板照推荐
//! 定）：一项是一组字段。`Enter` 开列表窗（同第 47 条：`a` 加、`Enter` 改、`d` 删、`J` `K` 挪，`Esc` 收起时动过的整份
//! 存）；加、改开这一条的表单窗，一个字段一行，`Enter` 照字段的控件改，滑块在行上 `←` `→` 调，`Esc` 回列表。密钥字段填的
//! 明文先留在这一条上（带 [`PLAIN`] 记号，只在终端里），存的时候先存进密钥库、再换成引用（`draft.rs`）。

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::choose::{Choose, LineEdit};
use super::schema::Item;
use super::target::Target;
use super::{choices, secret, slider};
use crate::pairs::{self, ListKey};
use crate::settings::popup::Popup;
use crate::settings::{Settings, Texts};

/// 填了还没存进密钥库的明文：`{PLAIN: "明文"}`，存的时候换成 `{"secret": 名字}`。
pub const PLAIN: &str = "\u{0}plain";

/// 记录列表的字（`text/zh.json` 的 `settings.more.records`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTexts {
    /// 没有标题的一条：`{n}` 从 1 数。
    pub nth: String,
    /// 一条都没有。
    pub empty: String,
    /// 列表窗的按键提示。
    pub hints: Vec<[String; 2]>,
    /// 表单窗的按键提示。
    pub form_hints: Vec<[String; 2]>,
}

/// 开着的列表窗。
#[derive(Debug, Clone, PartialEq)]
pub struct RecordsEdit {
    /// 这一项（键、名字、字段、标题字段）。
    pub item: Box<Item>,
    /// 一条条，照先后。
    pub records: Vec<Value>,
    /// 选中第几条。
    pub sel: usize,
    /// 动过（加、改、删、挪）。
    pub changed: bool,
}

/// 开着的一条的表单窗。
#[derive(Debug, Clone, PartialEq)]
pub struct RecordForm {
    /// 回去的列表窗。
    pub list: RecordsEdit,
    /// 改第几条；`None` 是新加的。
    pub at: Option<usize>,
    /// 这一条的字段。
    pub record: Map<String, Value>,
    /// 选中第几个字段。
    pub sel: usize,
}

/// 一条的标题：`title` 写的那个字段的值；没写 `title`、那个字段空着的写「第 N 项」（`n` 从 1 数）。
pub fn label(item: &Item, record: &Value, n: usize, texts: &RecordTexts) -> String {
    let titled = item
        .title
        .as_ref()
        .and_then(|t| record.get(t))
        .and_then(|v| match v {
            // 记录是包设置里的、外来的字：图格字、标签字符不进画面（发布前第五批审查 T5-3）。
            Value::String(s) if !s.is_empty() => Some(crate::tabs::printable(s).into_owned()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
    titled.unwrap_or_else(|| texts.nth.replace("{n}", &n.to_string()))
}

/// 页里那一行：各条的标题用 `sep`（界面语言的顿号、逗号）连，一条都没有的是「—」（同第 47 条）。不是记录列表的是 `None`。
pub fn summary(item: &Item, value: &Value, texts: &RecordTexts, sep: &str) -> Option<String> {
    if item.element != "record" {
        return None;
    }
    let list = value.as_array().map(Vec::as_slice).unwrap_or_default();
    if list.is_empty() {
        return Some("—".to_string());
    }
    let titles: Vec<String> = list
        .iter()
        .enumerate()
        .map(|(i, r)| label(item, r, i + 1, texts))
        .collect();
    Some(titles.join(sep))
}

/// `<项>[第几条].<字段>` 拆成三段（密钥字段在草稿里照它记，`draft.rs`）。
pub fn record_path(key: &str) -> Option<(String, usize, String)> {
    let open = key.rfind('[')?;
    let (base, rest) = key.split_at(open);
    let (index, field) = rest[1..].split_once("].")?;
    Some((base.to_string(), index.parse().ok()?, field.to_string()))
}

/// 新加的一条：各字段照默认填（没有默认的不写）。
fn fresh(item: &Item) -> Map<String, Value> {
    item.fields
        .iter()
        .filter(|f| !f.default.is_null())
        .map(|f| (f.key.clone(), f.default.clone()))
        .collect()
}

impl Settings {
    /// 开列表窗（`item` 现在的值是 `value`）。
    pub(in crate::settings) fn open_records(&mut self, item: &Item, value: &Value) {
        self.popup = Some(Popup::Records(RecordsEdit {
            item: Box::new(item.clone()),
            records: value.as_array().cloned().unwrap_or_default(),
            sel: 0,
            changed: false,
        }));
    }

    /// 列表窗的一个键：加、改开表单窗，删、挪记下动过，`Esc` 收起时动过的整份存。
    pub(in crate::settings) fn records_key(
        &mut self,
        mut edit: RecordsEdit,
        key: KeyEvent,
        texts: &Texts,
    ) {
        match pairs::list_key(&mut edit.records, &mut edit.sel, key) {
            ListKey::Stay => self.popup = Some(Popup::Records(edit)),
            ListKey::Changed => {
                edit.changed = true;
                self.popup = Some(Popup::Records(edit));
            }
            ListKey::Edit(at) => {
                let record = match at.and_then(|i| edit.records.get(i)) {
                    Some(Value::Object(r)) => r.clone(),
                    _ => fresh(&edit.item),
                };
                self.popup = Some(Popup::Record(RecordForm {
                    list: edit,
                    at,
                    record,
                    sel: 0,
                }));
            }
            ListKey::Close if edit.changed => self.save_records(edit, texts),
            ListKey::Close => self.popup = None,
        }
    }

    /// 表单窗的一个键：`↑` `↓` 挪；滑块的字段 `←` `→` 调；`Enter` 照字段的控件改；`Esc` 回列表，这一条换上。
    pub(in crate::settings) fn record_key(
        &mut self,
        mut form: RecordForm,
        key: KeyEvent,
        texts: &Texts,
    ) {
        let last = form.list.item.fields.len().saturating_sub(1);
        let field = form.list.item.fields.get(form.sel).cloned();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => form.sel = (form.sel + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => form.sel = form.sel.saturating_sub(1),
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h' | 'l') => {
                if let Some(field) = field.filter(|f| f.slide.is_some()) {
                    let up = matches!(key.code, KeyCode::Right | KeyCode::Char('l'));
                    let now = form.record.get(&field.key).cloned().unwrap_or(Value::Null);
                    let now = if now.is_null() {
                        field.default.clone()
                    } else {
                        now
                    };
                    if let Some(next) = slider::nudge(&field, &now, up) {
                        form.record.insert(field.key, next);
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(field) = field {
                    return self.record_field(form, &field, texts);
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => return self.record_done(form),
            _ => {}
        }
        self.popup = Some(Popup::Record(form));
    }

    /// 改一个字段：开关当场切，下拉开选择窗，几行字开多行窗，密钥开遮住的编辑窗，别的开编辑窗（数照数读）。
    fn record_field(&mut self, mut form: RecordForm, field: &Item, texts: &Texts) {
        let now = form
            .record
            .get(&field.key)
            .cloned()
            .unwrap_or_else(|| field.default.clone());
        if field.control == "toggle" {
            let on = now.as_bool().unwrap_or(false);
            form.record.insert(field.key.clone(), Value::Bool(!on));
            self.popup = Some(Popup::Record(form));
            return;
        }
        let secret = secret::is_secret(field);
        let key = field.key.clone();
        let target = Target::RecordField(Box::new(form), key.clone(), secret);
        // 选模型、人格、知识库的字段（第 33 条子规则）：同配置项。
        if field.kind == "reference" {
            return self.open_reference(target, &key, &now);
        }
        let named = self.more.named_item(field).map(<[_]>::to_vec);
        self.popup = Some(if !field.options.is_empty() || named.is_some() {
            let list = choices(field, named.as_deref(), &texts.more);
            let mut choose = Choose::open(&key, &field.name, list, &now);
            choose.target = target;
            Popup::Choose(choose)
        } else if field.control == "textarea" {
            let mut sheet = super::textarea::SheetEdit::open(field, &now);
            sheet.to = Some(target);
            Popup::Sheet(sheet)
        } else {
            let number = matches!(field.control.as_str(), "number" | "slider");
            let shown = if secret { Value::Null } else { now };
            let line = LineEdit::open(&key, &field.name, number, &shown);
            Popup::Line(line.float(field.kind == "float").to(target))
        });
    }

    /// 选择窗、编辑窗写好了：回到表单窗，这个字段换上；密钥字段留明文待存，空的不改。不是表单的字段交回 `false`。
    pub(in crate::settings) fn record_write(&mut self, target: &Target, value: Value) -> bool {
        let Target::RecordField(form, key, secret) = target else {
            return false;
        };
        let mut form = (**form).clone();
        let blank = value.is_null() || value.as_str().is_some_and(str::is_empty);
        match (*secret, blank) {
            (true, true) => {}
            (true, false) => {
                let plain = json!({ PLAIN: value.as_str().unwrap_or_default() });
                form.record.insert(key.clone(), plain);
            }
            (false, true) => {
                form.record.remove(key);
            }
            (false, false) => {
                form.record.insert(key.clone(), value);
            }
        }
        self.popup = Some(Popup::Record(form));
        true
    }

    /// 表单窗 `Esc`：回列表，这一条换上（新加的、一个字段都没填的不加）。
    fn record_done(&mut self, form: RecordForm) {
        let RecordForm {
            mut list,
            at,
            record,
            ..
        } = form;
        let value = Value::Object(record);
        match at {
            Some(i) if i < list.records.len() => {
                if list.records[i] != value {
                    list.records[i] = value;
                    list.changed = true;
                }
            }
            None if value.as_object().is_some_and(|r| r != &fresh(&list.item)) => {
                list.records.push(value);
                list.sel = list.records.len() - 1;
                list.changed = true;
            }
            _ => {}
        }
        self.popup = Some(Popup::Records(list));
    }

    /// 整份存：填了明文的密钥字段先记进草稿（照 `<项>[第几条].<字段>`），存的时候先存进密钥库、再换成引用；别的照写一项。
    fn save_records(&mut self, edit: RecordsEdit, texts: &Texts) {
        let mut records = edit.records;
        let mut plains = Vec::new();
        for (i, record) in records.iter_mut().enumerate() {
            let Some(fields) = record.as_object_mut() else {
                continue;
            };
            for (field, value) in fields.iter_mut() {
                if let Some(text) = value.get(PLAIN).and_then(Value::as_str) {
                    plains.push((format!("{}[{i}].{field}", edit.item.key), text.to_string()));
                    *value = Value::Null;
                }
            }
        }
        self.popup = None;
        let Some(data) = self.data.as_ref() else {
            return;
        };
        self.draft.set(&edit.item.key, Value::Array(records), data);
        for (path, text) in plains {
            self.draft.set_secret_at(&path, text);
        }
        self.refresh();
        self.save(false, texts);
    }
}

#[cfg(test)]
mod tests;
