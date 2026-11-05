//! 问之前写在预览块里的（蓝图「配置页」第 43c 条）：照看一眼的回应（`preview: true`）列出要装的、一起装的依赖、需要的
//! 能力、下载多大、装好多大、推荐的；卸的写一并删除什么、配置目录留着还是删、多大；升级的写旧版到新版、下载多大；批能力的一项一行。
//! 装、升级的另写来源、能力一项一行、核心给的「注意」（核心 PK-3 上，`caps.rs`）。

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::caps;
use super::preview::name_of;
use crate::drawer::size;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::remove::{self, Button};
use crate::settings::pages::store::{Store, StoreTexts};
use crate::theme;

/// 问的那几行（不带最后的「继续？」）。
pub fn lines(
    store: &Store,
    doing: &Doing,
    previews: &[Value],
    texts: &StoreTexts,
) -> Vec<Line<'static>> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    let v = |key: &str| StoreTexts::say(&texts.values, key);
    let sep = v("sep");
    let mut out = Vec::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    // 键、一行行的值：多数一行，能力一项一行，第二行起和第一行的值对齐。
    let mut extra: Vec<(String, Vec<(String, Style)>)> = Vec::new();
    let one = |k: String, v: String, style: Style| (k, vec![(v, style)]);
    match doing {
        Doing::Install(_) => {
            out.push(head(
                a("install").replace("{n}", &previews.len().to_string()),
            ));
            for p in previews {
                let id = p["package"].as_str().unwrap_or_default();
                let version = text(&p["version"]);
                let shown = match p["replaces"]["version"].as_str() {
                    Some(old) => v("replace")
                        .replace("{from}", old)
                        .replace("{to}", &version),
                    None => version,
                };
                rows.push((name_of(store, id), shown));
            }
            let with: Vec<String> = previews
                .iter()
                .flat_map(|p| list(&p["with"]))
                .map(|w| {
                    format!(
                        "{} {}",
                        name_of(store, &text(&w["package"])),
                        text(&w["version"])
                    )
                })
                .collect();
            let items: Vec<&Value> = previews
                .iter()
                .chain(previews.iter().flat_map(|p| list(&p["with"])))
                .collect();
            let recommends: Vec<String> = previews
                .iter()
                .flat_map(|p| list(&p["recommends"]))
                .map(|r| name_of(store, &text(&r["package"])))
                .collect();
            let total = |key: &str| previews.iter().filter_map(|p| p[key].as_u64()).sum::<u64>();
            extra.push(one(a("from"), caps::sources(&items, texts), Style::new()));
            // 「需要」排在依赖前面：依赖一长折成几行，也挤不走要批的能力（发布前第五批审查 T5-1）。
            extra.push((
                StoreTexts::say(&texts.keys, "needs"),
                caps::capabilities(store, &items, false, texts),
            ));
            extra.push(one(a("with"), with.join(&sep), Style::new()));
            extra.push(one(
                StoreTexts::say(&texts.keys, "download"),
                sized(total("download")),
                Style::new(),
            ));
            extra.push(one(
                StoreTexts::say(&texts.keys, "installed_size"),
                sized(total("installed")),
                Style::new(),
            ));
            extra.push(one(a("recommends"), recommends.join(&sep), Style::new()));
        }
        Doing::Remove(id) => {
            let p = previews.first().cloned().unwrap_or_default();
            // 没写版本的（出厂包）只写名字，不留空格。
            let version = text(&p["version"]);
            let name = name_of(store, id);
            let label = if version.is_empty() {
                name
            } else {
                format!("{name} {version}")
            };
            let line = a("remove").replace("{name}", &label);
            out.push(head(line));
            let mut drop = Vec::new();
            let settings = list(&p["settings"]).count();
            if settings > 0 {
                drop.push(a("drop_settings").replace("{n}", &settings.to_string()));
            }
            let (plain, with_path) = (a("drop_state"), a("drop_state_path"));
            drop.extend(crate::drawer::state_item(&p["state"], (&plain, &with_path)));
            extra.push(one(a("drop"), drop.join(&sep), theme::warn()));
            // 配置目录、设置项（PK-5、PK-5补）：照停着的按钮写留着还是会删掉。
            let purge = remove::selected(store) == Some(Button::Purge);
            let (kept, purged, settings_kept, settings_purge, join) = (
                a("config_kept"),
                a("config_purge"),
                a("settings_kept"),
                a("settings_purge"),
                a("config_join"),
            );
            let words = crate::drawer::ConfigWords {
                kept: &kept,
                purge: &purged,
                settings_kept: &settings_kept,
                settings_purge: &settings_purge,
                join: &join,
            };
            if let Some(said) = crate::drawer::config_item(&p["config"], Some(purge), words) {
                let style = if purge { theme::warn() } else { Style::new() };
                extra.push(one(a("config"), said, style));
            }
            // 系统安装的（核心 SD-1）：卸了只是不再加载，文件不动。
            if p["layer"] == "distro" {
                extra.push(one(a("note"), a("distro_note"), theme::dim()));
            }
            extra.push(one(
                a("size"),
                p["size"].as_u64().map(size).unwrap_or_default(),
                Style::new(),
            ));
        }
        Doing::Upgrade(_) => {
            let p = previews.first().cloned().unwrap_or_default();
            let ups: Vec<&Value> = list(&p["upgrades"]).collect();
            let with: Vec<&Value> = list(&p["with"]).collect();
            out.push(head(a("upgrade").replace("{n}", &ups.len().to_string())));
            let all: Vec<&Value> = ups.iter().chain(&with).copied().collect();
            extra.push(one(a("from"), caps::sources(&all, texts), Style::new()));
            // 升级只说新要的（批过的不列），一起装的新依赖照装的写「需要」（同核心命令行）。
            extra.push((a("new_caps"), caps::capabilities(store, &ups, true, texts)));
            extra.push((
                StoreTexts::say(&texts.keys, "needs"),
                caps::capabilities(store, &with, false, texts),
            ));
            for u in ups {
                let shown = v("replace")
                    .replace("{from}", &text(&u["from"]))
                    .replace("{to}", &text(&u["to"]));
                rows.push((name_of(store, &text(&u["package"])), shown));
            }
            let download = p["download"].as_u64().map(size).unwrap_or_default();
            extra.push(one(
                StoreTexts::say(&texts.keys, "download"),
                download,
                Style::new(),
            ));
        }
        Doing::Enable(id, caps) => {
            out.push(head(a("approve").replace("{name}", &name_of(store, id))));
            for cap in caps {
                out.push(Line::styled(format!("  {cap}"), theme::warn()));
            }
        }
    }
    let name_w = rows.iter().map(|(n, _)| n.width()).max().unwrap_or(0);
    for (name, version) in rows {
        let pad = " ".repeat(name_w - name.width());
        out.push(Line::from(vec![
            Span::raw(format!("  {name}{pad}  ")),
            Span::styled(version, theme::dim()),
        ]));
    }
    let extra: Vec<_> = extra
        .into_iter()
        .map(|(k, values)| (k, values.into_iter().filter(|(v, _)| !v.is_empty())))
        .map(|(k, values)| (k, values.collect::<Vec<_>>()))
        .filter(|(_, values)| !values.is_empty())
        .collect();
    let key_w = extra.iter().map(|(k, _)| k.width()).max().unwrap_or(0);
    for (k, values) in extra {
        let pad = " ".repeat(key_w - k.width());
        for (n, (value, style)) in values.into_iter().enumerate() {
            let lead = if n == 0 {
                format!("{k}{pad} : ")
            } else {
                " ".repeat(key_w + 3)
            };
            out.push(Line::from(vec![
                Span::styled(lead, theme::dim()),
                Span::styled(value, style),
            ]));
        }
    }
    // 核心给的「注意」（社区的、本机文件的），写在最后、问的那一句上面。
    if matches!(doing, Doing::Install(_) | Doing::Upgrade(_)) {
        out.extend(caps::notice(previews, texts));
    }
    out
}

fn head(text: String) -> Line<'static> {
    Line::raw(text)
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn list(v: &Value) -> impl Iterator<Item = &Value> {
    v.as_array().into_iter().flatten()
}

/// 零字节的不写（回应里没有这一格）。
fn sized(bytes: u64) -> String {
    if bytes == 0 {
        String::new()
    } else {
        size(bytes)
    }
}
