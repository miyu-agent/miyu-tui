//! 会话列表（`session.list`，蓝图 `tui.md`「会话列表 `/sessions`」第 1、3 条）：读成一行行，只留主会话。
//! C-3 合进来以前没有工作目录、忙不忙、最近动静这三格，读成没有。

use serde_json::Value;

/// 列表里的一个会话。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionInfo {
    /// 整个编号。
    pub session: String,
    /// 标题；没起名的是 `None`。
    pub title: Option<String>,
    /// 标题是人起的（核心的 `named`，最后一次写标题的是人才有）。
    pub named: bool,
    /// 置顶了。
    pub pinned: bool,
    /// 在哪个目录里干活（C-3）。
    pub cwd: Option<String>,
    /// 有一轮在跑（C-3）。
    pub busy: bool,
    /// 最近一次动静（C-3 的 `last_active`）。
    pub last_active: Option<jiff::Timestamp>,
    /// 没标题时核心给的预览：第一句话的第一行，最多 50 个字（核心 9-5）。
    pub preview: Option<String>,
    /// 用的人格的编号；没带人格（无人格）的是 `None`。新会话的人格框照最近那个会话停（「新会话」第 1 条）。
    pub persona: Option<String>,
    /// 用的预设的编号。
    pub preset: Option<String>,
    /// 连同它派的、分叉出来的子会话一共用了多少 token（核心 d98f5e03，口径同底栏的累计）；没有这一格的当 0。
    pub tokens: u64,
    /// 最近一次请求的上下文 token 数、模型窗口（核心 f3f30d85 的 `context: {used, window?}`）；没说过话的是 `None`，
    /// 窗口查不到的窗口是 `None`。
    pub context: Option<(u64, Option<u64>)>,
    /// 终端集成开的（核心 `origin: "terminal"`，2026-10-21 起开的才有）：会话列表多写一格「终端集成」。
    pub terminal: bool,
}

/// 读 `session.list` 的回应、订阅会话列表的回应（两样一个形状）：子会话、一次性会话、整理会话不要，照核心交回的先后（新的
/// 在前）。
pub fn read(result: &Value) -> Vec<SessionInfo> {
    result["sessions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(entry)
        .collect()
}

/// 读一项；子会话、一次性会话、整理会话（核心 D-1 下的 `kind: "dream"`，怎么画等 D-3）、读不成的是 `None`。
fn entry(s: &Value) -> Option<SessionInfo> {
    if !s["parent"].is_null() || s["oneshot"] == true || s["kind"] == "dream" {
        return None;
    }
    let text = |v: &Value| v.as_str().filter(|t| !t.is_empty()).map(str::to_string);
    Some(SessionInfo {
        session: s["session"].as_str()?.to_string(),
        title: text(&s["title"]),
        named: s["named"] == true,
        pinned: s["pinned"] == true,
        cwd: s["cwd"].as_str().map(str::to_string),
        busy: s["busy"] == true,
        last_active: s["last_active"].as_str().and_then(|t| t.parse().ok()),
        preview: text(&s["preview"]),
        persona: text(&s["persona"]),
        preset: super::session_preset(s),
        tokens: s["tokens"].as_u64().unwrap_or(0),
        context: s["context"]["used"]
            .as_u64()
            .map(|used| (used, s["context"]["window"].as_u64().filter(|w| *w > 0))),
        terminal: s["origin"] == "terminal",
    })
}

/// 最近的那个会话：最近一次动静最晚的，一样晚（都没有）的取排在前面的（核心交回的新的在前）。新会话的人格、预设框照
/// 它停在上一次选的（蓝图「新会话：人格、工作区」第 1、3 条，2026-10-09 项目主人定）。
pub fn latest(list: &[SessionInfo]) -> Option<&SessionInfo> {
    list.iter().reduce(|best, s| {
        if s.last_active > best.last_active {
            s
        } else {
            best
        }
    })
}

/// 订阅会话列表（核心 9-5）：回应是整张列表，之后推 `sessions.changed`。交回请求编号。
pub(super) async fn follow(rpc: &mut super::rpc::Rpc) -> std::io::Result<String> {
    rpc.send("subscribe", serde_json::json!({"stream": "sessions"}))
        .await
}

/// 推来的一条 `sessions.changed`（核心 9-5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// 照编号整项换上，没有的加到最前面。
    Entry(SessionInfo),
    /// 删掉了（不列的子会话、一次性会话也照这个：列表里本来就不该有）。
    Removed(String),
}

/// 读 `sessions.changed` 的参数；读不成的是 `None`。
pub fn change(params: &Value) -> Option<Change> {
    let session = params["session"].as_str()?.to_string();
    if params["removed"] == true {
        return Some(Change::Removed(session));
    }
    Some(entry(&params["entry"]).map_or(Change::Removed(session), Change::Entry))
}

/// 发了删除、核心还没推来「删掉了」的会话（蓝图「会话列表」第 2 条，2026-10-18 项目主人报多选删除时列表跳动）：核心一个
/// 一个推「删掉了」，每推一个界面照手上那一份整份换上列表；手上那一份没拿掉、或者删完以前推来了它们的整项，删了的又回到
/// 列表里，列表一会儿长一会儿短。
#[derive(Debug, Default)]
pub struct Deleting(std::collections::HashSet<String>);

impl Deleting {
    /// 删这几个：手上那一份当场拿掉，记着。
    pub fn start(&mut self, seen: Option<&mut Vec<SessionInfo>>, gone: &[String]) {
        if let Some(seen) = seen {
            seen.retain(|s| !gone.contains(&s.session));
        }
        self.0.extend(gone.iter().cloned());
    }

    /// 推来的变化要不要：删着的整项不要；它的「删掉了」到了就不再算删着的。
    pub fn admits(&mut self, change: &Change) -> bool {
        match change {
            Change::Entry(info) => !self.0.contains(&info.session),
            Change::Removed(session) => {
                self.0.remove(session);
                true
            }
        }
    }
}

/// 照一条变化改列表。
pub fn apply(list: &mut Vec<SessionInfo>, change: Change) {
    match change {
        Change::Entry(info) => match list.iter_mut().find(|s| s.session == info.session) {
            Some(slot) => *slot = info,
            None => list.insert(0, info),
        },
        Change::Removed(session) => list.retain(|s| s.session != session),
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, Deleting, SessionInfo, apply};

    fn info(session: &str) -> SessionInfo {
        SessionInfo {
            session: session.into(),
            ..SessionInfo::default()
        }
    }

    #[test]
    fn deleted_sessions_do_not_come_back_while_the_core_reports_them_one_by_one() {
        // 2026-10-18 项目主人报多选删除时列表跳动。
        let mut seen: Vec<SessionInfo> = ["a", "b", "c", "d"].iter().map(|s| info(s)).collect();
        let ids = |seen: &[SessionInfo]| seen.iter().map(|s| s.session.clone()).collect::<Vec<_>>();
        let mut deleting = Deleting::default();
        deleting.start(Some(&mut seen), &["b".into(), "c".into()]);
        assert_eq!(ids(&seen), ["a", "d"], "当场拿掉");
        // 核心先推 b 删掉了：c 不回来。
        let b = Change::Removed("b".into());
        assert!(deleting.admits(&b));
        apply(&mut seen, b);
        assert_eq!(ids(&seen), ["a", "d"]);
        // c 删完以前推来它的整项：不要。
        assert!(!deleting.admits(&Change::Entry(info("c"))));
        // c 删掉了以后，同一个编号再来的照常收（不会再有，防它记一辈子）。
        assert!(deleting.admits(&Change::Removed("c".into())));
        assert!(deleting.admits(&Change::Entry(info("c"))));
        assert!(deleting.admits(&Change::Entry(info("a"))), "别的照常");
    }

    use serde_json::json;

    use super::read;

    #[test]
    fn only_main_sessions_are_listed_and_missing_fields_read_as_none() {
        let result = json!({"sessions":[
            {"session":"a","title":"回文","pinned":true,"parent":null,"oneshot":false},
            {"session":"b","parent":"a","oneshot":false},
            {"session":"c","parent":null,"oneshot":true},
            {"session":"d","parent":null,"oneshot":false,"title":"","cwd":"/src","busy":true,
             "last_active":"2026-10-01T08:00:00Z","tokens":5120,"context":{"used":4400,"window":1000000}},
        ]});
        let list = read(&result);
        assert_eq!(
            list.iter().map(|s| s.session.as_str()).collect::<Vec<_>>(),
            ["a", "d"]
        );
        assert_eq!(list[0].title.as_deref(), Some("回文"));
        assert!(list[0].pinned && list[0].cwd.is_none() && list[0].last_active.is_none());
        assert_eq!(list[1].title, None, "空标题是没起名");
        assert!(list[1].busy);
        assert_eq!(list[1].cwd.as_deref(), Some("/src"));
        assert_eq!(
            (list[0].tokens, list[1].tokens),
            (0, 5120),
            "没有 tokens 的当 0（核心 d98f5e03）"
        );
        assert_eq!(
            (list[0].context, list[1].context),
            (None, Some((4400, Some(1_000_000)))),
            "上下文用量（核心 f3f30d85）"
        );
        assert!(list[1].last_active.is_some());
    }

    #[test]
    fn the_latest_session_is_the_last_active_one_and_carries_its_picks() {
        let list = read(&json!({"sessions":[
            {"session":"pinned","parent":null,"oneshot":false,"pinned":true,
             "last_active":"2026-10-01T08:00:00Z","persona":"old"},
            {"session":"new","parent":null,"oneshot":false,
             "last_active":"2026-10-09T08:00:00Z","preset":"full"},
        ]}));
        let latest = super::latest(&list).unwrap();
        assert_eq!(latest.session, "new", "置顶的排在前面，照动静挑");
        assert_eq!(latest.persona, None, "没带人格");
        assert_eq!(latest.preset.as_deref(), Some("full"));
        assert!(super::latest(&[]).is_none());
    }

    #[test]
    fn a_pushed_change_replaces_by_id_puts_new_ones_first_and_drops_removed_ones() {
        // 核心 9-5：`sessions.changed` 照编号整项换；没标题的带 `preview`。
        use super::{Change, apply, change};
        let mut list = read(&json!({"sessions":[
            {"session":"a","title":"回文","parent":null,"oneshot":false},
            {"session":"b","parent":null,"oneshot":false,"preview":"帮我看看这个"}
        ]}));
        assert_eq!(list[1].preview.as_deref(), Some("帮我看看这个"));
        let renamed = change(
            &json!({"session":"b","entry":{"session":"b","title":"看代码","parent":null,"oneshot":false}}),
        );
        apply(&mut list, renamed.unwrap());
        assert_eq!(list[1].title.as_deref(), Some("看代码"), "整项换");
        let new = change(
            &json!({"session":"c","entry":{"session":"c","parent":null,"oneshot":false,"busy":true}}),
        );
        apply(&mut list, new.unwrap());
        assert_eq!(list[0].session, "c", "新的排到前面");
        apply(
            &mut list,
            change(&json!({"session":"a","removed":true})).unwrap(),
        );
        assert_eq!(
            list.iter().map(|s| s.session.as_str()).collect::<Vec<_>>(),
            ["c", "b"]
        );
        let child =
            change(&json!({"session":"x","entry":{"session":"x","parent":"c","oneshot":false}}));
        assert_eq!(
            child,
            Some(Change::Removed("x".into())),
            "子会话不列：当没有"
        );
    }

    #[test]
    fn dream_sessions_are_not_listed() {
        // 核心 D-1 下：整理会话也列得出来，带 `kind: "dream"`；终端的会话列表不要（怎么画等 D-3）。
        use serde_json::json;
        let got = json!({"sessions": [
            {"session": "a1b2c3d4", "title": "平常的"},
            {"session": "d1d2d3d4", "kind": "dream", "dream": {"persona": "miyu", "manual": true}}
        ]});
        let ids: Vec<String> = super::read(&got).into_iter().map(|s| s.session).collect();
        assert_eq!(ids, ["a1b2c3d4"]);
        let change =
            json!({"session": "d1d2d3d4", "entry": {"session": "d1d2d3d4", "kind": "dream"}});
        assert_eq!(
            super::change(&change),
            Some(Change::Removed("d1d2d3d4".into()))
        );
    }

    #[test]
    fn sessions_opened_by_the_shell_hook_are_marked() {
        // 核心 1d08dfa32：`session.terminal` 开的会话一项带 `origin: "terminal"`；别的、旧的没有这一格。
        use serde_json::json;
        let got = json!({"sessions": [
            {"session": "a1b2c3d4", "origin": "terminal"},
            {"session": "e5f6a7b8"}
        ]});
        let marked: Vec<bool> = super::read(&got).into_iter().map(|s| s.terminal).collect();
        assert_eq!(marked, [true, false]);
    }
}
