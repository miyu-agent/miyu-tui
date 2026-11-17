//! 借来的项（核心 NS-1）：`origin` 是 `{"layer": "inherited", "from": "providers.<编号>.key"}`、值是 `null` 的，写「已继承」和借的
//! 哪一家，不写「未设置」（2026-11-17 项目主人本机验收第 10 条）。

use serde_json::json;

use crate::settings::data::Data;
use crate::settings::pages::inherited;
use crate::settings::pages::schema::Schema;

#[test]
fn a_borrowed_key_says_inherited_and_from_which_provider() {
    let texts = crate::config::Config::builtin().unwrap().text.settings;
    let schema = Schema::read(&json!({
        "pages": [{"id": "packages", "name": "软件包"}],
        "groups": [{"id": "net", "name": "网络搜索", "page": "packages"}],
        "items": [{"key": "net.search.deepseek.key", "type": "secret", "control": "secret", "layers": ["system"],
                   "name": "DeepSeek 的 key", "page": "packages", "group": "net"}]}));
    let item = &schema.items[0];
    let mut data = Data::default();
    data.read_models(&json!({"providers": [{"id": "ds", "name": {"value": "DeepSeek"}}]}));
    data.read_config(&json!({"items": {"net.search.deepseek.key": {
        "origin": {"layer": "inherited", "from": "providers.ds.key"}, "value": null}}}));
    assert_eq!(
        inherited(item, &data, &texts.more).as_deref(),
        Some("已继承（来自 DeepSeek）")
    );
    // 自己写了的（来源不是 `inherited`）照旧。
    data.read_config(&json!({"items": {"net.search.deepseek.key": {
        "origin": {"layer": "system"}, "value": "secret:x"}}}));
    assert_eq!(inherited(item, &data, &texts.more), None);
}
