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
            ("●", "! "),
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
        "! 切换前会先压缩会话避免缓存断裂导致的全量重放",
        "{rows:#?}"
    );
    assert_eq!(map[0], None, "选不中");
    assert!(
        rows[1].trim().is_empty() && map[1].is_none(),
        "和列表之间空一行：{rows:#?}"
    );
    // 2026-10-19 项目主人：现在用的行首写 `●`，光标停在它上面也不换成 `❯`，右边不写「当前」。
    assert!(
        rows[4].starts_with("● 小雪"),
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

#[test]
fn the_preset_box_shows_each_presets_description_and_the_persona_box_does_not() {
    // 2026-10-23 项目主人定（核心转来）：预设后面暗色写描述，没有的不写。
    let config = Config::builtin().unwrap();
    let texts = &config.text.persona_box;
    let mut described = persona("dev", "极简开发");
    described.summary = Some("只开命令和读写".into());
    let list = [described, persona("all", "全部功能")];
    let rows = |kind| {
        let (_, rows, _) = lines(
            &list,
            None,
            (None, false),
            ("●", "! "),
            kind,
            0,
            texts,
            60,
            10,
        );
        rows.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    let presets = rows(Kind::Preset);
    assert!(
        presets[0].contains("极简开发  只开命令和读写"),
        "{presets:#?}"
    );
    assert_eq!(
        presets[1].trim_end().trim_start_matches(|c| c != '全'),
        "全部功能"
    );
    assert!(!rows(Kind::Persona)[0].contains("只开命令"), "人格框不写");
}

#[test]
fn no_tools_shows_its_own_description_not_the_no_persona_note() {
    // 「无工具」编号空着（核心 P-16 下）：写它自己的描述，不写「无人格」那一句（2026-10-23 真机自测查出来的）。
    let config = Config::builtin().unwrap();
    let texts = &config.text.persona_box;
    let mut none = persona("", "无工具");
    none.summary = Some("不提供任何工具。".into());
    let list = [none];
    let (_, rows, _) = lines(
        &list,
        None,
        (None, false),
        ("●", "! "),
        Kind::Preset,
        0,
        texts,
        60,
        10,
    );
    let row = rows[0].to_string();
    assert!(row.contains("无工具  不提供任何工具。"), "{row}");
    assert!(!row.contains(&texts.none_note), "{row}");
}
