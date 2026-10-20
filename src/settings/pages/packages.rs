//! 「软件包」那一页先列包、进去一包一页（蓝图「配置页」第 43a 条，2026-10-16 项目主人定）：核心的清单里包的配置项都在
//! `packages` 那一页、一个包一组（组的编号是包的编号）；终端先列出有配置项的包，一行一个（名字、`package.list` 的一句
//! 说明、几项），进去是这个包自己的一页，照画一组的样子画它的项。包自己的那一页当成一页照常画、照常改：页的编号写成
//! `packages/<包>`，[`More::rows`] 照它只取这一组的项。

use std::collections::HashMap;

use serde_json::Value;

use super::More;
use super::schema::Row;

/// 核心放包的配置项的那一页。
pub const PACKAGES: &str = "packages";

/// 包 `id` 自己那一页的编号。
pub fn page_of(id: &str) -> String {
    format!("{PACKAGES}/{id}")
}

/// 是包自己那一页的：交回包的编号。
pub fn package_of(page: &str) -> Option<&str> {
    page.strip_prefix(PACKAGES)?.strip_prefix('/')
}

/// 列表里的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// 包的编号（也是组的编号）。
    pub id: String,
    /// 名字（组名，核心照界面语言给的）。
    pub name: String,
    /// 一句说明（`package.list` 的 `summary`）；没有的是 `None`。
    pub summary: Option<String>,
    /// 有几项。
    pub count: usize,
}

/// 一个包的作者、从哪来、上游（核心 F-12；清单没写的没有）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origin {
    /// 作者。
    pub author: Option<String>,
    /// 从哪来：`shipped`、`official`、`local`。
    pub from: Option<String>,
    /// 上游源码仓库的网址。
    pub upstream: Option<String>,
}

/// 读 `package.list` 的回应：每个包的作者、从哪来、上游。
pub fn origins(got: &Value) -> HashMap<String, Origin> {
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_string);
    got["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = p["package"].as_str()?;
            let origin = Origin {
                author: text(&p["author"]),
                from: text(&p["from"]),
                upstream: text(&p["upstream"]),
            };
            Some((id.to_string(), origin))
        })
        .collect()
}

impl More {
    /// 包页面包屑下面那一行（第 43a 条，2026-10-19 项目主人挑的样子）：「作者 · 从哪来 · 上游」，没有的那样不写，三样都没有
    /// 的是 `None`。上游去掉 `https://` 写。
    pub fn origin_line(
        &self,
        id: &str,
        from_words: &std::collections::BTreeMap<String, String>,
    ) -> Option<String> {
        let origin = self.origins.get(id)?;
        let parts: Vec<String> = [
            origin.author.clone(),
            origin
                .from
                .as_ref()
                .map(|f| from_words.get(f).cloned().unwrap_or_else(|| f.clone())),
            origin
                .upstream
                .as_ref()
                .map(|u| u.trim_start_matches("https://").to_string()),
        ]
        .into_iter()
        .flatten()
        .collect();
        (!parts.is_empty()).then(|| parts.join(" · "))
    }
}

impl More {
    /// 包 `id` 的上游完整网址：那一行末尾的上游点了照它打开（第 43a 条）。
    pub fn upstream(&self, id: &str) -> Option<&str> {
        self.origins.get(id)?.upstream.as_deref()
    }
}

/// 读 `package.list` 的回应：每个包的一句说明。
pub fn summaries(got: &Value) -> HashMap<String, String> {
    got["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| {
            let id = p["package"].as_str()?;
            let summary = p["summary"].as_str().filter(|s| !s.is_empty())?;
            Some((id.to_string(), summary.to_string()))
        })
        .collect()
}

/// 一项属于哪个包：核心写了 `package` 的照它，没写的照组的编号（原来一个包一组，组的编号就是包的编号）。
fn owner(item: &super::schema::Item) -> &str {
    if item.package.is_empty() {
        &item.group
    } else {
        &item.package
    }
}

impl More {
    /// 有配置项的包，照核心交回的先后。一个包可以有几组（2026-10-19 核心把联网搜索的 key 分出「API Key 和地址」一组，
    /// 项都还带 `package: "net"`）：照项的包合成一行，名字照和包同编号的那一组，项数连几组一起数。
    pub fn packages(&self) -> Vec<Listed> {
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<Listed> = Vec::new();
        for group in schema.groups.iter().filter(|g| g.page == PACKAGES) {
            let Some(first) = schema
                .items
                .iter()
                .find(|i| i.page == PACKAGES && i.group == group.id)
            else {
                continue;
            };
            let id = owner(first).to_string();
            if out.iter().any(|l| l.id == id) {
                continue;
            }
            let name = schema
                .groups
                .iter()
                .find(|g| g.page == PACKAGES && g.id == id)
                .unwrap_or(group)
                .name
                .clone();
            let count = schema
                .rows(PACKAGES)
                .iter()
                .filter(|r| matches!(r, Row::Item(i) if owner(&schema.items[*i]) == id))
                .count();
            out.push(Listed {
                summary: self.summaries.get(&id).cloned(),
                id,
                name,
                count,
            });
        }
        out
    }

    /// 包 `id` 那一页的行：属于它的项；和包同编号的那一组不带组名（标题已经写了包名），别的组照画组名（上面空一行）。
    pub(super) fn package_rows(&self, id: &str) -> Vec<Row> {
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        let mine = |g: usize| {
            let group = &schema.groups[g];
            group.id != id
                && schema
                    .items
                    .iter()
                    .any(|i| i.page == PACKAGES && i.group == group.id && owner(i) == id)
        };
        schema
            .rows(PACKAGES)
            .into_iter()
            .filter(|r| match r {
                Row::Item(i) => owner(&schema.items[*i]) == id,
                Row::Group(g) => mine(*g),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
