//! 核心认的斜杠命令（`protocol.md` 的 `command.catalog`，核心 O-6 补、X-14）：内置的、自己放的命令模板、包里的模板、
//! 扩展声明的，照名字排；带会话问的只列这个会话里打了不会被拒的。不推送：连上、打开命令列表、换语言时问一次。终端自己的
//! 同名命令照自己的（`commands.rs` 合的时候去掉）。

use serde_json::Value;

/// 清单里的一条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreCommand {
    /// 名字，不带 `/`。
    pub name: String,
    /// 别名。
    pub aliases: Vec<String>,
    /// 一句说明（照连接的语言）。
    pub summary: String,
    /// 名字后面跟什么的提示（模板的 `argument-hint`，例如 `<路径>`）；没写的是 `None`。
    pub argument: Option<String>,
    /// 扩展的整页带的命令：打了就开这一页（核心 X-15 下）；别的是 `None`。
    pub opens: Option<crate::widgets::PageRef>,
    /// 有几种写法的（核心 O-6 四补，`/goal` 有五种）：一种写法、一句说明，照连接的语言；`/help` 一行一种（蓝图
    /// 「帮助」）。没有的是空的。
    pub usage: Vec<(String, String)>,
}

/// 读回应的 `commands`；读不懂的那一条不要。
pub fn read(result: &Value) -> Vec<CoreCommand> {
    let text = |v: &Value| v.as_str().map(str::to_string);
    result["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some(CoreCommand {
                name: text(&c["name"]).filter(|n| !n.is_empty())?,
                aliases: c["aliases"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(text)
                    .collect(),
                summary: text(&c["summary"]).unwrap_or_default(),
                argument: text(&c["argument"]).filter(|a| !a.is_empty()),
                opens: crate::widgets::PageRef::read(&c["opens"]),
                usage: c["usage"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|u| {
                        Some((
                            text(&u["form"]).filter(|f| !f.is_empty())?,
                            text(&u["summary"]).unwrap_or_default(),
                        ))
                    })
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{CoreCommand, read};

    #[test]
    fn the_catalog_is_read_with_aliases_and_argument_hints() {
        let got = json!({"commands": [
            {"name": "clear", "aliases": ["reset"], "summary": "清空上下文"},
            {"name": "review", "aliases": [], "summary": "审一遍代码", "argument": "<路径>"},
            {"name": "", "summary": "坏的"},
            {"name": "ci", "aliases": [], "summary": "构建", "opens": {"package": "ciw", "widget": "all"}}
        ]});
        assert_eq!(
            read(&got),
            [
                CoreCommand {
                    name: "clear".into(),
                    aliases: vec!["reset".into()],
                    summary: "清空上下文".into(),
                    argument: None,
                    opens: None,
                    usage: Vec::new(),
                },
                CoreCommand {
                    name: "review".into(),
                    aliases: vec![],
                    summary: "审一遍代码".into(),
                    argument: Some("<路径>".into()),
                    opens: None,
                    usage: Vec::new(),
                },
                CoreCommand {
                    name: "ci".into(),
                    aliases: vec![],
                    summary: "构建".into(),
                    argument: None,
                    opens: Some(crate::widgets::PageRef {
                        package: "ciw".into(),
                        widget: "all".into(),
                    }),
                    usage: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn several_forms_are_read_in_order() {
        // 核心 O-6 四补：`usage` 一种写法一句说明，没写写法的不要。
        let got = read(&json!({"commands": [{"name": "goal", "summary": "长期目标",
            "argument": "[<目标> | pause]",
            "usage": [{"form": "/goal", "summary": "看进度"}, {"form": "", "summary": "坏的"},
                {"form": "/goal pause", "summary": "暂停"}]}]}));
        assert_eq!(
            got[0].usage,
            [
                ("/goal".to_string(), "看进度".to_string()),
                ("/goal pause".to_string(), "暂停".to_string())
            ]
        );
    }
}
