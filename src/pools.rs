//! 固定的三个模型池（核心 Q-1：`flagship`、`normal`、`lite` 配置里一个字没写也在）：排在所有池最前面，照旗舰、日常、
//! 轻量的先后；给人看的名字照界面语言（`text/<语言>.json` 的 `settings.pool_names`，2026-10-26 项目主人定；2026-11-09 改成
//! 「旗舰池」「日常池」「轻量池」）。`/model`、底栏写成 `@旗舰池`（[`label`]，2026-11-09 项目主人：不写 `@flagship`），
//! 配置页的模型池那一页写不带 `@` 的名字；几处共用这一套。

use std::collections::HashMap;

/// 固定的三个池，照排的先后。
pub const FIXED: [&str; 3] = ["flagship", "normal", "lite"];

/// 排在第几：固定的照 [`FIXED`] 的先后排最前，别的排在它们后面（同一档照原来的先后，排序要稳）。
pub fn rank(name: &str) -> usize {
    FIXED
        .iter()
        .position(|fixed| *fixed == name)
        .unwrap_or(FIXED.len())
}

/// 池的引用给人看的写法：固定的三个池（`@flagship` 这样）照界面语言写成 `@旗舰池`；别的池、不是池的交 `None`（照原样写）。
pub fn label(reference: &str, names: &HashMap<String, String>) -> Option<String> {
    let name = reference.strip_prefix('@')?;
    FIXED
        .contains(&name)
        .then(|| names.get(name))
        .flatten()
        .map(|shown| format!("@{shown}"))
}

#[cfg(test)]
mod tests {
    use super::{label, rank};
    use std::collections::HashMap;

    #[test]
    fn the_fixed_pools_go_first_flagship_normal_lite() {
        let mut names = vec!["duo", "lite", "mine", "normal", "flagship"];
        names.sort_by_key(|n| rank(n));
        assert_eq!(names, ["flagship", "normal", "lite", "duo", "mine"]);
    }

    #[test]
    fn fixed_pools_are_shown_by_their_names_with_an_at() {
        let names = HashMap::from([("flagship".to_string(), "旗舰池".to_string())]);
        assert_eq!(label("@flagship", &names).as_deref(), Some("@旗舰池"));
        // 自己建的池、没给名字的、不是池的照原样写
        assert_eq!(label("@duo", &names), None);
        assert_eq!(label("@lite", &names), None);
        assert_eq!(label("flagship", &names), None);
        assert_eq!(label("dev/flagship", &names), None);
    }
}
