//! 模型资料（`model.list`，核心 8-7、8-9）里界面要的那一点：冷却着的模型里最早什么时候恢复（蓝图「配置与模型」第 7 条）。

use jiff::Timestamp;
use serde_json::Value;

/// 每一家的每个模型里，状态是 `cooling` 的那些最早的 `until`；没有的是 `None`。
pub fn earliest_cooling(result: &Value) -> Option<Timestamp> {
    result["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|provider| provider["models"].as_array().into_iter().flatten())
        .filter(|model| model["state"] == "cooling")
        .filter_map(|model| model["until"].as_str()?.parse().ok())
        .min()
}

/// `/model` 框里的一行（蓝图「配置与模型」第 1 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// 交给核心的引用：模型是 `供应商/模型`，池是 `@名字`，挡位是挡位名。
    pub reference: String,
    /// 写在前面的名字。
    pub name: String,
    /// 暗色写在名字后面的：模型的供应商和窗口、池的分法和成员数、挡位指向哪个。
    pub detail: String,
    /// 能不能选。
    pub state: ChoiceState,
}

/// 一行能不能选。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceState {
    /// 能用。
    Ok,
    /// 在冷却，到这个时刻（没给的是 `None`）。
    Cooling(Option<Timestamp>),
    /// 写了 key，一个都没有值。
    NoKey,
}

impl ChoiceState {
    /// 选了能不能换过去：没有 key 的不能（发出去也是错）；冷却着的照换（2026-10-19 项目主人：「这是我主动操作啊」，
    /// 手动选的不该被冷却挡），列表里照旧写「冷却 10m」。
    pub fn pickable(self) -> bool {
        self != Self::NoKey
    }
}

/// 照 `model.list` 排出 `/model` 的一行行：池、模型，照这个先后。没有挡位（核心 8-8 补）；用不了的池不列（核心 8-23 的
/// `usable` 是假的，旧核心没有这一格的照成员是空的认）：选了核心也只会拒。
pub fn choices(result: &Value) -> Vec<Choice> {
    let mut out = Vec::new();
    // 固定的三个池排最前，照旗舰、日常、轻量（2026-10-26 项目主人，`pools.rs`）。
    let mut pools: Vec<&Value> = result["pools"].as_array().into_iter().flatten().collect();
    pools.sort_by_key(|pool| crate::pools::rank(pool["name"].as_str().unwrap_or_default()));
    for pool in pools {
        let Some(name) = pool["name"].as_str() else {
            continue;
        };
        let members = pool["models"].as_array().map_or(0, Vec::len);
        if members == 0 || pool["usable"] == false {
            continue;
        }
        out.push(Choice {
            reference: format!("@{name}"),
            name: format!("@{name}"),
            detail: format!(
                "{}{}{members}",
                pool["strategy"].as_str().unwrap_or("pin"),
                crate::sheet::sep()
            ),
            state: ChoiceState::Ok,
        });
    }
    for provider in result["providers"].as_array().into_iter().flatten() {
        let provider_id = provider["id"].as_str().unwrap_or_default();
        for model in provider["models"].as_array().into_iter().flatten() {
            let Some(reference) = model["ref"].as_str() else {
                continue;
            };
            // 名字写模型自己的（2026-10-26 项目主人：`deepseek-flash` 写成了目录里对上的「Deepseek V4.1 Flash 1017」）；
            // 配置里给它起了名字的照起的写。
            let name = &model["facts"]["name"];
            let named = name["value"].as_str().filter(|_| name["from"] == "config");
            let name = named.or(model["model"].as_str()).unwrap_or(reference);
            let window = model["facts"]["window"]["value"].as_u64();
            // 配置里起了名字的写完整的引用：同一个名字常有好几个，只写供应商分不出来。没起名字的名字就是模型编号，右边只写
            // 供应商，不把编号再写一遍（2026-10-27 项目主人：「这里是不是只要显示一个就好了？两者重复了」）。
            let shown = match named {
                None => provider_id.to_string(),
                Some(_) => reference.to_string(),
            };
            let detail = match window {
                Some(window) => format!(
                    "{shown}{}{}",
                    crate::sheet::sep(),
                    crate::meter::short(window)
                ),
                None => shown,
            };
            let state = match model["state"].as_str() {
                Some("cooling") => {
                    ChoiceState::Cooling(model["until"].as_str().and_then(|t| t.parse().ok()))
                }
                Some("no_key") => ChoiceState::NoKey,
                _ => ChoiceState::Ok,
            };
            out.push(Choice {
                reference: reference.to_string(),
                name: name.to_string(),
                detail,
                state,
            });
        }
    }
    out
}

/// 会话现在用的模型（订阅回应里的 `model`，核心 8-10）：轮换的池只有引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Current {
    /// 端点。
    pub endpoint: Option<String>,
    /// 模型。
    pub model: Option<String>,
    /// 引用：模型、`@池` 或挡位。
    pub reference: String,
    /// 接下来那个模型真用的思考强度（核心 8-18）；什么都不发的、轮换的池是 `None`。
    pub effort: Option<String>,
}

/// 订阅回应（`result`）里的 `model`；什么都没配的没有。
pub fn current(result: &Value) -> Option<Current> {
    let model = &result["model"];
    Some(Current {
        endpoint: model["endpoint"].as_str().map(str::to_string),
        model: model["model"].as_str().map(str::to_string),
        reference: model["ref"].as_str()?.to_string(),
        effort: model["effort"]["level"].as_str().map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn a_cooling_model_can_still_be_picked_by_hand() {
        // 2026-10-19 项目主人：「这是我主动操作啊，不应该这样！」只有没 key 的挡。
        use super::ChoiceState;
        assert!(ChoiceState::Ok.pickable());
        assert!(ChoiceState::Cooling(None).pickable());
        assert!(!ChoiceState::NoKey.pickable());
    }

    use super::{ChoiceState, choices, earliest_cooling};

    #[test]
    fn the_earliest_cooling_model_decides_when_things_come_back() {
        let list = json!({"providers":[
            {"id":"a","models":[
                {"model":"m1","state":"cooling","until":"2026-10-01T08:05:00Z"},
                {"model":"m2","state":"ok"}]},
            {"id":"b","models":[{"model":"m3","state":"cooling","until":"2026-10-01T08:02:00Z"}]}]});
        assert_eq!(
            earliest_cooling(&list).unwrap().to_string(),
            "2026-10-01T08:02:00Z"
        );
        assert_eq!(earliest_cooling(&json!({"providers":[]})), None);
    }

    #[test]
    fn a_pool_the_core_says_is_unusable_is_not_listed() {
        // 核心 8-23：池的 `usable` 是假的不列（成员都下架了被拿空的、以后别的原因用不了的），没有这一格的照成员认。
        let list = json!({"pools": [
            {"name": "gone", "strategy": "pin", "models": ["dev/old"], "usable": false},
            {"name": "duo", "strategy": "pin", "models": ["dev/m1"], "usable": true},
            {"name": "legacy", "strategy": "pin", "models": ["dev/m1"]}],
            "providers": []});
        let refs: Vec<String> = choices(&list).into_iter().map(|c| c.reference).collect();
        assert_eq!(refs, ["@duo", "@legacy"]);
    }

    #[test]
    fn the_fixed_pools_come_first_flagship_normal_lite() {
        // 2026-10-26 项目主人：照旗舰、日常、轻量排在所有池最前面。
        let pool = |name: &str| json!({"name": name, "strategy": "pin", "models": ["dev/m1"], "usable": true});
        let list = json!({"pools": [pool("duo"), pool("lite"), pool("normal"), pool("mine"), pool("flagship")],
            "providers": []});
        let refs: Vec<String> = choices(&list).into_iter().map(|c| c.reference).collect();
        assert_eq!(refs, ["@flagship", "@normal", "@lite", "@duo", "@mine"]);
    }

    #[test]
    fn choices_list_pools_with_members_then_models_with_their_state() {
        // 「配置与模型」第 1 条，核心 8-7 到 8-9；8-8 补起没有挡位（2026-10-01 项目主人定），空的池不列。
        let list = json!({
            "pools": [
                {"name": "duo", "strategy": "pin", "models": ["a/m", "dev/m1"], "subagent": false, "description": null},
                {"name": "lite", "strategy": "pin", "models": [], "subagent": true, "description": null}],
            "providers": [{"id": "dev", "models": [
                {"model": "m1", "ref": "dev/m1", "state": "ok",
                 "facts": {"window": {"value": 300000}, "name": {"value": "DeepSeek V4.1 Flash", "from": "catalog"}}},
                {"model": "m2", "ref": "dev/m2", "state": "cooling", "until": "2026-10-01T08:02:00Z", "facts": {}},
                {"model": "m3", "ref": "dev/m3", "state": "no_key", "facts": {}},
                {"model": "m4", "ref": "dev/m4", "state": "ok",
                 "facts": {"name": {"value": "我的模型", "from": "config", "layer": "system"}}}]}]});
        let got = choices(&list);
        let refs: Vec<&str> = got.iter().map(|c| c.reference.as_str()).collect();
        assert_eq!(
            refs,
            ["@duo", "dev/m1", "dev/m2", "dev/m3", "dev/m4"],
            "没有挡位，空的池不列"
        );
        assert_eq!(got[0].detail, "pin · 2");
        assert_eq!(
            (got[1].name.as_str(), got[1].detail.as_str()),
            ("m1", "dev · 300k"),
            "写模型自己的名字，不写目录里对上的（2026-10-26 项目主人）；右边只写供应商，不把编号再写一遍（10-27）"
        );
        assert!(matches!(got[2].state, ChoiceState::Cooling(Some(_))));
        assert_eq!(got[3].state, ChoiceState::NoKey);
        assert_eq!(got[3].name, "m3", "没有显示名的写模型名");
        assert_eq!(got[4].name, "我的模型", "配置里起了名字的照起的写");
        assert_eq!(
            got[4].detail, "dev/m4",
            "起了名字的右边写完整的引用，好分辨"
        );
    }
}
