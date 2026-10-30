//! 问之前写在预览块里的（蓝图「配置页」第 43c 条）：照看一眼的回应（`preview: true`）列出要装的、一起装的依赖、需要的
//! 能力、下载多大、装好多大、推荐的；卸的写一并删除什么、多大；升级的写旧版到新版、下载多大；批能力的一项一行。

use ratatui::text::{Line, Span};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::preview::name_of;
use crate::drawer::size;
use crate::settings::pages::store::flow::Doing;
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
    let mut extra: Vec<(String, String, bool)> = Vec::new();
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
            let needs: Vec<String> = previews
                .iter()
                .flat_map(|p| list(&p["capabilities"]))
                .filter_map(|c| c["name"].as_str().or(c["id"].as_str()).map(str::to_string))
                .collect();
            let recommends: Vec<String> = previews
                .iter()
                .flat_map(|p| list(&p["recommends"]))
                .map(|r| name_of(store, &text(&r["package"])))
                .collect();
            let total = |key: &str| previews.iter().filter_map(|p| p[key].as_u64()).sum::<u64>();
            extra.push((a("with"), with.join(&sep), false));
            extra.push((
                StoreTexts::say(&texts.keys, "needs"),
                needs.join(&sep),
                true,
            ));
            extra.push((
                StoreTexts::say(&texts.keys, "download"),
                sized(total("download")),
                false,
            ));
            extra.push((
                StoreTexts::say(&texts.keys, "installed_size"),
                sized(total("installed")),
                false,
            ));
            extra.push((a("recommends"), recommends.join(&sep), false));
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
            if p["state"] == Value::Bool(true) {
                drop.push(a("drop_state"));
            }
            extra.push((a("drop"), drop.join(&sep), true));
            extra.push((
                a("size"),
                p["size"].as_u64().map(size).unwrap_or_default(),
                false,
            ));
        }
        Doing::Upgrade(_) => {
            let p = previews.first().cloned().unwrap_or_default();
            let ups: Vec<&Value> = list(&p["upgrades"]).collect();
            out.push(head(a("upgrade").replace("{n}", &ups.len().to_string())));
            for u in ups {
                let shown = v("replace")
                    .replace("{from}", &text(&u["from"]))
                    .replace("{to}", &text(&u["to"]));
                rows.push((name_of(store, &text(&u["package"])), shown));
            }
            let download = p["download"].as_u64().map(size).unwrap_or_default();
            extra.push((StoreTexts::say(&texts.keys, "download"), download, false));
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
        .filter(|(_, v, _)| !v.is_empty())
        .collect();
    let key_w = extra.iter().map(|(k, _, _)| k.width()).max().unwrap_or(0);
    for (k, value, warn) in extra {
        let pad = " ".repeat(key_w - k.width());
        let style = if warn {
            theme::warn()
        } else {
            ratatui::style::Style::new()
        };
        out.push(Line::from(vec![
            Span::styled(format!("{k}{pad} : "), theme::dim()),
            Span::styled(value, style),
        ]));
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
