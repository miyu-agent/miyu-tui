//! 供应商的驱动下拉（蓝图「配置页」供应商窗，核心 MD-1）：选项照 `config.schema` 那一项（`providers.<id>.driver`）的
//! `options`（照原回应读：读成几页时键里有占位的那些滤掉了）：三个内置的在前，后面接装着的包声明的 `<包编号>/<驱动名>`，装卸包以后核心重算。清单还没读到的照三个内置的。
//! 配置里写了、这时不在选项里的（没装那个包、名字写错，核心记 `unknown_driver`）照原值多列一项，不显示成「默认」，也不
//! 悄悄换掉。第一项空的是「默认」（不写）。

use serde_json::Value;

/// 内置的三个，照先后。
const BUILT_IN: [&str; 3] = ["openai-chat", "anthropic", "openai-responses"];

/// 配置清单里驱动那一项的键。
const KEY: &str = "providers.<id>.driver";

/// 能选的驱动：照 `config.schema` 原回应（`more.raw` 记着的那一份；读成几页时键里有占位的都滤掉了，这一项不在）里那一项的
/// 选项，没读到的照内置的三个。
pub fn known(schema: Option<&Value>) -> Vec<String> {
    let listed: Vec<String> = schema
        .and_then(|got| got["items"].as_array())
        .and_then(|items| items.iter().find(|i| i["key"] == KEY))
        .and_then(|item| item["options"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|o| o["value"].as_str().map(str::to_string))
        .collect();
    if listed.is_empty() {
        BUILT_IN.map(String::from).to_vec()
    } else {
        listed
    }
}

/// 下拉的一项项（第一项空的是「默认」）和停在哪：配置里写的不在 `known` 里的照原值接在最后。
pub fn choices(known: &[String], written: Option<&str>) -> (Vec<String>, usize) {
    let mut options = vec![String::new()];
    options.extend(known.iter().cloned());
    if let Some(written) = written.filter(|w| !w.is_empty() && !options.iter().any(|o| o == w)) {
        options.push(written.to_string());
    }
    let at = written
        .and_then(|w| options.iter().position(|o| o == w))
        .unwrap_or(0);
    (options, at)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{choices, known};

    fn schema(values: &[&str]) -> serde_json::Value {
        let options: Vec<_> = values
            .iter()
            .map(|v| json!({"value": v, "name": v}))
            .collect();
        json!({"items": [
            {"key": "providers.<id>.name", "type": "text"},
            {"key": "providers.<id>.driver", "type": "option", "options": options}
        ]})
    }

    #[test]
    fn the_drivers_follow_the_schema_and_fall_back_to_the_built_in_three() {
        assert_eq!(
            known(None),
            ["openai-chat", "anthropic", "openai-responses"]
        );
        let got = schema(&["openai-chat", "anthropic", "openai-responses", "acme/fast"]);
        assert_eq!(
            known(Some(&got)).last().map(String::as_str),
            Some("acme/fast")
        );
    }

    #[test]
    fn a_written_driver_that_is_not_listed_stays_as_it_is() {
        let known = vec!["openai-chat".to_string(), "anthropic".to_string()];
        assert_eq!(
            choices(&known, Some("anthropic")),
            (vec!["".into(), "openai-chat".into(), "anthropic".into()], 2)
        );
        let (options, at) = choices(&known, Some("gone/drv"));
        assert_eq!(options.last().map(String::as_str), Some("gone/drv"));
        assert_eq!(options[at], "gone/drv", "停在写的那一个，不显示成默认");
        assert_eq!(choices(&known, None).1, 0, "没写的是默认");
    }
}
