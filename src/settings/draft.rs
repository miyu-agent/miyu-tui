//! 草稿（蓝图「配置页」第 16、24 条）：悬浮窗「确定」记进这里，`s` 一起存。草稿就是要发的那条 `config.set`：键 → 改成什么
//! 或删掉；另记要存进密钥库的明文（照要写引用的那一项记：供应商的 key、包的密钥项，存的时候才起名字）。写系统配置（核心 SU-3
//! 上起只剩这一层，`config.set` 不写 `layer`）。

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::data::Data;
use super::keys;

/// 一项改成什么。
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// 写成这个值。
    Set(Value),
    /// 从系统配置删掉，回到出厂默认。
    Unset,
}

/// 没存的改动。
#[derive(Debug, Default, Clone)]
pub struct Draft {
    changes: BTreeMap<String, Change>,
    /// 要写引用的那一项的键 → 新贴的明文（只在这里和发 `secret.set` 那一刻）：供应商的 `providers.<编号>.key`，别的密钥项
    /// （「配置页」第 48 条）照它自己的键。
    secrets: BTreeMap<String, String>,
    /// 新加的供应商（编号），照加的先后。
    pub new_providers: Vec<String>,
    /// 新加的模型（供应商编号、模型名），照加的先后。
    pub new_models: Vec<(String, String)>,
    /// 新建的池，照建的先后。
    pub new_pools: Vec<String>,
    /// 供应商改编号（旧、新）：别的改动照旧编号存完再改（`provider.rename`，`saving.rs`）。
    pub rename: Option<(String, String)>,
}

impl Draft {
    /// 改成 `value`；和系统配置现在写的一样的不算改动。
    pub fn set(&mut self, key: &str, value: Value, data: &Data) {
        if data.system.get(key) == Some(&value) {
            self.changes.remove(key);
        } else {
            self.changes.insert(key.to_string(), Change::Set(value));
        }
    }

    /// 从系统配置删掉；本来就没写的不算改动。
    pub fn unset(&mut self, key: &str, data: &Data) {
        if data.system.contains_key(key) {
            self.changes.insert(key.to_string(), Change::Unset);
        } else {
            self.changes.remove(key);
        }
    }

    /// 删掉系统配置里这张表下面写的每一项（删供应商、模型、池），连同草稿里新加的。
    pub fn unset_all(&mut self, prefix: &str, data: &Data) {
        self.changes.retain(|k, _| !keys::under(k, prefix));
        for key in data.written_under(prefix) {
            self.changes.insert(key, Change::Unset);
        }
        if let Some(id) = prefix
            .strip_prefix("providers.")
            .filter(|r| !r.contains('.'))
        {
            self.secrets.remove(&provider_key(id));
            self.new_providers.retain(|p| p != id);
            self.new_models.retain(|(p, _)| p != id);
        }
        self.new_models
            .retain(|(p, m)| keys::model_table(p, m) != prefix);
        self.new_pools.retain(|p| keys::pool(p) != prefix);
    }

    /// 新贴了一家供应商的 key：存的时候先 `secret.set`，再把引用写进 `key`（一家一个，核心 8-25）。
    pub fn set_secret(&mut self, provider: &str, value: String) {
        self.set_secret_at(&provider_key(provider), value);
    }

    /// 新贴了一个密钥，引用写进 `key` 这一项（「配置页」第 48 条）：存的时候先 `secret.set`。
    pub fn set_secret_at(&mut self, key: &str, value: String) {
        self.changes.remove(key);
        self.secrets.insert(key.to_string(), value);
    }

    /// 草稿里这一项是什么：写了的值，删了的 `None`，没改的照系统配置。
    pub fn value<'a>(&'a self, key: &str, data: &'a Data) -> Option<&'a Value> {
        match self.changes.get(key) {
            Some(Change::Set(v)) => Some(v),
            Some(Change::Unset) => None,
            None => data.system.get(key),
        }
    }

    /// 草稿里改过这一项。
    pub fn changed(&self, key: &str) -> bool {
        self.changes.contains_key(key)
    }

    /// 草稿把这张表在系统配置写的全删了（原来写了东西的才算）。
    pub fn removes(&self, prefix: &str, data: &Data) -> bool {
        let written = data.written_under(prefix);
        !written.is_empty()
            && written
                .iter()
                .all(|k| self.changes.get(k) == Some(&Change::Unset))
            && !self
                .changes
                .iter()
                .any(|(k, c)| keys::under(k, prefix) && matches!(c, Change::Set(_)))
    }

    /// 什么都没改。
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
            && self.secrets.is_empty()
            && self.new_providers.is_empty()
            && self.new_pools.is_empty()
            && self.rename.is_none()
    }

    /// 只改了供应商编号，别的什么都没动（`saving.rs` 直接改编号，不发空的 `config.set`）。
    pub fn only_renames(&self) -> bool {
        self.rename.is_some()
            && self.changes.is_empty()
            && self.secrets.is_empty()
            && self.new_providers.is_empty()
            && self.new_pools.is_empty()
    }

    /// 全扔掉。
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 这一份照 `named`（已经存进密钥库、引用写进这一次 `config.set` 的）发出去的样子：还没存的 key 不算发出去了。
    pub fn sending(&self, named: &[(String, String)]) -> Draft {
        let mut sent = self.clone();
        sent.secrets
            .retain(|key, _| named.iter().any(|(k, _)| k == key));
        sent
    }

    /// 只带一次改供应商编号的草稿（发出去以后记着，回话时照它算还剩什么）。
    pub fn renaming(rename: Option<(String, String)>) -> Draft {
        Draft {
            rename,
            ..Draft::default()
        }
    }

    /// 发出去 `sent` 以后又改的（2026-11-28 审计第 3 条：保存在途时又改的一项原来跟着回话一起清掉，从没发出去）。`old`
    /// 是发 `sent` 时读来的；`base` 是核心那头现在存着的：存成了的是 `old` 叠上 `sent`，没存成的还是 `old`。交回照 `base`
    /// 还要改的：人现在要的和 `sent` 发的不一样的那几项（改回原来的值也算），连同之后才贴的 key、才加的供应商、模型、池。
    pub fn since(&self, sent: &Draft, old: &Data, base: &Data) -> Draft {
        let mut left = Draft::default();
        let touched: BTreeSet<&String> = self.changes.keys().chain(sent.changes.keys()).collect();
        for key in touched {
            let want = self.value(key, old);
            if want == sent.value(key, old) {
                continue;
            }
            match want {
                Some(value) => left.set(key, value.clone(), base),
                None => left.unset(key, base),
            }
        }
        left.secrets = self
            .secrets
            .iter()
            .filter(|(key, value)| sent.secrets.get(*key) != Some(value))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        left.new_providers = later(&self.new_providers, &sent.new_providers);
        left.new_models = later(&self.new_models, &sent.new_models);
        left.new_pools = later(&self.new_pools, &sent.new_pools);
        left.rename = self
            .rename
            .clone()
            .filter(|r| sent.rename.as_ref() != Some(r));
        left
    }

    /// 草稿里写成值的每一项（拼出界面上看的那一份用）。
    pub fn sets(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.changes.iter().filter_map(|(k, c)| match c {
            Change::Set(v) => Some((k.as_str(), v)),
            Change::Unset => None,
        })
    }

    /// 这一家贴了新的 key、还没存。
    pub fn has_secret(&self, provider: &str) -> bool {
        self.secrets.contains_key(&provider_key(provider))
    }

    /// 存下以后要重新取模型列表的供应商：新加的、换了地址、接口、key 的；删掉的不算。
    pub fn reconnects(&self, data: &Data) -> Vec<String> {
        let mut ids: Vec<String> = self.new_providers.clone();
        ids.extend(self.secrets.keys().filter_map(|key| {
            let parts = split(key);
            (parts.len() == 3 && parts[0] == "providers" && parts[2] == "key")
                .then(|| parts[1].trim_matches('"').to_string())
        }));
        for (key, change) in &self.changes {
            let parts = split(key);
            if matches!(change, Change::Set(_))
                && parts.len() == 3
                && parts[0] == "providers"
                && matches!(parts[2].as_str(), "base_url" | "driver" | "key")
            {
                ids.push(parts[1].trim_matches('"').to_string());
            }
        }
        ids.sort();
        ids.dedup();
        ids.retain(|id| !self.removes(&keys::provider(id), data));
        ids
    }

    /// 要存进密钥库的：写引用的那一项、起好的名字（不撞已有的、不互相撞）、明文。`now` 是起名用的时刻（毫秒）。
    pub fn secrets(&self, data: &Data, now: u64) -> Vec<(String, String, String)> {
        let mut taken = data.secrets.clone();
        self.secrets
            .iter()
            .map(|(key, value)| {
                let name = secret_name(key, now, &taken);
                taken.insert(name.clone());
                (key.clone(), name, value.clone())
            })
            .collect()
    }

    /// 要发的 `config.set` 参数：不写 `layer`（就是系统配置），每项带 `expect`（原来写的什么）；`named` 是存好了的密钥
    /// （写引用的那一项、名字），引用写进那一项。
    pub fn request(&self, data: &Data, named: &[(String, String)]) -> Value {
        let mut all = self.changes.clone();
        for (key, name) in named {
            let reference = json!({"secret": name});
            // 记录列表一条里的密钥字段（「配置页」第 47b 条）：引用写进那一条。
            if let Some((base, i, field)) = crate::settings::pages::records::record_path(key) {
                if let Some(Change::Set(Value::Array(list))) = all.get_mut(&base)
                    && let Some(Value::Object(record)) = list.get_mut(i)
                {
                    record.insert(field, reference);
                }
                continue;
            }
            all.insert(key.clone(), Change::Set(reference));
        }
        let changes: Vec<Value> = all
            .iter()
            .map(|(key, change)| {
                let expect = data
                    .system
                    .get(key)
                    .map_or_else(|| json!({}), |v| json!({"value": v}));
                match change {
                    Change::Set(value) => json!({"key": key, "value": value, "expect": expect}),
                    Change::Unset => json!({"key": key, "unset": true, "expect": expect}),
                }
            })
            .collect();
        json!({"changes": changes})
    }
}

/// `now` 里 `before` 没有的，照原来的先后。
fn later<T: Clone + PartialEq>(now: &[T], before: &[T]) -> Vec<T> {
    now.iter()
        .filter(|x| !before.contains(x))
        .cloned()
        .collect()
}

/// 照 `.` 分段，引号里的点不算。
fn split(key: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut quoted = false;
    let mut escaped = false;
    for c in key.chars() {
        let last = parts.last_mut().expect("至少一段");
        match c {
            _ if escaped => {
                last.push(c);
                escaped = false;
            }
            '\\' if quoted => {
                last.push(c);
                escaped = true;
            }
            '"' => {
                last.push(c);
                quoted = !quoted;
            }
            '.' if !quoted => parts.push(String::new()),
            _ => last.push(c),
        }
    }
    parts
}

/// 一家供应商的 key 那一项：`providers.<编号>.key`。
fn provider_key(provider: &str) -> String {
    format!("{}.key", keys::provider(provider))
}

/// 密钥的名字（名字最长 64 个字符，`config.md`「名字的写法」）：供应商的 key 照旧 `<编号>-key`；别的密钥项照网页设置页的
/// 起法（`miyu-web` 的 `secretName`）：键转小写、不是字母数字的换成 `-`，接 `-` 和这一刻毫秒数的 36 进制。撞了接 `-2`、`-3`。
fn secret_name(key: &str, now: u64, taken: &BTreeSet<String>) -> String {
    let parts = split(key);
    let base: String = if parts.len() == 3 && parts[0] == "providers" && parts[2] == "key" {
        format!("{}-key", parts[1].trim_matches('"'))
    } else {
        format!("{}-{}", slug(key), base36(now))
    }
    .chars()
    .take(56)
    .collect();
    if !taken.contains(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|name| !taken.contains(name))
        .expect("总有一个没用过的")
}

/// 键转小写，连着的不是字母数字的换成一个 `-`（同网页的 `replace(/[^a-z0-9]+/g, '-')`）。
fn slug(key: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in key.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('-');
            gap = true;
        }
    }
    out
}

/// 十进制换成 36 进制（小写，同 JavaScript 的 `toString(36)`）。
fn base36(mut n: u64) -> String {
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    loop {
        out.push(digits[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests;
