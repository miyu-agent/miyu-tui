//! 人格的窗里列她在人生里记下的变化（核心 LF-11「后天」）：只读，旧的在前，为什么变的暗色写在下面。

use serde_json::json;

use super::{page, screen};
use crate::config::Config;
use crate::settings::pages::changes::Change;
use crate::settings::pages::choose::PersonaView;
use crate::settings::pages::personas;
use crate::settings::popup::Popup;

#[test]
fn the_persona_window_lists_what_changed_oldest_first_with_why_under_it() {
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let made = json!({"persona": "rain", "name": "小雨"});
    let open = |p: &mut crate::settings::Settings| {
        p.popup = Some(Popup::Persona(PersonaView {
            id: "rain".into(),
            detail: Some(personas::detail(&made)),
            error: None,
            cursor: 0,
        }));
    };
    open(&mut p);
    let (text, _) = screen(&mut p, &config, 100, 40);
    assert!(!text.contains("后天"), "没有记下的变化不写这几行：\n{text}");
    p.more.changes = Some((
        "rain".into(),
        vec![
            Change {
                text: "开始喝无糖的".into(),
                why: None,
            },
            Change {
                text: "这阵子常看书到十二点".into(),
                why: Some("迷上了推理小说".into()),
            },
        ],
    ));
    open(&mut p);
    let (text, _) = screen(&mut p, &config, 100, 40);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    // 2026-11-17 项目主人本机验收第 30 条：「后天」改叫「后天提示词」，说明「AI 自己通过推演人生产生的提示词」。
    let head = lines.iter().position(|l| {
        l.starts_with("后天提示词") && l.ends_with("AI 自己通过推演人生产生的提示词")
    });
    let head = head.unwrap_or_else(|| panic!("「后天提示词」打头、后面写说明：\n{text}"));
    assert_eq!(
        lines[head + 1],
        "开始喝无糖的",
        "一条一行、旧的在前：\n{text}"
    );
    assert_eq!(lines[head + 2], "这阵子常看书到十二点", "{text}");
    assert!(
        lines[head + 3].contains("迷上了推理小说"),
        "为什么变的在下面一行：\n{text}"
    );
    // 别的人格的不写。
    p.more.changes = Some((
        "other".into(),
        vec![Change {
            text: "别人的".into(),
            why: None,
        }],
    ));
    open(&mut p);
    let (text, _) = screen(&mut p, &config, 100, 40);
    assert!(!text.contains("别人的"), "{text}");
}

#[test]
fn the_life_context_shows_the_plain_default_and_says_where_else_to_change_it() {
    // 2026-11-17 项目主人本机验收第 30 条：「空着用出厂的 100,000」改「100000」，说明「也可以在人生会话中用 /context 命令单独修改」。
    use crate::settings::pages::life::LifeState;
    let config = Config::builtin().unwrap();
    let mut p = page(&config);
    let made = json!({"persona": "rain", "name": "小雨", "life_context_default": 100000});
    p.more.life = Some((
        "rain".into(),
        LifeState {
            state: "running".into(),
            ..LifeState::default()
        },
    ));
    let rows = crate::settings::pages::persona_edit::fields(true).len();
    for cursor in [0, rows - 1] {
        p.popup = Some(Popup::Persona(PersonaView {
            id: "rain".into(),
            detail: Some(personas::detail(&made)),
            error: None,
            cursor,
        }));
        let (text, _) = screen(&mut p, &config, 100, 40);
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("人生会话上下文窗口"))
            .unwrap_or_else(|| panic!("{text}"));
        assert!(lines[at].ends_with("  100000"), "只写数：\n{text}");
        let said = "也可以在人生会话中用 /context 命令单独修改";
        assert_eq!(
            lines[at + 1] == said,
            cursor == rows - 1,
            "光标停在这一行才写说明：\n{text}"
        );
    }
}
