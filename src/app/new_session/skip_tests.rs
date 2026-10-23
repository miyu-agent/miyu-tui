//! 新会话不问人格、预设（蓝图「新会话：人格、工作区」第 1a 条，2026-10-22 项目主人）：开着时照默认的选、不开框；设置
//! 晚到时开着的框收掉；默认的预设用不了照旧开框。

use super::super::Panel;
use super::super::test_support::app;
use super::Kind;
use crate::core::Persona;
use crate::transcript::Link;

fn row(id: &str, problem: Option<&str>) -> Persona {
    Persona {
        id: id.into(),
        name: Some(id.into()),
        summary: None,
        problem: problem.map(Into::into),
        avatar: None,
        seed: None,
        seeds: None,
    }
}

#[test]
fn a_new_session_takes_the_defaults_without_asking_when_told_to() {
    let (mut app, _sent) = app();
    app.talk.transcript.link = Link::Ready;
    app.config.layout.skip_pick = true;
    app.listed(
        Kind::Persona,
        vec![row("pirate", None), row("captain", None)],
    );
    assert!(
        app.boxes.panel.is_none(),
        "默认的还没读到：不开框，也不乱选"
    );
    assert!(app.talk.persona.chosen.is_none());
    app.picks_configured(Some("captain".into()), Some("work".into()));
    app.listed(Kind::Preset, vec![row("chat", None), row("work", None)]);
    assert!(app.boxes.panel.is_none(), "{:?}", app.boxes.panel);
    assert_eq!(app.talk.persona.chosen.as_deref(), Some("captain"));
    assert_eq!(app.talk.preset.chosen.as_deref(), Some("work"));
    // `/new`：照样不问。
    app.forget_picks();
    app.listed(
        Kind::Persona,
        vec![row("pirate", None), row("captain", None)],
    );
    app.listed(Kind::Preset, vec![row("chat", None), row("work", None)]);
    assert!(app.boxes.panel.is_none());
    assert_eq!(app.talk.preset.chosen.as_deref(), Some("work"));
}

#[test]
fn no_default_persona_means_none_and_an_unusable_default_preset_still_asks() {
    let (mut app, _sent) = app();
    app.talk.transcript.link = Link::Ready;
    app.config.layout.skip_pick = true;
    app.picks_configured(None, Some("work".into()));
    app.listed(Kind::Persona, vec![row("pirate", None)]);
    app.listed(
        Kind::Preset,
        vec![row("chat", None), row("work", Some("坏了"))],
    );
    assert_eq!(
        app.talk.persona.chosen.as_deref(),
        Some(""),
        "没设默认人格的是无人格"
    );
    assert!(
        matches!(
            app.boxes.panel,
            Some(Panel::Pick {
                kind: Kind::Preset,
                ..
            })
        ),
        "默认的预设用不了：照旧开框选"
    );
}

#[test]
fn the_setting_arriving_late_closes_an_untouched_box() {
    let (mut app, _sent) = app();
    app.talk.transcript.link = Link::Ready;
    app.picks_configured(Some("captain".into()), Some("work".into()));
    app.listed(
        Kind::Persona,
        vec![row("pirate", None), row("captain", None)],
    );
    assert!(
        matches!(app.boxes.panel, Some(Panel::Pick { .. })),
        "设置还没读到：照常开框"
    );
    app.config.layout.skip_pick = true;
    app.skip_pick_read();
    assert!(app.boxes.panel.is_none());
    assert_eq!(app.talk.persona.chosen.as_deref(), Some("captain"));
}
