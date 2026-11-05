//! 装、升级的时候批能力（核心 PK-3 上，设计 61）：问的时候列出每个包要的能力，按「是」就照看一眼的回应拼 `approve:
//! {"<编号>": [能力…]}` 带上。装的是要装的那个加 `with` 里的，升级的是 `upgrades` 加 `with` 里的；批过的（`held`）也带，没列的不写，
//! 空的不带。旧核心不认这个参数（带了整条被拒），也有 `capabilities`，不能照它判：看一眼写了 `feed`，或者来源是本机文件
//! （`source: "file"`）的才是新核心（同核心命令行 `pkg/approve.rs`）。

use serde_json::{Map, Value};

/// 看一眼的回应是不是新核心给的（认 `approve`）。
pub fn understood(preview: &Value) -> bool {
    let fed = |v: &Value| v["feed"].is_string();
    fed(preview)
        || preview["source"] == "file"
        || list(&preview["upgrades"]).any(fed)
        || list(&preview["with"]).any(fed)
}

/// 照一份看一眼拼 `approve`；旧核心的、一项能力都没有的是 `None`。
pub fn approve(preview: &Value) -> Option<Value> {
    if !understood(preview) {
        return None;
    }
    let mut map = Map::new();
    let top = preview["package"].is_string().then_some(preview);
    let all = top
        .into_iter()
        .chain(list(&preview["upgrades"]))
        .chain(list(&preview["with"]));
    for one in all {
        let Some(id) = one["package"].as_str() else {
            continue;
        };
        let caps: Vec<Value> = list(&one["capabilities"])
            .filter_map(|c| c["id"].as_str().map(Value::from))
            .collect();
        if !caps.is_empty() {
            map.insert(id.to_string(), Value::Array(caps));
        }
    }
    (!map.is_empty()).then_some(Value::Object(map))
}

/// 还有没批过的能力（不是 `held` 的）：问的那一句说「并批准」。
pub fn asks_approval(preview: &Value) -> bool {
    let top = preview["package"].is_string().then_some(preview);
    understood(preview)
        && top
            .into_iter()
            .chain(list(&preview["upgrades"]))
            .chain(list(&preview["with"]))
            .flat_map(|one| list(&one["capabilities"]))
            .any(|c| c["held"] != true)
}

fn list(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{approve, asks_approval};

    #[test]
    fn approve_lists_each_package_and_its_dependencies() {
        let preview = json!({"package": "weather", "source": "official", "feed": "official",
            "capabilities": [{"id": "net", "name": "联网", "held": true}, {"id": "fs.read", "name": "读文件"}],
            "with": [{"package": "geo", "source": "official", "feed": "official",
                      "capabilities": [{"id": "net", "name": "联网"}]},
                     {"package": "plain", "source": "official", "feed": "official"}]});
        assert_eq!(
            approve(&preview),
            Some(json!({"weather": ["net", "fs.read"], "geo": ["net"]})),
            "批过的也带、没有能力的不写"
        );
        assert!(asks_approval(&preview));
        let held = json!({"package": "w", "feed": "official",
            "capabilities": [{"id": "net", "name": "联网", "held": true}]});
        assert!(!asks_approval(&held), "都批过了不说「并批准」");
    }

    #[test]
    fn an_old_core_gets_no_approve() {
        // 旧核心的看一眼也有 `capabilities`，`source` 那时是源的名字。
        let old = json!({"package": "w", "source": "official",
            "capabilities": [{"id": "net", "name": "联网"}]});
        assert_eq!(approve(&old), None);
        assert!(!asks_approval(&old));
        let file = json!({"package": "w", "source": "file",
            "capabilities": [{"id": "net", "name": "联网"}]});
        assert_eq!(approve(&file), Some(json!({"w": ["net"]})), "本机文件的认");
    }

    #[test]
    fn an_upgrade_approves_its_upgrades_and_new_dependencies() {
        let preview = json!({"upgrades": [{"package": "im", "from": "1.0", "to": "1.1",
            "source": "official", "feed": "official", "capabilities": [{"id": "net", "name": "联网"}]}],
            "with": [], "download": 1});
        assert_eq!(approve(&preview), Some(json!({"im": ["net"]})));
    }
}
