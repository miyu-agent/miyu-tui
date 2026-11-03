//! 预览块里写什么（蓝图「配置页」第 43c 条）：平常照 pacman 的 `-Si`、`-Qi` 写「键 : 值」，跟着光标换；问的时候写要做的
//! 和「继续？」，做的时候写进度。只交回一行行字，画在 `mod.rs`。

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::core::{Progress, Stage};
use crate::drawer::size;
use crate::settings::pages::store::flow::Doing;
use crate::settings::pages::store::{Flow, Info, Item, Local, Remote, Store, StoreTexts, Tab};
use crate::theme;

/// 一行「键 : 值」：键、值、值的样子。
type Pair = (String, String, Style);

/// 平常的预览：光标停的那一个。`from` 是「来源」的几种叫法（`settings.more.package_from`）。
pub fn lines(
    store: &Store,
    texts: &StoreTexts,
    from: &std::collections::BTreeMap<String, String>,
) -> Vec<Line<'static>> {
    if let Some(flow) = &store.flow {
        return flowing(store, flow, texts);
    }
    let Some(item) = store.current() else {
        return Vec::new();
    };
    let info = store
        .infos
        .get(&(store.tab == Tab::Source, item.id().to_string()));
    let pairs = match (store.tab, item) {
        (Tab::Source, Item::Remote(p)) => remote(store, p, info, texts),
        (Tab::Installed, Item::Local(p)) => local(store, p, info, texts, from),
        (_, item) => {
            let summary = match item {
                Item::Local(p) => p.summary.clone(),
                Item::Remote(p) => p.summary.clone(),
            };
            let k = |key: &str| StoreTexts::say(&texts.keys, key);
            vec![
                (k("name"), item.name().to_string(), Style::new()),
                (k("summary"), summary, Style::new()),
            ]
        }
    };
    table(pairs)
}

/// 「键 : 值」排成行：键暗色、照最长的对齐；值空的那一行不写。
fn table(pairs: Vec<Pair>) -> Vec<Line<'static>> {
    let pairs: Vec<Pair> = pairs
        .into_iter()
        .filter(|(_, v, _)| !v.is_empty())
        .collect();
    let w = pairs.iter().map(|(k, _, _)| k.width()).max().unwrap_or(0);
    pairs
        .into_iter()
        .map(|(k, v, style)| {
            let pad = " ".repeat(w - k.width());
            Line::from(vec![
                Span::styled(format!("{k}{pad}"), theme::dim()),
                Span::styled(" : ", theme::dim()),
                Span::styled(v, style),
            ])
        })
        .collect()
}

/// 软件源里的一个：名字、版本、说明、作者、上游、软件源、依赖、包含、需要、下载大小、安装后大小、发布时间、下载量。
fn remote(store: &Store, p: &Remote, info: Option<&Info>, texts: &StoreTexts) -> Vec<Pair> {
    let k = |key: &str| StoreTexts::say(&texts.keys, key);
    let v = |key: &str| StoreTexts::say(&texts.values, key);
    let plain = Style::new();
    let mut version = p.version.clone();
    if let Some(had) = p.installed.as_ref().filter(|had| **had != p.version) {
        version.push_str(&v("installed_as").replace("{v}", had));
    }
    let downloads = p.downloads.map(|n| match p.week {
        Some(w) => v("downloads")
            .replace("{n}", &grouped(n))
            .replace("{w}", &grouped(w)),
        None => grouped(n),
    });
    let source = (p.source != "official").then(|| p.source.clone());
    let info = info.cloned().unwrap_or_default();
    vec![
        (k("name"), p.name.clone(), plain),
        (k("version"), version, plain),
        (k("summary"), p.summary.clone(), plain),
        (k("author"), info.author.unwrap_or_default(), plain),
        (
            k("upstream"),
            info.upstream.unwrap_or_default(),
            theme::md_url(),
        ),
        (k("source"), source.unwrap_or_default(), plain),
        (k("depends"), names(store, &p.depends, texts), plain),
        (k("provides"), provides(&p.provides, texts), plain),
        (k("needs"), info.needs.join(&v("sep")), theme::warn()),
        (
            k("download"),
            info.download.map(size).unwrap_or_default(),
            plain,
        ),
        (
            k("installed_size"),
            info.size.map(size).unwrap_or_default(),
            plain,
        ),
        (k("published"), date(p.published.as_deref()), plain),
        (k("downloads"), downloads.unwrap_or_default(), plain),
    ]
}

/// 装着的一个：名字、版本、说明、作者、上游、来源、安装时间、安装后大小、依赖、被依赖、包含、需要、设置。
fn local(
    store: &Store,
    p: &Local,
    info: Option<&Info>,
    texts: &StoreTexts,
    from: &std::collections::BTreeMap<String, String>,
) -> Vec<Pair> {
    let k = |key: &str| StoreTexts::say(&texts.keys, key);
    let v = |key: &str| StoreTexts::say(&texts.values, key);
    let plain = Style::new();
    let info = info.cloned().unwrap_or_default();
    let (version, style) = match store.upgrades.get(&p.id) {
        Some(to) => {
            let up = StoreTexts::say(&texts.states, "upgrade_to").replace("{to}", to);
            (format!("{}  {up}", p.version), theme::accent())
        }
        None => (p.version.clone(), plain),
    };
    let came = info
        .from
        .as_ref()
        .map(|f| from.get(f).cloned().unwrap_or_else(|| f.clone()));
    vec![
        (k("name"), p.name.clone(), plain),
        (k("version"), version, style),
        (k("summary"), p.summary.clone(), plain),
        (k("author"), info.author.unwrap_or_default(), plain),
        (
            k("upstream"),
            info.upstream.unwrap_or_default(),
            theme::md_url(),
        ),
        (k("from"), came.unwrap_or_default(), plain),
        (k("installed"), date(info.installed.as_deref()), plain),
        (
            k("installed_size"),
            info.size.map(size).unwrap_or_default(),
            plain,
        ),
        (k("depends"), names(store, &info.depends, texts), plain),
        (
            k("required_by"),
            names(store, &info.required_by, texts),
            plain,
        ),
        (k("provides"), provides(&info.provides, texts), plain),
        (k("needs"), info.needs.join(&v("sep")), theme::warn()),
    ]
}

/// 几个编号照名字写（装着的、软件源里的找得到名字的），用顿号接。
fn names(store: &Store, ids: &[String], texts: &StoreTexts) -> String {
    let name = |id: &String| {
        store
            .local(id)
            .map(|p| p.name.clone())
            .or_else(|| store.remote(id).map(|p| p.name.clone()))
            .unwrap_or_else(|| id.clone())
    };
    let all: Vec<String> = ids.iter().map(name).collect();
    all.join(&StoreTexts::say(&texts.values, "sep"))
}

/// 「包含」：种类的叫法接名字（「技能 forecast、脚本 radar」），没有名字的只写种类。
fn provides(list: &[(String, Vec<String>)], texts: &StoreTexts) -> String {
    let all: Vec<String> = list
        .iter()
        .map(|(kind, names)| {
            let word = texts
                .provides
                .get(kind)
                .cloned()
                .unwrap_or_else(|| kind.clone());
            // 程序只写它是哪一种（「内置功能」「扩展程序」），不写核心的种类词。
            if kind == "program" {
                let known = names.first().and_then(|n| texts.programs.get(n));
                return known.cloned().unwrap_or(word);
            }
            if names.is_empty() {
                word
            } else {
                format!("{word} {}", names.join(" "))
            }
        })
        .collect();
    all.join(&StoreTexts::say(&texts.values, "sep"))
}

/// 时刻只写日期（`2026-10-31T09:00:00Z` 写成 `2026-10-31`）。
fn date(at: Option<&str>) -> String {
    at.map(|t| t.get(..10).unwrap_or(t).to_string())
        .unwrap_or_default()
}

/// 数照千位加逗号（「1,284」）。
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 问的时候、做的时候。
fn flowing(store: &Store, flow: &Flow, texts: &StoreTexts) -> Vec<Line<'static>> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    match flow {
        Flow::Previewing { .. } => vec![Line::styled(
            StoreTexts::say(&texts.status, "previewing"),
            theme::dim(),
        )],
        Flow::Confirm { doing, previews } => {
            let mut out = super::asking::lines(store, doing, previews, texts);
            out.push(Line::default());
            out.push(Line::from(vec![Span::styled(a("go"), theme::accent())]));
            out
        }
        Flow::Running {
            doing, progress, ..
        } => running(store, doing, progress, texts),
    }
}

/// 做的时候：到了哪一段，一个包一行进度条（下载照字节数，核对、装的写满）。
fn running(
    store: &Store,
    doing: &Doing,
    progress: &[Progress],
    texts: &StoreTexts,
) -> Vec<Line<'static>> {
    let a = |key: &str| StoreTexts::say(&texts.ask, key);
    let stage = progress.iter().map(|p| p.stage).max_by_key(|s| match s {
        Stage::Download => 0,
        Stage::Verify => 1,
        Stage::Install => 2,
    });
    let head = match (doing, stage) {
        (_, Some(Stage::Download)) => a("downloading"),
        (_, Some(Stage::Verify)) => a("verifying"),
        _ => a("installing"),
    };
    let mut out = vec![Line::styled(head, theme::dim())];
    let name_w = progress
        .iter()
        .map(|p| name_of(store, &p.package).width())
        .max()
        .unwrap_or(0);
    for p in progress {
        let fraction = match p.stage {
            Stage::Download if p.total > 0 => p.done as f64 / p.total as f64,
            Stage::Download => 0.0,
            _ => 1.0,
        };
        let filled = (fraction.clamp(0.0, 1.0) * 24.0).round() as usize;
        let (block, rule) = crate::sheet::read(|s| (s.marks.block.clone(), s.marks.rule.clone()));
        let name = name_of(store, &p.package);
        let pad = " ".repeat(name_w - name.width());
        out.push(Line::from(vec![
            Span::raw(format!("{name}{pad}  ")),
            Span::styled(block.repeat(filled), theme::accent()),
            Span::styled(rule.repeat(24 - filled), theme::faint()),
            Span::styled(
                format!(" {:>3}%", (fraction * 100.0).round() as u64),
                theme::dim(),
            ),
        ]));
    }
    out
}

/// 编号照名字写：软件源里的、装着的找得到的。
pub(super) fn name_of(store: &Store, id: &str) -> String {
    store
        .remote(id)
        .map(|p| p.name.clone())
        .or_else(|| store.local(id).map(|p| p.name.clone()))
        .unwrap_or_else(|| id.to_string())
}
