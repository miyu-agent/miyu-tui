use serde_json::json;

use super::{Action, Block, PageRef, Place, Segment, State, Tone, View, Widgets, block, blocks};

fn weather() -> serde_json::Value {
    json!({
        "package": "wx", "id": "today", "place": "sidebar", "scope": "session", "title": "天气",
        "source": "Weather", "icon": "cloud-sun", "running": true,
        "content": [
            {"type": "heading", "text": "本周"},
            {"type": "text", "text": "晴", "tone": "accent"},
            {"type": "text", "text": "怪", "tone": "sparkly"},
            {"type": "pairs", "items": [{"label": "湿度", "value": "40%"}]},
            {"type": "progress", "value": 1.5, "text": "", "mark": 0.8},
            {"type": "list", "items": [
                {"title": "#128", "state": "doing", "action": {"call": "build.open", "params": {"id": 128}}},
                {"title": "#127", "state": "weird", "action": {"teleport": true}}
            ]},
            {"type": "buttons", "items": [
                {"label": "详情", "action": {"open": "week"}},
                {"label": "复制", "action": {"copy": "abc"}},
                {"label": "坏的", "action": {}}
            ]},
            {"type": "chart", "points": [1, 2]}
        ]
    })
}

#[test]
fn a_block_reads_every_kind_and_skips_what_it_does_not_know() {
    let got = block(&weather()).expect("读得出");
    assert_eq!(got.place, Place::Sidebar);
    assert_eq!(got.icon.as_deref(), Some("cloud-sun"));
    let content = got.content.expect("有内容");
    assert_eq!(content.len(), 7, "认不得的段不要：{content:?}");
    assert_eq!(content[0], Segment::Heading("本周".into()));
    assert_eq!(
        content[2],
        Segment::Text {
            text: "怪".into(),
            tone: Tone::Normal
        },
        "认不得的语气当普通"
    );
    assert_eq!(
        content[4],
        Segment::Progress {
            value: 1.0,
            text: None,
            mark: Some(0.8)
        },
        "超出的夹回来、空字当没写"
    );
    let Segment::List(items) = &content[5] else {
        panic!("{content:?}")
    };
    assert_eq!(items[0].state, Some(State::Doing));
    assert_eq!(
        items[0].action,
        Some(Action::Call {
            method: "build.open".into(),
            params: json!({"id": 128})
        })
    );
    assert_eq!(items[1].state, None, "认不得的状态不写记号");
    assert_eq!(items[1].action, None, "认不得的动作不能点");
    let Segment::Buttons(buttons) = &content[6] else {
        panic!("{content:?}")
    };
    assert_eq!(
        buttons.iter().map(|b| b.action.clone()).collect::<Vec<_>>(),
        [Action::Open("week".into()), Action::Copy("abc".into())],
        "没有动作的按钮不要"
    );
}

#[test]
fn a_block_without_content_or_a_known_place_reads_as_such() {
    let empty = json!({"package": "wx", "id": "temp", "place": "status", "source": "Weather", "running": false, "content": null});
    assert_eq!(block(&empty).expect("读得出").content, None);
    let unknown = json!({"package": "wx", "id": "x", "place": "toolbar", "source": "W"});
    assert_eq!(block(&unknown), None);
    let listed = blocks(&json!({"widgets": [empty, unknown]}));
    assert_eq!(listed.len(), 1);
}

#[test]
fn the_heading_leaves_out_a_title_that_repeats_the_source() {
    let mut b = block(&weather()).expect("读得出");
    assert_eq!(b.heading(), (Some("天气"), "Weather"));
    b.title = Some("Weather".into());
    assert_eq!(b.heading(), (None, "Weather"));
    b.title = None;
    assert_eq!(b.heading(), (None, "Weather"));
}

fn named(package: &str, id: &str) -> Block {
    let mut b = block(&weather()).expect("读得出");
    b.package = package.into();
    b.id = id.into();
    b
}

#[test]
fn a_change_replaces_removes_or_joins_its_package() {
    let mut widgets = Widgets::default();
    widgets.listed(vec![named("a", "1"), named("b", "1")]);
    widgets.changed("a", "2", Some(named("a", "2")));
    let order = |w: &Widgets| {
        w.blocks
            .iter()
            .map(|b| format!("{}/{}", b.package, b.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&widgets), ["a/1", "a/2", "b/1"], "接在同一个包后面");
    widgets.changed("c", "1", Some(named("c", "1")));
    assert_eq!(order(&widgets), ["a/1", "a/2", "b/1", "c/1"]);
    widgets.changed("a", "1", None);
    assert_eq!(order(&widgets), ["a/2", "b/1", "c/1"]);
    let mut gone = named("b", "1");
    gone.content = None;
    widgets.changed("b", "1", Some(gone));
    assert_eq!(widgets.shown(Place::Sidebar).count(), 2, "没内容的不画");
}

#[test]
fn the_view_says_where_it_looks() {
    let home = View {
        session: None,
        showing: vec![Place::Home, Place::Status],
        page: None,
    };
    assert_eq!(
        home.params(),
        json!({"stream": "widgets", "showing": ["home", "status"]})
    );
    let page = View {
        session: Some("s1".into()),
        showing: vec![],
        page: Some(PageRef {
            package: "wx".into(),
            widget: "week".into(),
        }),
    };
    assert_eq!(
        page.params(),
        json!({"stream": "widgets", "showing": [], "session": "s1", "page": {"package": "wx", "widget": "week"}})
    );
}

#[test]
fn the_right_side_of_a_list_item_is_one_text_or_up_to_four_toned_runs() {
    use super::Run;
    let b = block(&json!({
        "package": "git", "id": "changes", "place": "sidebar", "source": "Git", "running": true,
        "content": [{"type": "list", "items": [
            {"title": "a.rs", "right": "3m"},
            {"title": "b.rs", "right": [{"text": "+12", "tone": "good"}, {"text": "−3", "tone": "bad"}]},
            {"title": "c.rs", "right": [{"text": "1"}, {"text": "2"}, {"text": "3"}, {"text": "4"}, {"text": "5"}]},
            {"title": "d.rs", "right": ""}
        ]}]
    }))
    .expect("读得出");
    let Some(Segment::List(items)) = b.content.as_ref().and_then(|c| c.first()) else {
        panic!("{b:?}")
    };
    assert_eq!(
        items[0].right,
        [Run {
            text: "3m".into(),
            tone: None
        }],
        "一句字不带语气"
    );
    assert_eq!(
        items[1].right,
        [
            Run {
                text: "+12".into(),
                tone: Some(Tone::Good)
            },
            Run {
                text: "−3".into(),
                tone: Some(Tone::Bad)
            }
        ]
    );
    assert_eq!(items[2].right.len(), 4, "最多 4 段");
    assert!(items[3].right.is_empty());
}
