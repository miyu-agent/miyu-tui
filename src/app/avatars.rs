//! 人格头像替换吉祥物（蓝图 `tui.md`「空会话的首页」第 10 条，2026-10-11 项目主人）：首页画这个新会话要用的人格的头像
//! （人格框开着的跟着光标），侧边栏画开着的会话的人格的头像；没有头像、「无人格」、终端显示不了图的照旧画吉祥物。
//! 头像照 `persona.list` 给的版本去要、存成文件（`avatars.rs`），每一帧以后挑一张还没存的去要。

use std::path::PathBuf;

use serde_json::json;

use super::App;
use super::new_session::Pick;
use crate::core::{Command, Update};
use crate::oobe::asker::TAG_BASE;

/// 要头像的请求编号。
const AVATAR_TAG: u64 = TAG_BASE - 5;

/// 画在哪。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// 空会话的首页。
    Home,
    /// 宽屏的侧边栏。
    Sidebar,
}

/// 这一处画什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Avatar {
    /// 没有头像：照开关画吉祥物。
    None,
    /// 有头像，还没存下来：先空着（不先画吉祥物再换）。
    Waiting,
    /// 有头像，存在这份文件里。
    File(PathBuf),
}

/// 首页画谁的头像：人格框开着的照光标停的那一行（`under`），选过的照选的，测具设的照设的，开了会话的照会话的，都没有的
/// 照默认的。「无人格」（编号空着）没有头像。
pub(super) fn home_persona<'a>(
    under: Option<&'a str>,
    pick: &'a Pick,
    session: Option<&'a str>,
) -> Option<&'a str> {
    // 最近那个会话没带人格的（`Some(None)`）当选了「无人格」：编号空着，下面滤掉。
    let last = pick.last.as_ref().map(|l| l.as_deref().unwrap_or(""));
    under
        .or(pick.chosen.as_deref())
        .or(pick.env.as_deref())
        .or(session)
        .or(last)
        .or(pick.default.as_deref())
        .filter(|id| !id.is_empty())
}

/// 侧边栏画谁的头像：开着的会话的人格（侧边栏只在开了会话以后有）。
pub(super) fn sidebar_persona(session: Option<&str>) -> Option<&str> {
    session.filter(|id| !id.is_empty())
}

impl App {
    /// 配置页、引导的一条请求交给核心。传头像（`blob.put`，只有它带本机路径）先在后台缩到核心收的大小，缩好了由后台线程
    /// 发（review D2，2026-10-11：原来在界面线程里解码、重编码，大照片卡界面）；别的当场发。
    /// `system`：不管开没开管理员模式都带 `scope`（引导装语义模型，「管理员模式」第 5 条）。
    pub(super) fn send_ask(
        &self,
        tag: u64,
        method: &'static str,
        params: serde_json::Value,
        system: bool,
    ) {
        if method == "blob.put"
            && let Some(path) = params["path"].as_str()
        {
            let core = self.link.core.clone();
            crate::avatars::shrink::in_background(PathBuf::from(path), move |ready| {
                let params = json!({"path": ready.display().to_string()});
                core.send(Command::Ask {
                    system,
                    tag,
                    method,
                    params,
                });
            });
            return;
        }
        self.link.core.send(Command::Ask {
            system,
            tag,
            method,
            params,
        });
    }

    /// 挑一张还没存的头像去要（`tick`）：画得了图的要首页、侧边栏的（跟随头像的取色不用要：候选由核心给，「主题」第 5 条）。
    pub(super) fn ask_avatars(&mut self) {
        let shows = self.media.figures.borrow().shows();
        let seen = self.seen();
        let wanted: Vec<(String, String)> = [Place::Home, Place::Sidebar]
            .into_iter()
            .filter(|_| shows)
            .filter_map(|place| seen.wanted(place))
            .map(|(p, v)| (p.to_string(), v.to_string()))
            .collect();
        let refs: Vec<(&str, &str)> = wanted
            .iter()
            .map(|(p, v)| (p.as_str(), v.as_str()))
            .collect();
        if let Some(persona) = self.media.avatars.next(&refs) {
            self.link.core.send(Command::Ask {
                system: false,
                tag: AVATAR_TAG,
                method: "persona.avatar",
                params: json!({ "persona": persona }),
            });
        }
    }

    /// 头像回来了（交回 `true`）；断开了的，在要的那张不等了，连上以后再要。
    pub(super) fn avatar_update(&mut self, update: &Update) -> bool {
        match update {
            Update::Answer { tag, result } if *tag == AVATAR_TAG => {
                self.media.avatars.answered(result.as_ref().ok());
                true
            }
            Update::Disconnected => {
                self.media.avatars.dropped();
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Pick, home_persona, sidebar_persona};

    fn pick(chosen: Option<&str>, default: Option<&str>) -> Pick {
        Pick {
            chosen: chosen.map(str::to_string),
            default: default.map(str::to_string),
            ..Pick::default()
        }
    }

    #[test]
    fn the_home_follows_the_persona_box_then_the_choice_then_the_default() {
        let none = pick(None, Some("miyu"));
        assert_eq!(
            home_persona(Some("cat"), &none, None),
            Some("cat"),
            "框开着跟着光标"
        );
        assert_eq!(
            home_persona(Some(""), &none, None),
            None,
            "光标在「无人格」上"
        );
        assert_eq!(
            home_persona(None, &none, None),
            Some("miyu"),
            "没选过的照默认的"
        );
        let chosen = pick(Some("cat"), Some("miyu"));
        assert_eq!(
            home_persona(None, &chosen, None),
            Some("cat"),
            "选过的照选的"
        );
        let nobody = pick(Some(""), Some("miyu"));
        assert_eq!(home_persona(None, &nobody, None), None, "选了「无人格」");
        assert_eq!(
            home_persona(None, &pick(None, None), Some("dog")),
            Some("dog")
        );
        // 起来时会话列表还没到：照记下的最近那个会话用的，不先照默认的（2026-10-19：无人格也闪一帧头像）。
        let after_none = Pick {
            last: Some(None),
            ..pick(None, Some("miyu"))
        };
        assert_eq!(home_persona(None, &after_none, None), None, "上次无人格");
        let after_cat = Pick {
            last: Some(Some("cat".into())),
            ..pick(None, Some("miyu"))
        };
        assert_eq!(home_persona(None, &after_cat, None), Some("cat"));
    }

    #[test]
    fn the_sidebar_follows_the_open_session() {
        assert_eq!(sidebar_persona(Some("dog")), Some("dog"));
        assert_eq!(sidebar_persona(None), None, "没带人格的会话不照新会话选的");
    }
}
