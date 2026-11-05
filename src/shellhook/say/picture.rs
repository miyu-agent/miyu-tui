//! shell 里画她展示的图（排期第 33 条，2026-11-03 项目主人：「直接在 shell 里画出图」；图纸 `shellhook.md`「怎么印」第 3b 条）：
//! 终端是 kitty（herdr 里也算，herdr 原样转 kitty 图形协议、图留在它的滚动历史里，2026-11-03 实测）时，展示的图读到本地以后，
//! 那一条排成「记号那一行 + 垫着的空行」；活动区里记号那一行写「\[图片\] 标题」，定下来印进滚动区的那一下换成 kitty 图形协议
//! （`a=T`，PNG，光标不动），图盖在垫着的空行上，跟着字滚。画不了的（不是 kitty、读不成、画不成）交回 `None`，照原来写一行。

use std::collections::HashMap;
use std::path::Path;

use base64::Engine;
use ratatui::text::{Line, Span};

/// 记号那一行打头的字（私用区，正文里不会有）：后面接 blob 的哈希。
const MARK: char = '\u{F0E33}';

/// 一次 kitty 图形协议的命令里 base64 最多几个字节（kitty 的规矩：分块，每块最多 4096）。
const CHUNK: usize = 4096;

/// 做好的一张：印进滚动区的字、占几列几行；做不成的记下，不再试。
enum Made {
    Ready {
        apc: String,
        rows: u16,
        label: String,
    },
    Failed,
}

/// 这一回 shell 里的图。
pub struct Pictures {
    /// 终端认 kitty 图形协议。
    kitty: bool,
    /// 一格多少像素（宽、高）。
    cell: (u32, u32),
    made: HashMap<String, Made>,
}

impl Pictures {
    /// 照环境变量认 kitty（`KITTY_WINDOW_ID`、`TERM=xterm-kitty`；herdr 里照样带着外面 kitty 的），不问终端：问不到要等，
    /// 每句话起来都等一下不值。`cell` 是一格多少像素（宽、高），问不到的照 8×16。
    pub fn detect(var: impl Fn(&str) -> Option<String>, cell: Option<(u32, u32)>) -> Self {
        let kitty = var("KITTY_WINDOW_ID").is_some_and(|v| !v.is_empty())
            || var("TERM").as_deref() == Some("xterm-kitty");
        Self {
            kitty,
            cell: cell.unwrap_or((8, 16)),
            made: HashMap::new(),
        }
    }

    /// 一格多少像素：问窗口大小（`TIOCGWINSZ` 的像素那两格）；终端不报像素的是 `None`。
    pub fn cell() -> Option<(u32, u32)> {
        let size = ratatui::crossterm::terminal::window_size().ok()?;
        (size.columns > 0 && size.rows > 0 && size.width > 0 && size.height > 0).then(|| {
            (
                u32::from(size.width / size.columns).max(1),
                u32::from(size.height / size.rows).max(1),
            )
        })
    }

    /// 画不了图的（测试、不是 kitty 的终端）。
    pub fn none() -> Self {
        Self {
            kitty: false,
            cell: (8, 16),
            made: HashMap::new(),
        }
    }

    /// `blob` 那张图（本地在 `path`）排成的行：记号那一行（行首 `indent` 列空着），下面垫空行到图的高度。最宽 `width` 列，
    /// 最高 `max_rows` 行。画不了的是 `None`。`label` 是活动区里记号那一行写的。
    pub fn lines(
        &mut self,
        (blob, path): (&str, &Path),
        (indent, width, max_rows): (u16, u16, u16),
        label: &str,
    ) -> Option<Vec<Line<'static>>> {
        if !self.kitty {
            return None;
        }
        let cell = self.cell;
        let made = self.made.entry(blob.to_string()).or_insert_with(|| {
            make(path, cell, (indent, width, max_rows), label).unwrap_or(Made::Failed)
        });
        let Made::Ready { rows, .. } = made else {
            return None;
        };
        let mut out = vec![Line::from(Span::raw(format!("{MARK}{blob}")))];
        out.extend((1..*rows).map(|_| Line::default()));
        Some(out)
    }

    /// 活动区：记号那一行换成「\[图片\] 标题」（还会变的不印图：活动区每一帧擦了重画，图会留下残影）。
    pub fn live(&self, lines: &[Line<'static>]) -> Vec<Line<'static>> {
        lines
            .iter()
            .map(|line| match self.ready(line) {
                Some(Made::Ready { label, .. }) => {
                    Line::from(Span::styled(label.clone(), crate::theme::md_image()))
                }
                _ => line.clone(),
            })
            .collect()
    }

    /// 定下来印进滚动区的几行（`lines` 和画好的 `strings` 一一对应）：记号那一行换成图。
    pub fn settle(&self, lines: &[Line<'static>], strings: &mut [String]) {
        for (line, out) in lines.iter().zip(strings.iter_mut()) {
            if let Some(Made::Ready { apc, .. }) = self.ready(line) {
                out.clone_from(apc);
            }
        }
    }

    /// 记号那一行对应的那一张。
    fn ready(&self, line: &Line<'static>) -> Option<&Made> {
        let [span] = line.spans.as_slice() else {
            return None;
        };
        let blob = span.content.strip_prefix(MARK)?;
        self.made.get(blob)
    }
}

/// 读图、缩到放得下、编成 PNG，写成 kitty 图形协议（分块，`C=1` 光标不动，`c`、`r` 占几列几行），行首空 `indent` 列。
fn make(
    path: &Path,
    cell: (u32, u32),
    (indent, width, max_rows): (u16, u16, u16),
    label: &str,
) -> Option<Made> {
    // 读回来存下的文件可能没有扩展名：照内容认。
    let picture = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let (w, h) = (picture.width().max(1), picture.height().max(1));
    let room = u32::from(width.saturating_sub(indent)).max(1);
    let mut cols = (w.div_ceil(cell.0)).clamp(1, room);
    let mut rows = (u64::from(cols * cell.0) * u64::from(h) / u64::from(w))
        .div_ceil(u64::from(cell.1))
        .max(1);
    let max_rows = u64::from(max_rows.max(1));
    if rows > max_rows {
        rows = max_rows;
        cols = u32::try_from(rows * u64::from(cell.1) * u64::from(w) / u64::from(h))
            .unwrap_or(1)
            .div_ceil(cell.0)
            .clamp(1, room);
    }
    let fit = picture.thumbnail(cols * cell.0, u32::try_from(rows).ok()? * cell.1);
    let mut png = Vec::new();
    fit.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    let rows = u16::try_from(rows).ok()?;
    let apc = apc(&png, u16::try_from(cols).ok()?, rows);
    Some(Made::Ready {
        apc: format!("{}{apc}", " ".repeat(usize::from(indent))),
        rows,
        label: format!("{}{label}", " ".repeat(usize::from(indent))),
    })
}

/// PNG 写成 kitty 图形协议：一次传完就放（`a=T`），每块最多 [`CHUNK`] 字节，后面还有的 `m=1`。
fn apc(png: &[u8], cols: u16, rows: u16) -> String {
    let data = base64::engine::general_purpose::STANDARD.encode(png);
    let chunks: Vec<&str> = data
        .as_bytes()
        .chunks(CHUNK)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect();
    let mut out = String::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            out.push_str(&format!(
                "\u{1b}_Ga=T,f=100,q=2,C=1,c={cols},r={rows},m={more};{chunk}\u{1b}\\"
            ));
        } else {
            out.push_str(&format!("\u{1b}_Gm={more};{chunk}\u{1b}\\"));
        }
    }
    out
}

#[cfg(test)]
mod tests;
