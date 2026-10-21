//! 「吉祥物」「主题」两项的选择窗列的包（蓝图 `tui.md`「吉祥物包」第 3 条、「主题」第 5 条）：照 `package.list` 挑读成了的
//! `kind = "mascot"`、`kind = "theme"`，和人格一个形状（编号、名字），第一行「内置」「默认主题」由 `choices` 加。

use serde_json::Value;

use super::personas::Persona;

/// 读 `package.list` 的回应：吉祥物包一行行。
pub fn read(got: &Value) -> Vec<Persona> {
    of_kind(got, "mascot", None)
}

/// 主题包一行行（「主题」第 5 条）：默认主题包不列，第一行「默认主题」就是它。
pub fn looks(got: &Value) -> Vec<Persona> {
    of_kind(got, "theme", Some(crate::sheet::DEFAULT_PACKAGE))
}

/// 这一种（`kind`）读成了的包，跳过 `skip` 那一个。
fn of_kind(got: &Value, kind: &str, skip: Option<&str>) -> Vec<Persona> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    got["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["kind"] == kind && p["package"].as_str() != skip)
        .map(|p| Persona {
            id: text(&p["package"]).unwrap_or_default(),
            name: text(&p["name"]),
            summary: text(&p["summary"]),
            problem: text(&p["problem"]),
            avatar: None,
            seed: None,
            seeds: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::read;

    #[test]
    fn only_mascot_packages_are_listed() {
        let got = json!({"packages": [
            {"package": "tui", "kind": "ui", "name": "终端界面"},
            {"package": "pudding", "kind": "mascot", "name": "布丁", "mascot": {"model": "mascot.json"}},
            {"package": "broken", "layer": "personal", "code": "bad_toml", "problem": "写错了"}
        ]});
        let list = read(&got);
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].id.as_str(), list[0].label()), ("pudding", "布丁"));
    }

    #[test]
    fn theme_packages_are_listed_without_the_default_one() {
        let got = json!({"packages": [
            {"package": "theme-default", "kind": "theme", "name": "默认主题", "theme": {"file": "theme.json"}},
            {"package": "theme-ascii", "kind": "theme", "name": "纯 ASCII", "theme": {"file": "theme.json"}},
            {"package": "pudding", "kind": "mascot", "name": "布丁"}
        ]});
        let list = super::looks(&got);
        assert_eq!(list.len(), 1, "默认主题包不列：第一行就是它");
        assert_eq!(
            (list[0].id.as_str(), list[0].label()),
            ("theme-ascii", "纯 ASCII")
        );
    }
}
