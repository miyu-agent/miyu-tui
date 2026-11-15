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
    let first = lines
        .iter()
        .position(|l| l.contains("后天") && l.contains("开始喝无糖的"));
    let first = first.unwrap_or_else(|| panic!("「后天」打头、第一条接在后面：\n{text}"));
    assert!(
        lines[first + 1].contains("这阵子常看书到十二点"),
        "旧的在前：\n{text}"
    );
    assert!(
        lines[first + 2].contains("迷上了推理小说"),
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
