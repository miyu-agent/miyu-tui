//! 软件包带的供应商（核心 PV-4，蓝图「配置页」第 17 条）：`config.get` 里这一家几项的来源是 `{"layer": "package", "package"}`，
//! 供应商的窗里那几行右边写「来自软件包 X」，接口照包给的那一个显示、不写成「自动」。人改过的那一项写进了系统配置，来源不再是
//! `package`，不标。

use super::{Field, Form, Row, Target, Val, keys};
use crate::settings::data::Data;

/// 来源一格里软件包给的那几行的前缀：后面接包的名字（[`super::Row::source`]）。
pub const PACKAGE: &str = "package:";

/// 包能给、窗里有的那几行：行和配置里的那一格。
const GIVEN: [(Field, &str); 3] = [
    (Field::Name, "name"),
    (Field::BaseUrl, "base_url"),
    (Field::Driver, "driver"),
];

impl Form {
    /// 一家已有的供应商的窗：照 `data` 里包给的项标来源，包的名字照 `name`（编号到名字，取不到的照编号）。不是供应商的窗、
    /// 没有包给的项的原样交回。
    pub fn packaged(mut self, data: &Data, name: impl Fn(&str) -> String) -> Self {
        let Target::Provider(Some(id)) = &self.target else {
            return self;
        };
        let table = keys::provider(id);
        for (field, item) in GIVEN {
            let key = format!("{table}.{item}");
            let Some(package) = data.packaged.get(&key) else {
                continue;
            };
            let Some(row) = self.rows.iter_mut().find(|r| r.field == field) else {
                continue;
            };
            row.source = Some(format!("{PACKAGE}{}", name(package)));
            if field == Field::Driver {
                let given = data.values.get(&key).and_then(|(v, _)| v.as_str());
                pick_given(row, given);
            }
        }
        // 接口换成包给的那一个不算改动：没动这一格保存时照旧不写。
        self.initial = self.rows.clone();
        self
    }
}

/// 接口那一行没写（停在第一项「自动」）的，停到包给的那一个上；不在选项里的照原值接在最后（同 `drivers::choices`）。
fn pick_given(row: &mut Row, given: Option<&str>) {
    let (Val::Choice { options, at }, Some(given)) = (&mut row.val, given) else {
        return;
    };
    if *at != Some(0) || given.is_empty() {
        return;
    }
    let found = options.iter().position(|o| o == given);
    *at = Some(found.unwrap_or_else(|| {
        options.push(given.to_string());
        options.len() - 1
    }));
}
