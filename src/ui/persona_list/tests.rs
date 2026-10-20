//! 选人格、预设的框：中途换（「新会话：人格、工作区」第 2a 条，2026-10-19）最上面一行暗色写切换前先压缩，现在用的写「当前」。

use super::lines;
use crate::app::new_session::Kind;
use crate::config::Config;
use crate::core::Persona;

fn persona(id: &str, name: &str) -> Persona {
    Persona {
        id: id.into(),
        name: Some(name.into()),
        summary: None,
        problem: None,
        avatar: None,
        seed: None,
        seeds: None,
    }
}

#[test]
fn switching_midway_shows_the_note_first_and_dots_the_current_one() {
    let config = Config::builtin().unwrap();
    let texts = &config.text.persona_box;
    let list = [
        persona("", "无人格"),
        persona("miyu", "miyu"),
        persona("xue", "小雪"),
    ];
    let text = |current: Option<&str>, switching: bool| {
        let (_, rows, map) = lines(
            &list,
            Some("miyu"),
            (current, switching),
            "●",
            Kind::Persona,
            2,
            texts,
            60,
            10,
        );
        let rows: Vec<String> = rows
            .iter()
            .map(|l| l.to_string().trim_end().to_string())
            .collect();
        (rows, map)
    };
    let (rows, map) = text(Some("xue"), true);
    assert_eq!(
        rows[0].trim(),
        "切换前会先压缩会话避免缓存断裂导致的全量重放",
        "{rows:#?}"
    );
    assert_eq!(map[0], None, "选不中");
    // 2026-10-19 项目主人：现在用的行首写 `●`，光标停在它上面也不换成 `❯`，右边不写「当前」。
    assert!(
        rows[3].starts_with("● 小雪"),
        "现在用的、选着的写 ●：{rows:#?}"
    );
    assert!(!rows.iter().any(|r| r.contains("当前")), "{rows:#?}");
    assert!(
        !rows.iter().any(|r| r.ends_with("默认")),
        "不写默认：{rows:#?}"
    );
    // 还没开会话、还没选：照旧写默认，没有那一行。
    let (rows, map) = text(None, false);
    assert!(rows[1].ends_with("默认"), "{rows:#?}");
    assert!(map.iter().all(Option::is_some));
    // 还没开会话、已经选过（2026-10-20 项目主人报：空会话选了再打开没有圆点）：选的那个写 ●，没有切换那一行。
    let (rows, map) = text(Some("xue"), false);
    assert!(rows[2].starts_with("● 小雪"), "{rows:#?}");
    assert!(map.iter().all(Option::is_some), "{rows:#?}");
}
