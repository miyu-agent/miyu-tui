//! 她给人看的文件（核心 D-5 `show`，蓝图 `tui.md`「展示」第 3–5 条）：图片画出来（同她发的图），别的一张卡片（种类图标、
//! 文件名、「种类 · 大小 · 几行」、「点击打开」）；同一个路径后来又展示过的照常画，最后一行换成「已更新」。

use ratatui::style::Modifier;
use ratatui::text::Span;
use serde::Deserialize;

use super::figure_rows;
use super::rows::{Ctx, Row, clip};
use crate::core::Shown;
use crate::markdown::{Figure, FigureKind};
use crate::theme;
use crate::transcript::Entry;

/// 展示那几行的字（`text/zh.json` 的 `shown`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShownTexts {
    /// 种类的说法：`html`、`pdf`、`markdown`、`text`、`file`。
    pub kinds: std::collections::HashMap<String, String>,
    /// 几行：`{n} 行`。
    pub lines: String,
    /// 卡片最下面那一行。
    pub open: String,
    /// 后来又展示过的：卡片最后一行、图下面那一行（第 5 条）。
    pub updated: String,
    /// 没有能打开它的程序（第 7 条）：`{path}` 是副本的完整路径。
    pub no_program: String,
    /// 点了开不了：副本拷不出来（第 6 条）。
    pub expired: String,
    /// 点了开不了：核心那边读不出这份内容（第 6 条）。
    pub missing: String,
}

/// 正文里这一条排成的行。
pub fn rows(entry: &Entry, ctx: &Ctx) -> Vec<Row> {
    let Some(file) = &entry.shown else {
        return Vec::new();
    };
    let words = &ctx.config.text.shown;
    let replaced = ctx.replaced.contains(&entry.id);
    let mut rows = if file.image() {
        let mut rows = image(file, ctx);
        // 后来同一个路径又展示过的图：下面一行暗色「已更新」（第 5 条）。
        if replaced {
            let text = clip(&words.updated, ctx.width);
            rows.push(ctx.row(ctx.blank_slot(), vec![Span::styled(text, theme::dim())]));
        }
        rows
    } else {
        card(file, entry.unopened.as_deref(), replaced, ctx)
    };
    // 点哪一行都是开它（第 6 条）：挂上展示的地址，点链接那一套认得（`app/shown.rs`）。图那几行只有图本身能点（2026-10-22
    // 项目主人：原来整行都能点，图右边的空档也是手形、点了也开）。
    let link = format!("{}{}", crate::shown_files::SCHEME, entry.id);
    let figures = ctx.figures.borrow();
    for row in &mut rows {
        let picture = row
            .figure
            .as_ref()
            .and_then(|cell| Some((cell.x, cell.x + figures.cols(cell.key)?)));
        let (from, to) = picture.unwrap_or((0, ctx.width));
        row.links = vec![(from, to, link.clone())];
    }
    drop(figures);
    rows
}

/// 卡片、缩成的一行写的名字：她写了标题的写标题，没写的写文件名。
fn heading(file: &Shown) -> &str {
    file.title.as_deref().unwrap_or(&file.name)
}

/// 读回来、拷成带文件名的副本以后的位置；还没回来的是 `None`（第 6 条，`shown_files.rs`）。
fn local(file: &Shown, ctx: &Ctx) -> Option<std::path::PathBuf> {
    let saved = ctx.cards.borrow_mut().file(&file.blob)?.to_path_buf();
    crate::shown_files::copy(&file.blob, &file.name, &saved)
}

/// 图：同她发的图，没回来是「正在渲染图片」那一行，显示不了、读不成、画不成的写一行图片色的「\[图片\] 标题」（第 3 条）。
fn image(file: &Shown, ctx: &Ctx) -> Vec<Row> {
    let label = || {
        let text = format!("{} {}", ctx.config.text.sent.image, heading(file));
        let span = Span::styled(clip(&text, ctx.width), theme::md_image());
        vec![ctx.row(ctx.blank_slot(), vec![span])]
    };
    if !ctx.figures.borrow().shows() {
        return label();
    }
    let Some(path) = local(file, ctx) else {
        if ctx.cards.borrow().unreadable(&file.blob) {
            return label();
        }
        return vec![figure_rows::pending(Vec::new(), ctx)];
    };
    let figure = Figure {
        kind: FigureKind::Image,
        source: path.display().to_string(),
        fallback: Vec::new(),
        width: None,
        height: None,
    };
    let rows = figure_rows::rows(Vec::new(), &figure, ctx);
    if rows.is_empty() { label() } else { rows }
}

/// 卡片：标题（加粗）、暗色「种类 · 大小 · 几行」、暗色「点击打开」（第 4 条）；开不了的最后一行换成黄字「没有能打开它的
/// 程序：<副本路径>」（第 7 条），放不下的折行，路径照常能复制。
fn card(file: &Shown, unopened: Option<&str>, replaced: bool, ctx: &Ctx) -> Vec<Row> {
    let words = &ctx.config.text.shown;
    // 种类图标（这一套写了的才画，2026-10-17 项目主人要的）。
    let mut title = Vec::new();
    if let Some(icon) = ctx.config.icons.files.get(kind_key(file)) {
        title.push(Span::styled(format!("{icon} "), theme::dim()));
    }
    title.push(Span::styled(
        clip(heading(file), ctx.width),
        theme::md_link().add_modifier(Modifier::BOLD),
    ));
    // 标题后面暗色写文件名；标题就是文件名的不重复（2026-10-17 真模型走查：她常把文件名当标题写）。
    if file.title.as_deref().is_some_and(|t| t.trim() != file.name) {
        title.push(Span::styled(format!("  {}", file.name), theme::dim()));
    }
    let mut facts = vec![kind(file, words), size(file.size)];
    if let Some(n) = lines(file, ctx) {
        facts.push(words.lines.replace("{n}", &n.to_string()));
    }
    let facts = clip(&facts.join(&crate::sheet::sep()), ctx.width);
    let mut rows: Vec<Row> = [title, vec![Span::styled(facts, theme::dim())]]
        .into_iter()
        .map(|content| ctx.row(ctx.blank_slot(), content))
        .collect();
    match unopened {
        Some(path) => {
            let text = words.no_program.replace("{path}", path);
            for line in crate::cells::wrap(&text, usize::from(ctx.width.max(1))) {
                rows.push(ctx.row(ctx.blank_slot(), vec![Span::styled(line, theme::warn())]));
            }
        }
        None => {
            // 后来又展示过的：最后一行写「已更新」，点它开的是最新那一版（第 5 条）。
            let last = if replaced {
                &words.updated
            } else {
                &words.open
            };
            rows.push(ctx.row(
                ctx.blank_slot(),
                vec![Span::styled(clip(last, ctx.width), theme::dim())],
            ));
        }
    }
    rows
}

/// 种类图标的键：`html`、`pdf`、`markdown`、`code`（认出了语言的文本）、`text`、`file`。
fn kind_key(file: &Shown) -> &'static str {
    match file.media_type.as_str() {
        "text/html" => "html",
        "application/pdf" => "pdf",
        "text/markdown" => "markdown",
        "text/plain" if file.language.is_some() => "code",
        "text/plain" => "text",
        _ => "file",
    }
}

/// 种类写成人话：代码写语言名。
fn kind(file: &Shown, words: &ShownTexts) -> String {
    let word = |key: &str| {
        words
            .kinds
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    };
    match file.media_type.as_str() {
        "text/html" => word("html"),
        "application/pdf" => word("pdf"),
        "text/markdown" => word("markdown"),
        "text/plain" => file.language.clone().unwrap_or_else(|| word("text")),
        _ => word("file"),
    }
}

/// 字节数写成 B、KB、MB（一位小数）。
pub fn size(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let n = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", n / 1024.0)
    } else {
        format!("{:.1} MB", n / 1024.0 / 1024.0)
    }
}

/// 文本类（HTML、Markdown、代码、文本）有几行：读回来以后数，没回来的、别的种类是 `None`。大过 5 MB 的不数。
fn lines(file: &Shown, ctx: &Ctx) -> Option<usize> {
    let texty = matches!(
        file.media_type.as_str(),
        "text/html" | "text/markdown" | "text/plain"
    );
    if !texty || file.size > 5 * 1024 * 1024 {
        return None;
    }
    let path = ctx.cards.borrow_mut().file(&file.blob)?.to_path_buf();
    let text = std::fs::read(path).ok()?;
    let n = text.iter().filter(|&&b| b == b'\n').count();
    Some(if text.last().is_some_and(|&b| b != b'\n') {
        n + 1
    } else {
        n
    })
}

#[cfg(test)]
mod tests;
