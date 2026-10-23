//! 模型常写的几样 HTML 标签照 Markdown 的样子画（蓝图 `tui.md`「她的回答：Markdown」第 12 条）：
//! 行内的叠一层样子、成一条链接、转上下标；外壳去掉标签；`<hr>` 是分隔线。没收尾的行内标签到这一块结尾为止。

use std::borrow::Cow;

use ratatui::style::{Modifier, Style};

use super::super::{Container, Renderer};
use crate::theme;

/// 开着的一个行内标签。
pub(in crate::markdown) enum Opened {
    /// 叠了一层样子：那一层的编号。
    Style(u32),
    /// 一条链接（`<a href>`）：链接色那一层的编号。
    Link(u32),
    /// 上标（真）、下标：之后的字照公式的表转。
    Script(bool),
}

/// 一个认得的标签怎么画。
enum Kind {
    /// 叠一层样子。
    Style(Style),
    /// 链接，带着地址。
    Link(String),
    /// 上标（真）、下标。
    Script(bool),
    /// 外壳，自成一块：去掉标签。
    Block,
    /// 行内的外壳：去掉标签，只留字。
    Strip,
    /// 分隔线。
    Rule,
    /// 标题（`<h1>` 到 `<h6>`）：自成一块、加粗，`<h1>` 加下划线，同 Markdown 的标题。
    Heading(bool),
    /// 列表的一项（`<li>`）：自成一行，前面写记号。
    Item,
    /// 引用（`<blockquote>`）：同 Markdown 的引用，行首一条竖线。
    Quote,
    /// 表格的一行（`<tr>`）：自成一行。
    Row,
    /// 表格的一格（`<td>`、`<th>`）：格子之间空两格。
    Cell,
}

/// 认得的标签；别的交回 `None`，原样写出来。
fn kind(name: &str, attrs: &[(String, String)]) -> Option<Kind> {
    let style = |m: Modifier| Kind::Style(Style::new().add_modifier(m));
    Some(match name {
        "b" | "strong" => Kind::Style(theme::md_bold()),
        "i" | "em" => Kind::Style(theme::md_italic()),
        "u" | "ins" => style(Modifier::UNDERLINED),
        "s" | "del" | "strike" => style(Modifier::CROSSED_OUT),
        "code" | "kbd" | "samp" | "tt" => Kind::Style(theme::md_code()),
        "mark" => Kind::Style(theme::md_mark()),
        "sub" => Kind::Script(false),
        "sup" => Kind::Script(true),
        "a" => match attrs.iter().find(|(k, _)| k == "href") {
            Some((_, href)) => Kind::Link(href.clone()),
            None => Kind::Strip,
        },
        "hr" => Kind::Rule,
        // 2026-10-23 review：模型也写这几种块，原来照原样露出标签。
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Kind::Heading(name == "h1"),
        "li" => Kind::Item,
        "blockquote" => Kind::Quote,
        "tr" => Kind::Row,
        "td" | "th" => Kind::Cell,
        "p" | "div" | "center" | "section" | "article" | "header" | "footer" | "main" | "aside"
        | "nav" | "figure" | "figcaption" | "ul" | "ol" | "dl" | "dt" | "dd" | "pre" | "table"
        | "thead" | "tbody" | "tfoot" | "caption" => Kind::Block,
        "span" | "font" | "small" | "big" => Kind::Strip,
        _ => return None,
    })
}

impl Renderer<'_> {
    /// 认得的标签照 Markdown 的样子画；交回假表示不认得，由外面原样写出来。
    pub(in crate::markdown) fn common_tag(
        &mut self,
        name: &str,
        attrs: &[(String, String)],
        closing: bool,
    ) -> bool {
        let Some(kind) = kind(name, attrs) else {
            return false;
        };
        match (kind, closing) {
            (Kind::Rule, false) => {
                self.ensure_title();
                self.rule();
            }
            (Kind::Block, _) => {
                self.ensure_title();
                self.flush();
                // 居不居中照这一层记：`<center>`，或者写了 `align="center"` 的（2026-09-30 项目主人要做）。
                if closing {
                    self.centered.pop();
                } else {
                    let align = attrs
                        .iter()
                        .any(|(k, v)| k == "align" && v.eq_ignore_ascii_case("center"));
                    self.centered.push(name == "center" || align);
                    self.gap();
                }
            }
            (Kind::Heading(top), false) => {
                self.ensure_title();
                self.flush();
                self.gap();
                let style = if top {
                    theme::md_title().add_modifier(Modifier::UNDERLINED)
                } else {
                    theme::md_title()
                };
                let id = self.push_style(style);
                self.html_open.push((name.to_string(), Opened::Style(id)));
            }
            (Kind::Heading(_), true) => {
                self.close_tag(name);
                self.flush();
            }
            (Kind::Item, false) => {
                self.ensure_title();
                self.flush();
                let bullet = format!("{} ", self.labels.bullets[0]);
                self.push(&bullet, theme::md_list());
            }
            (Kind::Item | Kind::Row, true) => self.flush(),
            (Kind::Row, false) => {
                self.ensure_title();
                self.flush();
            }
            (Kind::Cell, false) => {}
            (Kind::Cell, true) => self.push("  ", Style::new()),
            (Kind::Quote, false) => {
                self.ensure_title();
                self.flush();
                self.gap();
                self.containers.push(Container::Quote);
            }
            (Kind::Quote, true) => {
                self.flush();
                // 只收它自己开的那一层：最里面不是引用的（没开过就收尾）不管。
                if matches!(self.containers.last(), Some(Container::Quote)) {
                    self.containers.pop();
                }
            }
            (Kind::Rule | Kind::Strip, _) => {}
            (_, true) => self.close_tag(name),
            (Kind::Style(style), false) => {
                self.ensure_title();
                let id = self.push_style(style);
                self.html_open.push((name.to_string(), Opened::Style(id)));
            }
            (Kind::Link(href), false) => {
                self.ensure_title();
                let id = self.push_style(theme::md_link());
                self.link_starts.push((href, self.pieces.len()));
                self.html_open.push((name.to_string(), Opened::Link(id)));
            }
            (Kind::Script(up), false) => {
                self.html_open.push((name.to_string(), Opened::Script(up)));
            }
        }
        true
    }

    /// 一块结束：没收尾的行内标签到这里为止。
    pub(in crate::markdown) fn close_open_tags(&mut self) {
        while let Some((_, opened)) = self.html_open.pop() {
            self.undo(opened);
        }
    }

    /// 在 `<sub>`、`<sup>` 里的字转成下标、上标（和公式同一张表）；转不动的字照原样。
    pub(in crate::markdown) fn scripted<'t>(&self, text: &'t str) -> Cow<'t, str> {
        let up = self.html_open.iter().rev().find_map(|(_, o)| match o {
            Opened::Script(up) => Some(*up),
            _ => None,
        });
        let Some(up) = up else {
            return Cow::Borrowed(text);
        };
        let table = if up {
            &self.math.superscripts
        } else {
            &self.math.subscripts
        };
        Cow::Owned(text.chars().map(|c| *table.get(&c).unwrap_or(&c)).collect())
    }

    /// 收尾标签：收掉最近一个开着的同名标签；没开过的不管。
    fn close_tag(&mut self, name: &str) {
        if let Some(at) = self.html_open.iter().rposition(|(n, _)| n == name) {
            let (_, opened) = self.html_open.remove(at);
            self.undo(opened);
        }
    }

    fn undo(&mut self, opened: Opened) {
        match opened {
            Opened::Style(id) => self.drop_style(id),
            Opened::Link(id) => {
                self.drop_style(id);
                self.end_link(false);
            }
            Opened::Script(_) => {}
        }
    }

    /// 叠一层样子，交回它的编号。
    pub(in crate::markdown) fn push_style(&mut self, style: Style) -> u32 {
        self.style_seq += 1;
        self.styles.push((self.style_seq, style));
        self.style_seq
    }

    /// 照编号拿掉那一层；已经没了的不管。
    pub(in crate::markdown) fn drop_style(&mut self, id: u32) {
        if let Some(at) = self.styles.iter().rposition(|(k, _)| *k == id) {
            self.styles.remove(at);
        }
    }

    /// Markdown 自己叠一层（强调、加粗、标题、链接……），收尾时 [`Self::pop_md_style`] 拿掉的就是这一层。
    pub(in crate::markdown) fn push_md_style(&mut self, style: Style) {
        let id = self.push_style(style);
        self.md_styles.push(id);
    }

    /// Markdown 的一层收尾：拿掉它自己叠的最后一层，不管中间夹着没收的 HTML 标签。
    pub(in crate::markdown) fn pop_md_style(&mut self) {
        if let Some(id) = self.md_styles.pop() {
            self.drop_style(id);
        }
    }
}
