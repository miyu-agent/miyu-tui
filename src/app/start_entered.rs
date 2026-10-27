//! 起来进已有的会话时不问人格、预设（蓝图 `tui.md`「新会话：人格、工作区」第 1d 条，2026-10-26 项目主人报：启动时打开的是
//! 最近那个会话，还弹预设框，那个会话已经选过了）。
//!
//! 连上以前照记下的列表开框（`look.rs`）时还不知道这一回进不进已有的会话（`ui.startup` 是 `recent` 的由核心线程读、
//! 进），框先开出来，会话接上了也没收。改成「在等进已有的会话」（`Boxes::entering`）时当没有开头的框，照常画输入框：
//! - 起来时：`--resume` 的、上一回起来进了已有的会话的，先当在等；
//! - 头一次连上进了已有的会话（`Update::Ready`）：接着等到会话接上，开着的框收掉，打了的字留在输入框；
//! - 没进的（`Reconnected`）、要进的会话没了（`SessionGone`）：不等了，照常问。进没进记下来，下一回照它。

use super::seen::Seen;
use super::{App, Panel};
use crate::core::Update;

impl Seen<'_> {
    /// 在等进已有的会话：还没接上会话，开头不问人格、预设。
    pub fn entering(&self) -> bool {
        self.boxes.entering && self.talk.transcript.session.is_none()
    }
}

impl App {
    /// 头一次连上的结果：记下进没进已有的会话；进了的收掉开头的框，没进的照常问。要进的会话没了也照常问。
    pub(super) fn entered_update(&mut self, update: &Update) {
        let entered = match update {
            Update::Ready(_) if !self.boxes.entered_known => true,
            Update::Reconnected if !self.boxes.entered_known => false,
            Update::SessionGone(_) if self.boxes.entering => {
                self.boxes.entering = false;
                self.ask_next();
                return;
            }
            _ => return,
        };
        self.boxes.entered_known = true;
        self.boxes.entering = entered;
        self.link.look.entered = entered;
        self.save_look();
        if entered {
            if matches!(self.boxes.panel, Some(Panel::Pick { .. })) {
                self.boxes.panel = None;
            }
        } else {
            self.ask_next();
        }
    }
}
