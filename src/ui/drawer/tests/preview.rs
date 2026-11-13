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

/// 2026-11-02 项目主人报：带预览的提问，中线错位、选项被截成 `…`。用的是那天真问的一道（中英混排的说明在空格处折行，
/// 原来那一行多带一格空白，中线往右错一列）。
#[test]
fn the_divider_stays_in_one_column_and_long_labels_fold() {
    let grub = "GRUB_CMDLINE_LINUX=\"zswap.enabled=0 rootfstype=btrfs clearcpuid=514,avx512f,avx512dq,avx512cd,avx512bw,avx512vl,pku\"";
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "你想怎么走？", "options": [
            {"label": "改 GRUB 屏蔽 AVX-512 后重启（推荐）", "description": "在 clearcpuid= 里加上 avx512*、pku 等，重启后 XCR0 自然变成 7，壳的检查原样通过；副作用是全系统没有 AVX-512（可随时改回）", "preview": grub},
            {"label": "先不重启，继续在用户态想办法", "description": "保留我对 artifact.dll 的补丁绕开检查，我再深挖它为什么卡在保护壳的 VM 里空转（现在正在跑一个 15 分钟的长测试）"},
            {"label": "我能拿到 brcly 的 LD_PRELOAD 助手", "description": "RIN 帖里现在推荐用 brcly 的 LD_PRELOAD + 原版 Proton-GE，比这个 7 月的老改版兼容性更好"}
        ]}]
    }))
    .unwrap();
    let d = Drawer::question(None, asked);
    // 窄于 `side_min`（86）的挪到下面、没有竖线，只看标题不截。
    let narrow = lines(&d, 60);
    assert!(!narrow.iter().any(|l| l.contains('…')), "{narrow:#?}");
    for width in [86, 90, 110, 140] {
        let got = lines(&d, width);
        let columns: Vec<usize> = got
            .iter()
            .filter_map(|row| row.find('│').map(|bar| row[..bar].width()))
            .collect();
        assert!(columns.len() > 4, "{width} 列：{got:#?}");
        assert!(
            columns.iter().all(|c| *c == columns[0]),
            "{width} 列：竖线都在一列 {columns:?}\n{got:#?}"
        );
        assert!(
            !got.iter().any(|l| l.contains('…')),
            "{width} 列：不截：{got:#?}"
        );
        let left: String = got
            .iter()
            .map(|l| l.split('│').next().unwrap_or_default().replace(' ', ""))
            .collect();
        assert!(
            left.contains("我能拿到brcly的LD_PRELOAD助手"),
            "{width} 列：标题折下去写全：{got:#?}"
        );
    }
}

/// 2026-11-11 项目主人：「文字会导致中间的竖线错位」——预览、选项里的制表符、控制字符原样画进格子，终端照制表位多走几列，
/// 这一行顶到下一行去，竖线就歪了。预览里的制表符照制表位展开，别的控制字符去掉；选项、说明、问题里的都去掉。
#[test]
fn tabs_and_control_characters_never_reach_the_screen() {
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪\u{1b}[31m种？", "options": [
            {"label": "缩\t进", "description": "带\u{7}铃", "preview": "fn main() {\n\tprintln!(\"hi\");\r\n}"},
            {"label": "别的", "preview": "x"}
        ]}]
    }))
    .unwrap();
    let d = Drawer::question(None, asked);
    for width in [100, 50] {
        let got = lines(&d, width);
        let bad = |c: char| c.is_control();
        assert!(!got.iter().any(|l| l.contains(bad)), "{got:#?}");
        assert!(
            got.iter().any(|l| l.contains("    println!(\"hi\");")),
            "制表符照 4 列展开：{got:#?}"
        );
    }
    let bars: Vec<usize> = lines(&d, 100)
        .iter()
        .filter_map(|l| l.find('│').map(|at| l[..at].width()))
        .collect();
    assert!(bars.windows(2).all(|p| p[0] == p[1]), "竖线一列：{bars:?}");
}

/// 同一天：「预览内容比较多的时候」抽屉比窗口高，原来照高度一裁，预览下半截没了、也不说还有（那一行只照 16 行算）。放不下
/// 时预览少露几行（最少 3 行），「下面还有 N 行」照实写，`Ctrl+J` 照旧能滚到底。
#[test]
fn a_tall_preview_shrinks_to_the_drawer_and_says_how_much_is_below() {
    let many: String = (1..=30).map(|i| format!("第 {i} 行\n")).collect();
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [{"question": "哪种？", "options": [
            {"label": "很多", "preview": many},
            {"label": "别的", "preview": "x"}
        ]}]
    }))
    .unwrap();
    let mut d = Drawer::question(None, asked);
    let shown = |d: &mut Drawer| -> Vec<String> {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 12)).unwrap();
        let mut caret = crate::caret::Caret::default();
        term.draw(|f| {
            super::super::draw(f, (f.area(), 0), d, &texts(), &look(), false, &mut caret);
        })
        .unwrap();
        let buf = term.backend().buffer();
        (0..12)
            .map(|y| {
                // 宽字后面那一格是占位的空白，跳过。
                let mut line = String::new();
                let mut skip = 0;
                for x in 0..100 {
                    if skip > 0 {
                        skip -= 1;
                        continue;
                    }
                    let symbol = buf[(x, y)].symbol();
                    skip = symbol.width().saturating_sub(1);
                    line.push_str(symbol);
                }
                line.trim_end().to_string()
            })
            .collect()
    };
    let got = shown(&mut d);
    // 12 行：问题、空行、8 行并排（「预览」加 7 行图）、空行、按键提示。
    assert!(got[2].contains("预览 · 下面还有 23 行"), "{got:#?}");
    assert!(got[9].contains("第 7 行"), "{got:#?}");
    assert!(got[11].contains("Enter"), "按键提示还在：{got:#?}");
    for _ in 0..40 {
        d.key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
        shown(&mut d);
    }
    let got = shown(&mut d);
    assert!(got[9].contains("第 30 行"), "滚得到底：{got:#?}");
}

/// 2026-11-13 主人 1113 初验第 4 条：抽屉照内容变宽，切题时宽度跟着变「太违和」，宽度定死。几道题里只要有一页带宽预览，
/// 每一页（连「确认」页）都要同一个宽。
#[test]
fn the_drawer_keeps_one_width_across_its_questions() {
    let asked: Asked = serde_json::from_value(serde_json::json!({
        "call_id": "c",
        "questions": [
            {"question": "第一道", "options": [{"label": "宽", "preview": "x".repeat(120)}, {"label": "窄"}]},
            {"question": "第二道", "options": [{"label": "一个很长很长很长的选项写在这里", "description": "说明"}, {"label": "短"}]}
        ]
    }))
    .unwrap();
    let mut d = Drawer::question(None, asked);
    let widths: Vec<Option<u16>> = (0..d.pages())
        .map(|page| {
            d.tab = page;
            super::super::preview_wanted(&d, &texts(), &look(), 200)
        })
        .collect();
    assert!(widths[0].is_some());
    assert!(
        widths.iter().all(|w| *w == widths[0]),
        "切题、进确认页都不变：{widths:?}"
    );
}
