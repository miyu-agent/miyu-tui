//! 要画的东西的账（蓝图 `app.md` 的 `media` 块）：回答排好的行、正文按条排好的行、做好的图、链接卡片、mermaid 图、人格头像、
//! 换吉祥物要的。排正文时记下要的，主循环每一帧以后发给核心；回来了记进账。

use std::cell::RefCell;

use crate::figures::Figures;
use crate::ui::row_cache::RowCache;
use crate::ui::rows::MdCache;

/// 要画的东西的账。
pub struct Media {
    /// 回答排好的行的缓存（`ui/rows.rs`）。
    pub md_cache: RefCell<MdCache>,
    /// 正文按条缓存排好的行（`ui/row_cache`）。
    pub row_cache: RefCell<RowCache>,
    /// 正文里做好的图（蓝图「图片、公式和 mermaid 图」）。
    pub figures: RefCell<Figures>,
    /// 链接卡片的账：排正文时记下要的卡片、图，主循环每一帧以后发（`cards.rs`）。
    pub cards: RefCell<crate::link_cards::LinkCards>,
    /// mermaid 图的账：排正文时记下要画的，主循环每一帧以后交给核心（`diagrams.rs`）。
    pub diagrams: RefCell<crate::diagrams::Diagrams>,
    /// 人格头像的账：首页、侧边栏要画的，主循环每一帧以后挑一张去要（`avatars.rs`）。
    pub avatars: crate::avatars::Avatars,
    /// 换吉祥物：配置选的吉祥物包、本机试画的（`mascot_pick.rs`）。
    pub mascot_pick: super::mascot_pick::MascotPick,
    /// 要开、还在等 `blob.get` 回来的展示：blob 到条目编号（蓝图「展示」第 6 条，`shown.rs`）。
    pub opening: std::collections::HashMap<String, Vec<u64>>,
}

impl Media {
    /// 起来时：回答排好的行记 `md_keep` 条，图交给 `figures` 做；链接卡片、头像照机器的缓存读回来。
    pub fn new(md_keep: usize, figures: Figures) -> Self {
        Self {
            md_cache: RefCell::new(MdCache::new(md_keep)),
            row_cache: RefCell::new(RowCache::default()),
            figures: RefCell::new(figures),
            cards: RefCell::new(crate::link_cards::LinkCards::cached()),
            diagrams: RefCell::default(),
            avatars: crate::avatars::Avatars::cached(),
            mascot_pick: Default::default(),
            opening: Default::default(),
        }
    }
}
