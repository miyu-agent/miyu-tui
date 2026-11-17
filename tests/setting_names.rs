//! 终端自己的设置项（`tui/package.toml` 的 `[settings]`）的名字照核心 UX-1 定的命名规矩（核心蓝图 `config.md`「给人看的字」的
//! 「命名的规矩」，项目主人 2026-11-17）：软件术语、名词短语，不写成一句话、不带「AI」「你」；开关一律正着说。机器只拦一眼看得出的
//! 几样（同核心 `miyu_config::naming`），名词短语靠人审。

/// 一种语言里名字不许带的：句子里才有的字。英文照整词比（不分大小写），别的照字比。
fn sentence_words(language: &str) -> &'static [&'static str] {
    match language {
        "zh" => &[
            "你", "您", "我", "吗", "把", "让", "怎么", "什么", "多久", "几", "谁", "AI", "？",
            "。", "！", "，",
        ],
        "ja" => &["あなた", "自分", "AI", "？", "。", "！"],
        _ => &["you", "your", "me", "my", "ai", "?", "!"],
    }
}

/// 一种语言里开关的名字不许带的：反着说的字。
fn negative_words(language: &str) -> &'static [&'static str] {
    match language {
        "zh" => &["不", "禁用", "禁止", "关掉", "关闭", "隐藏", "停用"],
        "ja" => &["しない", "無効", "オフ", "隠す", "尋ねない"],
        _ => &["disable", "don't", "hide", "off", "no", "not", "skip"],
    }
}

/// `name` 里带了 `words` 里的哪一个：英文照整词，别的照字。
fn found<'a>(name: &str, words: &[&'a str], language: &str) -> Option<&'a str> {
    if language == "en" {
        let lower = name.to_lowercase();
        let tokens: Vec<&str> = lower
            .split(|c: char| !(c.is_alphanumeric() || c == '\''))
            .filter(|t| !t.is_empty())
            .collect();
        return words.iter().copied().find(|w| {
            if w.chars().all(|c| c.is_alphanumeric() || c == '\'') {
                tokens.contains(w)
            } else {
                lower.contains(w)
            }
        });
    }
    words.iter().copied().find(|w| name.contains(w))
}

/// `name = { en = "…", zh = "…", ja = "…" }` 里一种语言的字。
fn said<'a>(line: &'a str, language: &str) -> Option<&'a str> {
    let start = line.find(&format!("{language} = \""))? + language.len() + 4;
    let rest = &line[start..];
    Some(&rest[..rest.find('"')?])
}

/// 清单里的每一项：节名、类型、名字那一行。
fn items(manifest: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut kind = String::new();
    for line in manifest.lines() {
        if let Some(head) = line.strip_prefix("[settings.") {
            section = head.trim_end_matches(']').to_string();
            kind.clear();
        } else if line.starts_with('[') {
            section.clear();
        } else if let Some(t) = line.strip_prefix("type = \"") {
            kind = t.trim_end_matches('"').to_string();
        } else if line.starts_with("name = {") && !section.is_empty() {
            out.push((section.clone(), kind.clone(), line.to_string()));
        }
    }
    out
}

#[test]
fn terminal_setting_names_are_noun_phrases_and_switches_say_it_positively() {
    let manifest = include_str!("../tui/package.toml");
    let listed = items(manifest);
    assert!(listed.len() > 20, "读到了清单里的项：{}", listed.len());
    let mut problems = Vec::new();
    for (section, kind, line) in &listed {
        for language in ["zh", "en", "ja"] {
            let Some(name) = said(line, language) else {
                problems.push(format!("{section} 没有 {language} 的名字"));
                continue;
            };
            if let Some(word) = found(name, sentence_words(language), language) {
                problems.push(format!(
                    "{section} 的名字「{name}」像一句话：带了「{word}」"
                ));
            }
            // 部件列表的「藏不藏」（`tui.side` 这类）只在配置文件里写、配置页不画；改成正着说要连配置的写法一起改，另议。
            let parts = section.ends_with(".fields.hidden");
            if kind == "bool"
                && !parts
                && let Some(word) = found(name, negative_words(language), language)
            {
                problems.push(format!("{section} 的名字「{name}」反着说：带了「{word}」"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
