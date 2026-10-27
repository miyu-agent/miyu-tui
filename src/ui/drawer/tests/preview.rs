//! 提问选项的预览：并排、窄了挪下面、跟着内容放宽、折行、滚（蓝图「确认和提问的抽屉」第 3 条）。

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthStr;

use super::super::view;
use super::{lines, look, press, texts};
use crate::drawer::{Asked, Drawer};

#[test]
fn a_preview_sits_on_the_right_under_a_dim_title_and_below_when_narrow() {
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪种？", "options": [
            {"label": "左右", "preview": "┌──┬──┐\n│甲│乙│\n└──┴──┘"},
            {"label": "上下", "preview": "┌──┐\n└──┘"}
        ]}]
    }))
    .unwrap();
    let d = Drawer::question(None, asked);
    let wide = lines(&d, 100);
    // 选项只占自己要的宽（这里最宽的「  3. 输入其他答案」17 列），竖线在第 18 列，预览从第 20 列起，和选项顶上对齐：第一行暗色
    // 「预览」，下面是图；
    // 不铺底色，中间一条竖线（2026-10-07 项目主人；2026-10-23 照 Claude Code 改成选项往左让、预览拿剩下的，原来竖线在中线）。
    assert!(wide[2].starts_with("› 1. 左右"), "{wide:?}");
    let title = wide[2].find("预览").expect("第一行写预览");
    assert_eq!(wide[2][..title].width(), 20);
    let at = wide[3].find('┌').expect("图在下一行");
    assert_eq!(wide[3][..at].width(), 20);
    assert!(wide[5].contains("└──┴──┘"));
    assert!(!wide.iter().any(|l| l.contains('╭')), "不描边");
    for row in &wide[2..6] {
        let bar = row.find('│').expect("每行都有竖线");
        assert_eq!(row[..bar].width(), 18, "竖线紧跟在选项那一列后面：{row}");
    }
    let v = view(&d, &texts(), &look(), 100, false);
    assert!(
        v.lines[3].spans.iter().all(|s| s.style.bg.is_none()),
        "不铺底色：{:?}",
        v.lines[3]
    );
    // 窄了：挪到选项下面，中间空一行。
    let narrow = lines(&d, 50);
    assert_eq!(narrow[2], "› 1. 左右");
    let title = narrow.iter().position(|l| l == "预览").expect("有预览");
    assert_eq!(narrow[title - 1], "", "{narrow:?}");
    assert_eq!(narrow[title + 1], "┌──┬──┐", "{narrow:?}");
    assert!(!narrow[2].contains('│'), "上下排不画竖线");
}

#[test]
fn a_wide_preview_wraps_and_a_long_one_scrolls_with_ctrl_j() {
    // 2026-10-23 项目主人：预览放不下的折行（Claude Code 也折），太长的 `Ctrl+J` `Ctrl+K` 滚。
    let long_line = "chromium --headless --no-sandbox --user-data-dir=/tmp/profile --screenshot=out.png https://example.com";
    let many: String = (1..=40).map(|i| format!("第 {i} 行\n")).collect();
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪种？", "options": [
            {"label": "一行", "preview": long_line},
            {"label": "很多", "preview": many}
        ]}]
    }))
    .unwrap();
    let mut d = Drawer::question(None, asked);
    let got = lines(&d, 80);
    assert!(
        got.iter().any(|l| l.contains("chromium --headless")),
        "{got:#?}"
    );
    assert!(
        got.iter().any(|l| l.contains("example.com")),
        "折下来了，没截掉：{got:#?}"
    );
    assert!(!got.iter().any(|l| l.contains('…')), "{got:#?}");
    // 第二项 40 行：露 16 行，「预览」后面写下面还有几行，按键提示多「Ctrl+J/K 滚动预览」。
    press(&mut d, KeyCode::Down);
    (d.command_rows, d.command_window) = super::super::preview::scroll(&d, &texts(), &look(), 80);
    assert_eq!((d.command_rows, d.command_window), (40, 16));
    let got = lines(&d, 80);
    assert!(got.iter().any(|l| l.contains("下面还有 24 行")), "{got:#?}");
    assert!(
        got.last().is_some_and(|l| l.contains("Ctrl+J/K 滚动预览")),
        "{got:#?}"
    );
    d.key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    let got = lines(&d, 80);
    assert!(
        got.iter().any(|l| l.contains("第 2 行")) && !got.iter().any(|l| l.ends_with("第 1 行")),
        "{got:#?}"
    );
    // 换回第一项：从头露起。
    press(&mut d, KeyCode::Up);
    assert_eq!(d.command_top, 0);
}

#[test]
fn a_wide_preview_asks_for_a_wider_drawer() {
    // 2026-10-23 项目主人照 Claude Code：预览比输入框宽时抽屉跟着放宽（`ui/mod.rs` 照它放宽排版）。
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪种？", "options": [
            {"label": "宽", "preview": "x".repeat(120)},
            {"label": "窄", "preview": "y"}
        ]}]
    }))
    .unwrap();
    let d = Drawer::question(None, asked);
    let want = super::super::preview_wanted(&d, &texts(), &look(), 200).expect("有预览");
    // 选项那一列照最宽的「  3. 输入其他答案」17 列（带序号，2026-10-27），空 3 列，预览 120 列。
    assert_eq!(want, 17 + 3 + 120);
    let plain: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c", "questions": [{"question": "哪种？", "options": [{"label": "甲"}]}]
    }))
    .unwrap();
    assert_eq!(
        super::super::preview_wanted(&Drawer::question(None, plain), &texts(), &look(), 200),
        None
    );
}
