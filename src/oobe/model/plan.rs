//! 接模型那一步要发的请求（「第一次打开的引导」第 17–20 条，照 `miyu setup` 第 4、7、8、10 条）：试一家的参数、
//! 配置里的编号怎么起、存的时候写哪几项。

use serde_json::{Value, json};

use super::rows::Provider;
use crate::settings::forms::{path_name, unique};

/// 自定义的三种接口，照界面上的先后。
pub const DRIVERS: [&str; 3] = ["openai-chat", "anthropic", "openai-responses"];

/// 试哪一家。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// 列出来的一家。
    Listed(Provider),
    /// 自定义：接口是 [`DRIVERS`] 里第几种、地址、人起的编号。
    Custom {
        /// 接口。
        driver: usize,
        /// 地址，去掉末尾的 `/`。
        base_url: String,
        /// 人起的编号；没起的照地址的主机名自动起（2026-10-13 项目主人定：默认是自动起的那个，可以改）。
        id: Option<String>,
        /// 显示名称；没填的不写，照编号显示。
        name: Option<String>,
    },
}

/// 名字照 `miyu setup` 第 7 条：自定义的是地址的主机名。
pub fn test_params(target: &Target, key: Option<&str>, model: Option<&str>) -> Value {
    let mut params = match target {
        Target::Listed(p) => match &p.configured {
            Some(id) => json!({"provider": id}),
            None => {
                let mut candidate = json!({"catalog": p.catalog});
                if let Some(env) = &p.env {
                    candidate["key"] = json!({"env": env});
                } else if let Some(key) = key.filter(|k| !k.is_empty()) {
                    candidate["key"] = json!({"value": key});
                }
                if let Some(url) = &p.base_url {
                    candidate["base_url"] = json!(url);
                }
                json!({"candidate": candidate})
            }
        },
        Target::Custom {
            driver, base_url, ..
        } => {
            let mut candidate = json!({"driver": DRIVERS[*driver], "base_url": base_url});
            if let Some(key) = key.filter(|k| !k.is_empty()) {
                candidate["key"] = json!({"value": key});
            }
            json!({"candidate": candidate})
        }
    };
    if let Some(model) = model.filter(|m| !m.is_empty()) {
        params["model"] = json!(model);
    }
    params
}

/// 地址合不合：`http://`、`https://` 开头。交回去掉前后空白和末尾 `/` 的。
pub fn url(text: &str) -> Option<String> {
    let text = text.trim().trim_end_matches('/');
    (text.starts_with("http://") || text.starts_with("https://")).then(|| text.to_string())
}

/// 配置里的编号（`miyu setup` 第 4、10 条）：列出来的照目录的编号，不合写法的改写（另交回要写的 `catalog`）；自定义的照
/// 主机名起；配置里已经有的往后加 `-2`、`-3`。配好了的就是配置里那个。
pub fn config_id(target: &Target, existing: &[String]) -> (String, Option<String>) {
    let (base, catalog) = match target {
        Target::Listed(p) => match &p.configured {
            Some(id) => return (id.clone(), None),
            None => {
                let id = path_name(&p.catalog);
                let catalog = (id != p.catalog).then(|| p.catalog.clone());
                (id, catalog)
            }
        },
        Target::Custom { id: Some(id), .. } => return (id.clone(), None),
        Target::Custom { base_url, .. } => (path_name(&host_name(base_url)), None),
    };
    (unique(base, existing), catalog)
}

/// 地址的主机名起编号：本机的（`localhost`、IP）是 `local`；别的取倒数第二段，倒数第二段是通用的二级域名的再往前一段。
fn host_name(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest
        .split(['/', ':'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let local =
        host == "localhost" || host.parse::<std::net::IpAddr>().is_ok() || host.starts_with('[');
    if local {
        return "local".to_string();
    }
    let parts: Vec<&str> = host.split('.').collect();
    let generic = ["co", "com", "net", "org", "edu", "gov", "ac"];
    match parts.as_slice() {
        [.., a, b, _] if generic.contains(b) => (*a).to_string(),
        [.., a, _] => (*a).to_string(),
        [one] => (*one).to_string(),
        [] => "custom".to_string(),
    }
}

/// key 怎么写进配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRef {
    /// 引用环境变量。
    Env(String),
    /// 存进密钥库的，名字是编号。
    Secret,
    /// 本机的服务：不要 key。
    Local,
    /// 不要 key。
    None,
}

/// 存（`config.set` 系统配置）：这一家的 key 引用或者本机的 `local`（配好了的不写）、自定义的接口和地址、不合写法的 `catalog`、
/// `models.chat`。池不写：`lite`、`normal`、`flagship` 三个固定池配置里一个字没写也在（核心 Q-1；原来一个池都没有的另写
/// 三个预设的池，`standard` 改名 `normal` 以后照旧写会多出一个人建的 `standard`）。
pub fn save_params(
    target: &Target,
    (id, catalog): (&str, Option<&str>),
    key: &KeyRef,
    model: &str,
) -> Value {
    let mut changes = Vec::new();
    let mut set = |key: String, value: Value| changes.push(json!({"key": key, "value": value}));
    let configured = matches!(target, Target::Listed(p) if p.configured.is_some());
    if !configured {
        // 一家一个 key（核心 8-25）：本机的服务不要 key、写 `local`；没贴的不写。
        match key {
            KeyRef::Env(env) => set(format!("providers.{id}.key"), json!({"env": env})),
            KeyRef::Secret => set(format!("providers.{id}.key"), json!({"secret": id})),
            KeyRef::Local => set(format!("providers.{id}.local"), json!(true)),
            KeyRef::None => {}
        }
        if let Target::Custom {
            driver,
            base_url,
            name,
            ..
        } = target
        {
            set(format!("providers.{id}.driver"), json!(DRIVERS[*driver]));
            set(format!("providers.{id}.base_url"), json!(base_url));
            if let Some(name) = name {
                set(format!("providers.{id}.name"), json!(name));
            }
        }
        if let Some(catalog) = catalog {
            set(format!("providers.{id}.catalog"), json!(catalog));
        }
    }
    set("models.chat".to_string(), json!(format!("{id}/{model}")));
    // 引导里接的模型是个人的（2026-10-24 项目主人定，「管理员模式」第 5 条；原来写系统配置）。
    json!({"changes": changes})
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{KeyRef, Target, config_id, save_params, test_params, url};
    use crate::oobe::model::rows::Provider;

    fn listed(catalog: &str) -> Provider {
        Provider {
            catalog: catalog.into(),
            name: catalog.into(),
            supported: true,
            ..Provider::default()
        }
    }

    #[test]
    fn what_is_tested_follows_how_the_provider_is_reached() {
        let configured = Target::Listed(Provider {
            configured: Some("zen".into()),
            ..listed("opencode")
        });
        assert_eq!(
            test_params(&configured, None, None),
            json!({"provider": "zen"})
        );
        let found = Target::Listed(Provider {
            env: Some("DEEPSEEK_API_KEY".into()),
            ..listed("deepseek")
        });
        assert_eq!(
            test_params(&found, None, None),
            json!({"candidate": {"catalog": "deepseek", "key": {"env": "DEEPSEEK_API_KEY"}}})
        );
        let pasted = Target::Listed(listed("openai"));
        assert_eq!(
            test_params(&pasted, Some("sk-x"), Some("gpt-4o")),
            json!({"candidate": {"catalog": "openai", "key": {"value": "sk-x"}}, "model": "gpt-4o"})
        );
        let custom = Target::Custom {
            driver: 1,
            base_url: "https://api.example.invalid/v1".into(),
            id: None,
            name: None,
        };
        assert_eq!(
            test_params(&custom, Some(""), None),
            json!({"candidate": {"driver": "anthropic", "base_url": "https://api.example.invalid/v1"}}),
            "空着的 key 不带"
        );
    }

    #[test]
    fn ids_come_from_the_catalog_or_the_host_and_never_collide() {
        assert_eq!(
            url(" https://api.example.invalid/v1/ "),
            Some("https://api.example.invalid/v1".into())
        );
        assert_eq!(url("api.example.invalid"), None);
        let custom = |u: &str| Target::Custom {
            driver: 0,
            base_url: u.into(),
            id: None,
            name: None,
        };
        assert_eq!(
            config_id(&custom("https://api.example.invalid/v1"), &[]).0,
            "example"
        );
        assert_eq!(
            config_id(&custom("https://api.example.co.uk/v1"), &[]).0,
            "example"
        );
        assert_eq!(
            config_id(&custom("http://127.0.0.1:8788/v1"), &[]).0,
            "local"
        );
        assert_eq!(
            config_id(&custom("http://localhost:1234/v1"), &[]).0,
            "local"
        );
        let taken = vec!["example".to_string(), "example-2".to_string()];
        assert_eq!(
            config_id(&custom("https://api.example.invalid"), &taken).0,
            "example-3"
        );
        assert_eq!(
            config_id(&Target::Listed(listed("302ai")), &[]),
            ("p-302ai".to_string(), Some("302ai".to_string()))
        );
        assert_eq!(
            config_id(&Target::Listed(listed("wafer.ai")), &[]).0,
            "wafer-ai"
        );
        // 核心的编号最长 32 个、不收 Windows 保留名：起的编号连尾巴都守着。
        let long = "a".repeat(40);
        assert_eq!(
            config_id(&Target::Listed(listed(&long)), &[]).0,
            "a".repeat(32)
        );
        let taken = vec!["a".repeat(32)];
        let next = config_id(&Target::Listed(listed(&long)), &taken).0;
        assert_eq!(next, format!("{}-2", "a".repeat(30)));
        assert_eq!(config_id(&Target::Listed(listed("con")), &[]).0, "p-con");
    }

    #[test]
    fn saving_writes_the_key_reference_and_the_chat_model_but_no_pools() {
        let pasted = Target::Listed(listed("deepseek"));
        let got = save_params(
            &pasted,
            ("deepseek", None),
            &KeyRef::Secret,
            "deepseek-flash",
        );
        let changes = got["changes"].as_array().unwrap();
        assert!(got.get("layer").is_none(), "不写层：就是系统配置");
        assert!(
            changes.contains(
                &json!({"key": "providers.deepseek.key", "value": {"secret": "deepseek"}})
            )
        );
        assert!(
            changes.contains(&json!({"key": "models.chat", "value": "deepseek/deepseek-flash"}))
        );
        assert!(
            !got.to_string().contains("pools."),
            "池不写：三个固定池核心自己有（Q-1）"
        );
        let configured = Target::Listed(Provider {
            configured: Some("zen".into()),
            ..listed("opencode")
        });
        let only = save_params(&configured, ("zen", None), &KeyRef::None, "m");
        assert_eq!(
            only["changes"],
            json!([{"key": "models.chat", "value": "zen/m"}]),
            "配好了的只写 models.chat"
        );
        let custom = Target::Custom {
            driver: 0,
            base_url: "https://api.example.invalid/v1".into(),
            id: None,
            name: None,
        };
        let got = save_params(&custom, ("example", None), &KeyRef::None, "m");
        assert!(
            got["changes"]
                .as_array()
                .unwrap()
                .contains(&json!({"key": "providers.example.driver", "value": "openai-chat"}))
        );
        assert!(
            !got.to_string().contains("providers.example.key\""),
            "没贴 key 的不写 key（核心 8-25）"
        );
        assert!(
            !got.to_string().contains("providers.example.name"),
            "没填显示名称的不写"
        );
        let named = Target::Custom {
            driver: 0,
            base_url: "https://api.example.invalid/v1".into(),
            id: None,
            name: Some("中转站".into()),
        };
        let got = save_params(&named, ("example", None), &KeyRef::None, "m");
        assert!(
            got["changes"]
                .as_array()
                .unwrap()
                .contains(&json!({"key": "providers.example.name", "value": "中转站"})),
            "填了显示名称的写进去（2026-10-26 项目主人）"
        );
        let local = Target::Listed(Provider {
            base_url: Some("http://127.0.0.1:11434/v1".into()),
            ..listed("ollama")
        });
        let got = save_params(&local, ("ollama", None), &KeyRef::Local, "qwen3");
        let changes = got["changes"].as_array().unwrap();
        assert!(
            changes.contains(&json!({"key": "providers.ollama.local", "value": true})),
            "本机的服务写 local：{got}"
        );
        assert!(!got.to_string().contains("providers.ollama.key\""));
    }
}
