use ratatui::text::Span;

use super::{Model, Part, fit, left_line, level_label, middle, model_spans, split};

#[test]
fn a_narrow_left_side_is_clipped_with_an_ellipsis() {
    // 2026-09-29 28 列实测：`▣ 工作区 · deepseek-v4.` 硬截，没有 `…`。
    let spans = vec![
        Span::raw("▣ 工作区"),
        Span::raw(" · "),
        Span::raw("deepseek-v4.1-flash"),
        Span::raw(" dev"),
    ];
    assert_eq!(
        left_line(spans.clone(), 40).to_string(),
        "▣ 工作区 · deepseek-v4.1-flash dev"
    );
    let clipped = left_line(spans, 22).to_string();
    assert_eq!(
        clipped, "▣ 工作区 · deepseek-v…",
        "占满 22 列，最后一格是 …"
    );
}

#[test]
fn the_button_sits_in_the_middle_of_the_gap() {
    // 左 20 列、右 30 列、宽 100：空当是第 22 列到第 68 列，按钮 10 列放正中。
    assert_eq!(middle(20, 30, 10, 100), Some(22 + (46 - 10) / 2));
    assert_eq!(middle(20, 30, 50, 100), None, "放不下");
}

fn part(keep: u8, text: &'static str) -> Part {
    Part::new(keep, vec![Span::raw(text)])
}

fn shown(spans: Vec<Span<'static>>) -> String {
    spans.iter().map(|s| s.content.as_ref()).collect()
}

#[test]
fn fit_drops_the_least_kept_first_and_keeps_the_order() {
    let parts = || {
        vec![
            part(2, "回复中 3s"),
            part(0, "200 tok/s"),
            part(3, "1.7k/1M"),
            part(1, "Σ3k"),
        ]
    };
    assert_eq!(
        shown(fit(parts(), 100)),
        "回复中 3s · 200 tok/s · 1.7k/1M · Σ3k"
    );
    // 放不下：先丢速度，再丢累计。
    assert_eq!(shown(fit(parts(), 25)), "回复中 3s · 1.7k/1M · Σ3k");
    assert_eq!(shown(fit(parts(), 20)), "回复中 3s · 1.7k/1M");
    assert_eq!(shown(fit(parts(), 8)), "1.7k/1M");
    // 上下文都放不下了：放得下的照放（2026-10-11 项目主人：明明还有空，看不到别的）。
    assert_eq!(shown(fit(parts(), 3)), "Σ3k");
    assert_eq!(shown(fit(parts(), 2)), "");
}

#[test]
fn a_long_part_that_does_not_fit_lets_a_shorter_one_in() {
    // 2026-10-11 项目主人报：明明还有空，累计却看不到。花费那一格比累计晚丢，它太长放不下时累计跟着丢了。
    let parts = vec![
        part(3, "342.6k/1M(32.7%)"),
        part(1, "Σ27.5M(C97%)"),
        part(2, "¥12.34 · $0.56 · €0.78"),
    ];
    assert_eq!(shown(fit(parts, 32)), "342.6k/1M(32.7%) · Σ27.5M(C97%)");
}

#[test]
fn a_crowded_line_shortens_the_model_before_dropping_numbers() {
    // 2026-10-11 项目主人定：放不下时模型编号先只写最后一段，空出的地方给速度、上下文、累计；宽的照旧写全。
    // 2026-10-19 项目主人：速度、花费比权限级别的字先丢；累计、上下文比它晚。格子的 `keep` 照真的：速度 1、花费 2、
    // 累计 3、上下文 4。
    let full = || {
        vec![
            Span::raw("▣ 工作区 · "),
            Span::raw("clinepass/cline-pass/deepseek-v4.1-flash dev · max"),
        ]
    };
    let short = || {
        vec![
            Span::raw("▣ 工作区 · "),
            Span::raw("deepseek-v4.1-flash dev · max"),
        ]
    };
    let cells = || {
        vec![
            part(1, "41 tok/s"),
            part(4, "342.6k/1M(32.7%)"),
            part(3, "Σ27.5M(C97.3%)"),
            part(2, "¥0.12"),
        ]
    };
    // 2026-10-23 项目主人：供应商名最后才收。
    let iconic = || vec![Span::raw("▣"), Span::raw(" deepseek-v4.1-flash dev · max")];
    let bare = || vec![Span::raw("▣"), Span::raw(" deepseek-v4.1-flash · max")];
    let lefts = || [full(), short(), iconic(), bare()];
    let (left, right, _) = split(lefts(), cells(), 0, 95);
    assert_eq!(left.to_string(), "▣ 工作区 · deepseek-v4.1-flash dev · max");
    assert_eq!(
        right.to_string(),
        "41 tok/s · 342.6k/1M(32.7%) · Σ27.5M(C97.3%) · ¥0.12"
    );
    let (left, _, _) = split(lefts(), cells(), 0, 140);
    assert!(left.to_string().contains("clinepass/"), "宽的照旧写全");
    // 再窄：速度、花费先丢，权限级别的字还在。
    let (left, right, _) = split(lefts(), cells(), 0, 82);
    assert_eq!(left.to_string(), "▣ 工作区 · deepseek-v4.1-flash dev · max");
    assert_eq!(right.to_string(), "342.6k/1M(32.7%) · Σ27.5M(C97.3%)");
    // 再窄：连累计、上下文都要挤掉时，权限级别的字才收掉、只留图标（2026-10-18 项目主人）。
    let (left, right, _) = split(lefts(), cells(), 0, 70);
    assert_eq!(left.to_string(), "▣ deepseek-v4.1-flash dev · max");
    assert_eq!(right.to_string(), "342.6k/1M(32.7%) · Σ27.5M(C97.3%)");
    // 最窄：供应商名也收掉。
    let (left, _, _) = split(lefts(), cells(), 0, 62);
    assert_eq!(left.to_string(), "▣ deepseek-v4.1-flash · max");
}

#[test]
fn cooling_says_how_long_until_it_is_back_and_goes_away_on_time() {
    // 「配置与模型」第 7 条，核心 8-9。
    let texts = crate::config::Config::builtin().unwrap().text.models;
    let now: jiff::Timestamp = "2026-10-01T08:00:00Z".parse().unwrap();
    let at = |s: &str| Some(s.parse::<jiff::Timestamp>().unwrap());
    assert_eq!(
        super::cooling_word(None, now, &texts).as_deref(),
        Some("模型均在冷却")
    );
    assert_eq!(
        super::cooling_word(at("2026-10-01T08:01:10Z"), now, &texts).as_deref(),
        Some("模型均在冷却，2 分钟后恢复"),
        "向上取整"
    );
    assert_eq!(
        super::cooling_word(at("2026-10-01T07:59:00Z"), now, &texts),
        None,
        "到点了变回原样"
    );
}

#[test]
fn an_icon_only_level_drops_the_dot_too() {
    // 2026-10-19 项目主人：窄了收掉「开放权限」这几个字以后，后面那个「·」也收掉。
    assert_eq!(
        level_label(Some("⏵⏵ "), "开放权限", true),
        ("⏵⏵ 开放权限".to_string(), " · ".to_string())
    );
    assert_eq!(
        level_label(Some("⏵⏵ "), "开放权限", false),
        ("⏵⏵".to_string(), " ".to_string())
    );
    assert_eq!(
        level_label(None, "开放权限", false),
        ("开放权限".to_string(), " · ".to_string()),
        "没有图标的照旧写字"
    );
}

#[test]
fn a_shortened_model_keeps_the_provider_name_until_last() {
    // 2026-10-19 项目主人：宽度不够时供应商信息也可以隐藏；2026-10-23 改成最后才收（「最后再缩，而不是一开始就缩」）。
    let model = || "clinepass/cline-pass/deepseek-v4.1-flash".to_string();
    assert_eq!(
        shown(model_spans(model(), "magpie", Model::Full)),
        "clinepass/cline-pass/deepseek-v4.1-flash magpie"
    );
    assert_eq!(
        shown(model_spans(model(), "magpie", Model::Short)),
        "deepseek-v4.1-flash magpie"
    );
    assert_eq!(
        shown(model_spans(model(), "magpie", Model::Bare)),
        "deepseek-v4.1-flash"
    );
}

/// 用量那一格卡住时的几截只隔一个空格（`tight`），放不下先收「用了多少 / 线」（「框下面那一行」用量那一格）。
#[test]
fn tight_parts_join_with_a_space_and_drop_by_keep() {
    let tight = |keep: u8, text: &'static str| {
        let mut p = part(keep, text);
        p.tight = true;
        p
    };
    let parts = || {
        vec![
            part(5, "上下文压缩失败"),
            tight(3, "203k/190k"),
            tight(5, "压缩"),
        ]
    };
    assert_eq!(shown(fit(parts(), 40)), "上下文压缩失败 203k/190k 压缩");
    assert_eq!(shown(fit(parts(), 20)), "上下文压缩失败 压缩");
}

/// 左段两件照配置排（「挂载位」第 4 条 SL-2）：换先后时中间是 ` · `，藏起来的不写，空的不留分隔。
#[test]
fn the_left_side_follows_the_configured_order() {
    use crate::config::slots::Slot;
    let slot = |part: &str, hidden: bool| Slot {
        part: part.into(),
        hidden,
        rows: 0,
    };
    let joined = |config: &[Slot], model: Vec<Span<'static>>| -> String {
        let level = (vec![Span::raw("L")], " ".to_string());
        super::arranged(config, level, model)
            .iter()
            .map(|s| s.content.to_string())
            .collect()
    };
    let model = || vec![Span::raw("M")];
    assert_eq!(joined(&[], model()), "L M", "出厂：权限级别接它自己的分隔");
    let sep = crate::sheet::sep();
    assert_eq!(joined(&[slot("model", false)], model()), format!("M{sep}L"));
    assert_eq!(joined(&[slot("level", true)], model()), "M");
    assert_eq!(joined(&[], Vec::new()), "L", "没有模型不留分隔");
}
