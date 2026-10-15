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

impl More {
    /// 有配置项的包，照核心交回的先后。
    pub fn packages(&self) -> Vec<Listed> {
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        let rows = schema.rows(PACKAGES);
        schema
            .groups
            .iter()
            .filter(|g| g.page == PACKAGES)
            .map(|g| {
                let count = rows
                    .iter()
                    .filter(|r| matches!(r, Row::Item(i) if schema.items[*i].group == g.id))
                    .count();
                Listed {
                    id: g.id.clone(),
                    name: g.name.clone(),
                    summary: self.summaries.get(&g.id).cloned(),
                    count,
                }
            })
            .filter(|l| l.count > 0)
            .collect()
    }

    /// 包 `id` 那一页的行：这一组的项，不带组名（标题已经写了包名）。
    pub(super) fn package_rows(&self, id: &str) -> Vec<Row> {
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        schema
            .rows(PACKAGES)
            .into_iter()
            .filter(|r| matches!(r, Row::Item(i) if schema.items[*i].group == id))
            .collect()
    }
}

#[cfg(test)]
mod tests;
