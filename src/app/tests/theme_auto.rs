//! 出厂「自动」配色（蓝图「主题」第 2 条子规则，2026-11-02 项目主人）：看着的人格交得出颜色的跟随头像，不然往下找，都没有的
//! tokyonight；配置里写的还是 `auto`。

use crate::core::Persona;

#[test]
fn auto_follows_the_persona_of_the_open_session_when_it_has_colors() {
    let (mut app, _sent) = crate::app::test_support::app();
    assert_eq!(app.chosen().colors, "auto", "出厂自动");
    assert_eq!(
        app.choice().colors,
        "tokyonight",
        "什么都借不到：tokyonight"
    );
    app.talk.transcript.session = Some("s-1".into());
    app.talk.transcript.persona = Some("writer".into());
    app.talk.persona.list = Some(vec![Persona {
        id: "writer".into(),
        seed: Some("#2fab34".into()),
        ..Persona::default()
    }]);
    assert_eq!(
        app.choice().colors,
        "persona",
        "会话的人格有主题色：跟随头像"
    );
    assert_eq!(app.chosen().colors, "auto", "配置里写的不变");
    // 只算挑哪一套，不设上去：配色是全局的，设了会搅乱并着跑的别的测试。
    let seed = app.wanted_seed();
    let themes = &app.config.themes;
    let picked = crate::theme::choice::pick(&app.choice(), None, themes, seed).expect("挑得出");
    assert_eq!(picked.name, "persona-night", "照人格的主色生成了一套");
}
