//! 「吉祥物」「主题」两项的选择窗列的包（蓝图 `tui.md`「吉祥物包」第 3 条、「主题」第 5 条）：照 `package.list` 挑读成了的
//! `kind = "mascot"`、`kind = "theme"`，和人格一个形状（编号、名字），第一行「内置」「默认主题」由 `choices` 加。

use serde_json::Value;

use super::personas::Persona;

/// 读 `package.list` 的回应：吉祥物包一行行。
pub fn read(got: &Value) -> Vec<Persona> {
    of_kind(got, "mascot", None)
}

/// 主题一行行（「主题」第 5 条）：默认主题包不列，第一行「默认主题」就是它；接着是编进程序的纯 ASCII（`ascii` 是它照界面
/// 语言的名字，todolist 63），一直在；再接装了的主题包。外观包可以只带网页的主题，没带终端主题的不列（[`tui_file`]，
/// 2026-11-02 核心定的形状）；和内置编号重名的包不另列。
pub fn looks(got: &Value, ascii: &str) -> Vec<Persona> {
    let mut list = of_kind(got, "theme", Some(crate::sheet::DEFAULT_PACKAGE));
    list.retain(|p| {
        p.id != crate::sheet::ASCII
            && crate::core::live_packages(got)
                .any(|item| item["package"] == p.id.as_str() && tui_file(item).is_some())
    });
    let builtin = Persona {
        id: crate::sheet::ASCII.to_string(),
        name: Some(ascii.to_string()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
        tokens: None,
    };
    std::iter::once(builtin).chain(list).collect()
}

/// `package.list` 一项外观包里终端主题的文件：`theme.tui.file`；核心改形状以前的 `theme.file` 也认（过渡）。只带网页主题
/// （`theme.web`）的没有。
pub fn tui_file(item: &Value) -> Option<&str> {
    item["theme"]["tui"]["file"]
        .as_str()
        .or_else(|| item["theme"]["file"].as_str())
}

/// 这一种（`kind`）读成了的包，跳过 `skip` 那一个。
fn of_kind(got: &Value, kind: &str, skip: Option<&str>) -> Vec<Persona> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    crate::core::live_packages(got)
        .filter(|p| p["kind"] == kind && p["package"].as_str() != skip)
        .map(|p| Persona {
            id: text(&p["package"]).unwrap_or_default(),
            name: text(&p["name"]),
            summary: text(&p["summary"]),
            problem: text(&p["problem"]),
            avatar: None,
            seed: None,
            seeds: None,
            tokens: None,
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
            {"package": "broken", "layer": "system", "code": "bad_toml", "problem": "写错了"}
        ]});
        let list = read(&got);
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].id.as_str(), list[0].label()), ("pudding", "布丁"));
    }

    /// 纯 ASCII 编进了程序（todolist 63）：没装任何主题包也列着，排在装了的前面；默认主题包不列（第一行「默认主题」就是它）。
    #[test]
    fn the_builtin_ascii_look_is_always_listed_before_installed_packages() {
        let got = json!({"packages": [
            {"package": "tui-theme-default", "kind": "theme", "name": "默认主题", "theme": {"file": "theme.json"}},
            {"package": "night", "kind": "theme", "name": "夜", "theme": {"tui": {"file": "theme.json"}}},
            {"package": "pudding", "kind": "mascot", "name": "布丁"}
        ]});
        let list = super::looks(&got, "纯 ASCII");
        let rows: Vec<(&str, &str)> = list.iter().map(|p| (p.id.as_str(), p.label())).collect();
        assert_eq!(rows, [("ascii", "纯 ASCII"), ("night", "夜")]);
        assert_eq!(
            super::looks(&json!({"packages": []}), "Plain ASCII").len(),
            1
        );
    }

    /// 核心 SD-1：被盖住的那份（带 `covered`）不再列一行。
    #[test]
    fn a_covered_theme_is_listed_once() {
        let got = json!({"packages": [
            {"package": "night", "kind": "theme", "layer": "distro", "theme": {"file": "old.json"},
             "covered": {"by": "home", "version": "1.0.0", "over": "1.1.0", "newer": false}},
            {"package": "night", "kind": "theme", "layer": "home", "theme": {"file": "t.json"}}
        ]});
        assert_eq!(
            super::looks(&got, "纯 ASCII").len(),
            2,
            "内置的一行加 night 一行"
        );
    }

    /// 外观包（2026-11-02 核心定：`[theme.tui] file`、`[theme.web] dir`，带一样就行）：只带网页主题的不列；新形状
    /// `theme.tui.file`、旧形状 `theme.file` 都认。
    #[test]
    fn appearance_packages_without_a_terminal_theme_are_not_listed() {
        let got = json!({"packages": [
            {"package": "both", "kind": "theme", "name": "两样都带", "theme": {"tui": {"file": "t.json"}, "web": {"dir": "web"}}},
            {"package": "web-only", "kind": "theme", "name": "只有网页的", "theme": {"web": {"dir": "web"}}},
            {"package": "old", "kind": "theme", "name": "旧形状", "theme": {"file": "theme.json"}}
        ]});
        let ids: Vec<String> = super::looks(&got, "纯 ASCII")
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, ["ascii", "both", "old"]);
        assert_eq!(super::tui_file(&got["packages"][0]), Some("t.json"));
        assert_eq!(super::tui_file(&got["packages"][1]), None);
        assert_eq!(super::tui_file(&got["packages"][2]), Some("theme.json"));
    }
}
