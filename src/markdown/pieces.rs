//! 收着的行内片段（蓝图 `tui.md`「她的回答：Markdown」）：链接收尾时把收了的字标成链接，换行时先把这一行排掉、接着画
//! 这一行的图，整个拿走片段时把没收完的链接一起挪好起点（review A1）。从 `mod.rs` 挪出来（2026-10-11：到了 500 行）。

use ratatui::style::Style;

use super::Renderer;
use super::inline::Piece;
use super::links;
use crate::theme;

impl Renderer<'_> {
    /// 链接收完：标题成链；标题和地址不一样的，后面跟 ` <地址>`。图片写成 `[图片: 说明] <地址>`。
    pub(super) fn end_link(&mut self, image: bool) {
        let Some((url, start)) = self.link_starts.pop() else {
            return;
        };
        let start = start.min(self.pieces.len());
        let label: String = self.pieces[start..]
            .iter()
            .map(|p| p.text.as_str())
            .collect();
        for piece in &mut self.pieces[start..] {
            piece.link = Some(url.clone());
        }
        if image {
            // 没写说明的写文件名（2026-09-30 项目主人：原来方括号里空着）。
            if label.trim_end() == self.labels.image.trim_end() {
                let name = links::file_name(&url);
                self.pieces
                    .push(Piece::linked(name, theme::md_image(), &url));
            }
            self.pieces
                .push(Piece::linked("]", theme::md_image(), &url));
            self.image(&url);
        }
        if image || label.trim() != url {
            self.pieces.extend(links::address(&url));
        } else {
            // 只有地址的（`<地址>`）用地址色，和 `<地址>` 里那一截一个样（蓝图第 9 条）。
            let style = self.style().patch(theme::md_url());
            for piece in &mut self.pieces[start..] {
                piece.style = style;
            }
        }
    }

    /// 一段话里换行。这一行里有本机的图的，先把这一行排出来、紧接着画它的图，再接着排这一段下面的行
    /// （蓝图「图片、公式和 mermaid 图」第 3 条：每张图画在自己那一行下面）。
    pub(super) fn line_break(&mut self) {
        if self.images.is_empty() {
            self.push("\n", Style::new());
            return;
        }
        let pieces = self.take_pieces();
        if !pieces.is_empty() {
            self.emit(links::relink_title(pieces), true);
        }
        self.flush_images();
    }

    /// 把收着的片段整个拿走（换行时先排、段落收尾、表格的格子）：没收完的链接，已经收了的那一截先标上地址，起点挪到
    /// 新的开头，链接收尾时不再指着拿走以前的位置（review A1，2026-10-11：原来越界崩溃）。
    pub(super) fn take_pieces(&mut self) -> Vec<Piece> {
        let len = self.pieces.len();
        for (url, start) in &mut self.link_starts {
            for piece in &mut self.pieces[(*start).min(len)..] {
                piece.link = Some(url.clone());
            }
            *start = 0;
        }
        std::mem::take(&mut self.pieces)
    }
}
