//! 同一家供应商自用、共用两份（核心 PL-1，2026-10-24 项目主人定「两份都能用」）：系统配置、个人设置都配了的那一家，
//! `model.list` 列两份，`<编号>` 是自用的、`<编号>@system` 是共用的。只有两份都在时才标「自用」「共用」（2026-10-27 项目
//! 主人照推荐定：只有一份的不写），引用照列表原样发回。

/// 共用那份的编号后缀。
const SYSTEM: &str = "@system";

/// 去掉共用的后缀：两份是同一家。
pub fn base(id: &str) -> &str {
    id.strip_suffix(SYSTEM).unwrap_or(id)
}

/// 列表 `ids` 里这一家有两份时，`id` 这一份是不是共用的：`Some(true)` 共用、`Some(false)` 自用；只有一份的是 `None`。
pub fn shared<'a>(id: &str, ids: impl IntoIterator<Item = &'a str>) -> Option<bool> {
    let twins = ids
        .into_iter()
        .filter(|other| base(other) == base(id))
        .count();
    (twins > 1).then(|| id.ends_with(SYSTEM))
}

#[cfg(test)]
mod tests {
    use super::{base, shared};

    #[test]
    fn only_a_provider_listed_twice_is_tagged() {
        let ids = ["relay", "relay@system", "deepseek"];
        assert_eq!(shared("relay", ids), Some(false), "自用");
        assert_eq!(shared("relay@system", ids), Some(true), "共用");
        assert_eq!(shared("deepseek", ids), None, "只有一份不标");
        assert_eq!(base("relay@system"), "relay");
    }
}
