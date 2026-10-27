//! 固定的三个模型池（核心 Q-1：`flagship`、`normal`、`lite` 配置里一个字没写也在）：排在所有池最前面，照旗舰、日常、
//! 轻量的先后；给人看的名字照界面语言（`text/<语言>.json` 的 `settings.pool_names`，2026-10-26 项目主人定）。`/model`
//! 和配置页的模型池那一页共用这一套。

/// 固定的三个池，照排的先后。
pub const FIXED: [&str; 3] = ["flagship", "normal", "lite"];

/// 排在第几：固定的照 [`FIXED`] 的先后排最前，别的排在它们后面（同一档照原来的先后，排序要稳）。
pub fn rank(name: &str) -> usize {
    FIXED
        .iter()
        .position(|fixed| *fixed == name)
        .unwrap_or(FIXED.len())
}

#[cfg(test)]
mod tests {
    use super::rank;

    #[test]
    fn the_fixed_pools_go_first_flagship_normal_lite() {
        let mut names = vec!["duo", "lite", "mine", "normal", "flagship"];
        names.sort_by_key(|n| rank(n));
        assert_eq!(names, ["flagship", "normal", "lite", "duo", "mine"]);
    }
}
