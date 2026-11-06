use crate::app::prefix_ask::PrefixAsk;
use crate::config::Config;

fn text(
    ask: Option<&PrefixAsk>,
    button: usize,
    max: usize,
) -> (String, Vec<String>, Vec<Option<usize>>) {
    let config = Config::builtin().unwrap();
    let (chrome, lines, map) = super::lines(ask, button, &config.text.prefix, 60, max);
    let title = chrome.title.iter().map(|s| s.content.to_string()).collect();
    let lines = lines
        .iter()
        .map(|l| l.to_string().trim_end().to_string())
        .collect();
    (title, lines, map)
}

#[test]
fn the_box_lists_the_files_then_three_buttons() {
    let ask = PrefixAsk {
        files: vec!["root/personas/alpha/prompts/persona.md".into()],
        more: 3,
        command: None,
    };
    let (title, lines, map) = text(Some(&ask), 0, 20);
    assert_eq!(title, "下一轮的缓存会失效");
    assert_eq!(
        lines,
        [
            "  这些文件改了：",
            "    root/personas/alpha/prompts/persona.md",
            "    还有 3 个",
            "",
            "  [ 直接发送 ]  [ 压缩上下文并发送 ]  [ 取消 ]"
        ]
    );
    assert_eq!(map.last(), Some(&Some(0)), "按钮那一行点得到");
}

#[test]
fn a_short_box_keeps_the_buttons() {
    let files = (1..=20).map(|i| format!("root/prompts/{i}.md")).collect();
    let ask = PrefixAsk {
        files,
        more: 0,
        command: None,
    };
    let (_, lines, _) = text(Some(&ask), 2, 5);
    assert_eq!(
        lines,
        [
            "  这些文件改了：",
            "    root/prompts/1.md",
            "    还有 19 个",
            "",
            "  [ 直接发送 ]  [ 压缩上下文并发送 ]  [ 取消 ]"
        ]
    );
    let (_, tiny, _) = text(Some(&ask), 2, 1);
    assert_eq!(tiny, ["  [ 直接发送 ]  [ 压缩上下文并发送 ]  [ 取消 ]"]);
}
