//! Markdown 排版的测试：每种写法排出来的字（蓝图 `tui.md`「她的回答：Markdown」）。

use super::{Kit, MdLine, render};
use crate::config::Config;
use crate::theme;

fn draw(text: &str, width: u16) -> Vec<MdLine> {
    let config = Config::builtin().unwrap();
    let kit = Kit {
        languages: &config.languages,
        math: &config.math,
        labels: &config.text.markdown,
        tab: config.layout.output.tab,
    };
    render(text, width, &kit, &[])
}

/// 每行：引子加内容。
fn lines(text: &str, width: u16) -> Vec<String> {
    draw(text, width)
        .iter()
        .map(|l| {
            let lead: String = l.lead.iter().map(|s| s.content.as_ref()).collect();
            format!("{lead}{}", super::inline::plain(&l.folded))
        })
        .collect()
}

#[test]
fn headings_drop_their_hashes_and_blocks_are_spaced() {
    // 不写 `#`；一级标题加下划线，别的级别一个样（`tui.md`「她的回答：Markdown」第 3 条）。只加粗、不上颜色，和正文同色
    // （2026-10-12 项目主人：原来是标题色）。
    use ratatui::style::{Modifier, Style};
    assert_eq!(lines("# 标题\n正文", 30), vec!["标题", "", "正文"]);
    assert_eq!(lines("### 小小节", 30), vec!["小小节"]);
    let top = &draw("# 标题", 30)[0].folded.spans[0];
    assert_eq!(
        top.style,
        Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    );
    let second = &draw("## 小节", 30)[0].folded.spans[0];
    let third = &draw("#### 小小节", 30)[0].folded.spans[0];
    assert_eq!(second.style, Style::new().add_modifier(Modifier::BOLD));
    assert_eq!(third.style, second.style, "二级以下一个样");
}

#[test]
fn a_single_newline_stays_a_newline() {
    assert_eq!(lines("第一行\n第二行", 30), vec!["第一行", "第二行"]);
}

#[test]
fn lists_nest_and_align_their_continuations() {
    assert_eq!(
        lines("- 一\n  - 二\n- 三四五六七八", 10),
        vec!["▪ 一", "  • 二", "▪ 三四五六", "  七八"]
    );
    // 第三层起一律 `◦`（2026-09-29 项目主人：原来依次是 • ◦ ▪，改成 ▪ • ◦）。
    assert_eq!(
        lines("- a\n  - b\n    - c\n      - d", 40),
        vec!["▪ a", "  • b", "    ◦ c", "      ◦ d"]
    );
    assert_eq!(lines("3. a\n4. b", 20), vec!["3. a", "4. b"]);
    assert_eq!(
        lines("- [x] 好了\n- [ ] 没好", 20),
        vec!["☑ 好了", "☐ 没好"]
    );
}

#[test]
fn list_markers_are_accented_and_quotes_are_dim_italic() {
    use ratatui::style::{Color, Modifier};
    let _theme = theme::hold();
    // 2026-09-29 项目主人：列表记号暗色和字分不开层次，换主题色；引用暗一些、斜体（先改过正体，又改回来）。
    let list = draw("- 苹果", 30);
    let marker = &list[0].lead[0];
    assert_eq!(marker.content, "▪ ");
    assert_ne!(marker.style, theme::dim(), "记号不再是暗色");
    assert_eq!(marker.style, theme::md_list());
    let quoted = draw("> 引用的话", 30);
    let style = quoted[0].folded.spans[0].style;
    assert!(style.add_modifier.contains(Modifier::ITALIC), "引用斜体");
    assert_eq!(style.fg, Some(Color::Rgb(0x82, 0x8b, 0xb8)), "暗灰蓝");
}

#[test]
fn links_are_dark_blue_and_bold_is_dark_orange() {
    use ratatui::style::{Color, Modifier};
    // 2026-09-29 项目主人：链接原来是高亮的蓝，标题、地址、裸地址一律换成暗蓝。
    let _theme = theme::hold();
    let dark_blue = Some(Color::Rgb(0x6a, 0x8f, 0xd8));
    assert_eq!(theme::md_link().fg, dark_blue);
    assert_eq!(theme::md_url().fg, dark_blue);
    assert!(theme::md_link().add_modifier.contains(Modifier::BOLD));
    // 2026-09-30 项目主人：粗体换成原来行内代码的暗橙。
    assert_eq!(theme::md_bold().fg, Some(Color::Rgb(0xe5, 0xa0, 0x7a)));
    assert!(theme::md_bold().add_modifier.contains(Modifier::BOLD));
    // 2026-10-18 项目主人：行内代码原来是公式色、和正文一个颜色，分不开；换成 tokyonight 的 cyan。
    assert_eq!(theme::md_code().fg, Some(Color::Rgb(0x7d, 0xcf, 0xff)));
    assert_ne!(
        theme::md_code().fg,
        theme::md_math().fg,
        "行内代码和行内公式分得开"
    );
}

#[test]
fn list_markers_and_quote_bars_are_not_copied() {
    let quoted = draw("> 引用的话", 30);
    assert_eq!(super::inline::plain(&quoted[0].folded), "引用的话");
    assert_eq!(quoted[0].lead.len(), 1);
    assert_eq!(quoted[0].folded.spans[0].style, theme::md_quote());
}

#[test]
fn code_blocks_get_frames_that_are_not_copied() {
    let drawn = draw("```python\nx = 1  # c\n```", 20);
    let text: Vec<String> = drawn
        .iter()
        .map(|l| super::inline::plain(&l.folded))
        .collect();
    assert_eq!(
        text,
        vec!["╭─ code python ─────", "x = 1  # c", "────────────────────"]
    );
    assert_eq!(
        drawn.iter().map(|l| l.copy).collect::<Vec<_>>(),
        vec![false, true, false]
    );
}

#[test]
fn an_unfinished_code_block_runs_to_the_end() {
    let text = lines("看：\n```rust\nfn main() {", 20);
    assert_eq!(
        text.last().map(String::as_str),
        Some("────────────────────")
    );
    assert!(text.contains(&"fn main() {".to_string()));
}

#[test]
fn links_show_their_address_and_know_where_they_are() {
    let drawn = draw("见 [Miyu 仓库](https://example.com)。", 60);
    assert_eq!(
        super::inline::plain(&drawn[0].folded),
        "见 Miyu 仓库 <https://example.com>。"
    );
    let urls: Vec<&str> = drawn[0]
        .folded
        .links
        .iter()
        .map(|(_, _, u)| u.as_str())
        .collect();
    assert!(urls.iter().all(|u| *u == "https://example.com") && !urls.is_empty());
    // 标题、` <`、地址、`>` 整个是一个链接，悬停时下划线不断（「见 」占 3 列，句号不算）。
    assert_eq!(
        drawn[0].folded.links,
        vec![(3, 34, "https://example.com".to_string())]
    );
    // 标题和地址一样的只写一次，用地址色、不加粗，和 `<地址>` 里那一截一个样。
    assert_eq!(lines("<https://a.com>", 60), vec!["https://a.com"]);
    assert_eq!(
        draw("<https://a.com>", 60)[0].folded.spans[0].style,
        theme::md_url()
    );
}

#[test]
fn bare_urls_and_title_url_lines_are_links() {
    let drawn = draw("官网 https://a.com。", 60);
    assert_eq!(drawn[0].folded.links[0].2, "https://a.com");
    let whole = draw("Miyu 仓库 (https://example.com)", 60);
    let (from, _, url) = &whole[0].folded.links[0];
    assert_eq!(
        (*from, url.as_str()),
        (0, "https://example.com"),
        "整行从头就是链接"
    );
    assert_eq!(
        super::inline::plain(&whole[0].folded),
        "Miyu 仓库 (https://example.com)",
        "字原样，括号不换"
    );
}

#[test]
fn title_url_items_in_a_list_are_links_too() {
    let text = "- Rust 官网 （ https://www.rust-lang.org/ ）\n- 裸的 https://a.com";
    assert_eq!(
        lines(text, 60),
        vec![
            "▪ Rust 官网 （ https://www.rust-lang.org/ ）",
            "▪ 裸的 https://a.com"
        ]
    );
    let item = &draw(text, 60)[0].folded;
    // 整行一个链接：标题链接色，括号暗，地址地址色。
    assert_eq!(
        item.links,
        vec![(0, 42, "https://www.rust-lang.org/".to_string())]
    );
    let style_of = |text: &str| {
        item.spans
            .iter()
            .find(|s| s.content.contains(text))
            .map(|s| s.style)
    };
    assert_eq!(style_of("Rust"), Some(theme::md_link()));
    assert_eq!(style_of("（"), Some(theme::dim()));
    assert_eq!(style_of("https"), Some(theme::md_url()));
}

#[test]
fn tables_are_drawn_with_rules() {
    let text = lines("| a | b |\n|---|--:|\n| 1 | 2 |", 30);
    assert_eq!(
        text,
        vec![
            "┌───┬───┐",
            "│ a │ b │",
            "├───┼───┤",
            "│ 1 │ 2 │",
            "└───┴───┘"
        ]
    );
}

#[test]
fn rules_images_and_breaks() {
    assert_eq!(lines("a\n\n---\n\nb", 60)[2], "─".repeat(20));
    assert_eq!(
        lines("![猫](https://a.com/c.png)", 60),
        vec!["[图片: 猫] <https://a.com/c.png>"]
    );
    assert_eq!(lines("上<br>下", 60), vec!["上", "下"]);
}

#[test]
fn inline_math_turns_into_unicode_in_the_math_color() {
    let drawn = draw("能量 $E=mc^2$，角 $\\alpha$。", 60);
    assert_eq!(super::inline::plain(&drawn[0].folded), "能量 E=mc²，角 α。");
    let math = drawn[0]
        .folded
        .spans
        .iter()
        .find(|s| s.content.contains("mc²"))
        .map(|s| s.style);
    assert_eq!(math, Some(theme::md_math()));
}

#[test]
fn display_math_and_mermaid_are_figures_that_carry_their_fallback() {
    use super::FigureKind;
    let drawn = draw(
        "前面\n\n$$\\frac{a+1}{b}$$\n\n```mermaid\ngraph TD\nA-->B\n```\n\n后面",
        40,
    );
    let figures: Vec<_> = drawn.iter().filter_map(|l| l.figure.as_ref()).collect();
    assert_eq!(figures.len(), 2);
    assert_eq!(
        (figures[0].kind, figures[0].source.as_str()),
        (FigureKind::Math, "\\frac{a+1}{b}")
    );
    // 画不成图时写成 Unicode，分式上下摞（2026-09-30 项目主人要做）。
    let fallback: Vec<String> = figures[0]
        .fallback
        .iter()
        .map(|l| super::inline::plain(&l.folded).trim_end().to_string())
        .collect();
    assert_eq!(fallback, [" a+1", "─────", "  b"]);
    assert_eq!(figures[1].kind, FigureKind::Mermaid);
    assert_eq!(figures[1].source, "graph TD\nA-->B\n");
    // mermaid 画不成图时照普通代码块写：框线、两行源码、框线。
    let fallback: Vec<String> = figures[1]
        .fallback
        .iter()
        .map(|l| super::inline::plain(&l.folded))
        .collect();
    assert!(fallback[0].starts_with("╭─ code mermaid "));
    assert_eq!(&fallback[1..3], ["graph TD", "A-->B"]);
    // 图的那一行不是空行：结尾的、块间的空行照旧。
    assert_eq!(lines("```mermaid\nA\n```", 40), vec![""]);
    assert!(drawn.last().is_some_and(|l| l.figure.is_none()));
}

#[test]
fn only_local_images_are_drawn_after_their_paragraph() {
    use super::FigureKind;
    let drawn = draw("看 ![图](~/a.png) 和 ![网](https://x/b.png)", 60);
    assert_eq!(drawn.len(), 2, "那一行字，加一张本机的图");
    let figure = drawn[1].figure.as_ref().unwrap();
    assert_eq!(
        (figure.kind, figure.source.as_str(), figure.fallback.len()),
        (FigureKind::Image, "~/a.png", 0)
    );
}

#[test]
fn each_image_is_drawn_under_its_own_line() {
    // 2026-09-30 项目主人：一段里一行一张的，原来地址都列完再连着画图，看不出哪张是哪个地址。
    let drawn = draw(
        "插画：\n![](/p/a.png)\n![](/p/b.png)\n- ![](/p/c.png)\n  ![](/p/d.png)",
        60,
    );
    let shape: Vec<String> = drawn
        .iter()
        .map(|l| match &l.figure {
            Some(f) => format!("图 {}", f.source),
            None => {
                let lead: String = l.lead.iter().map(|s| s.content.as_ref()).collect();
                format!("{lead}{}", super::inline::plain(&l.folded))
            }
        })
        .collect();
    assert_eq!(
        shape,
        [
            "插画：",
            "[图片: a.png] </p/a.png>",
            "图 /p/a.png",
            "[图片: b.png] </p/b.png>",
            "图 /p/b.png",
            "",
            "▪ [图片: c.png] </p/c.png>",
            "图 /p/c.png",
            "  [图片: d.png] </p/d.png>",
            "图 /p/d.png",
        ],
        "每张图紧接着画在自己那一行下面；列表的记号只写一次"
    );
}

#[test]
fn text_after_display_math_is_kept() {
    let text = lines("$$\\text{速度}=\\frac{d}{t}$$\n\n完。", 40);
    assert_eq!(text.last().map(String::as_str), Some("完。"), "{text:?}");
    let inline = lines("前 $$x^2$$ 后", 40);
    assert!(inline.iter().any(|l| l.contains('后')), "{inline:?}");
}

#[test]
fn an_image_line_is_one_link_from_its_first_character() {
    let drawn = draw("![图](/a/b.png)", 60);
    let text = super::inline::plain(&drawn[0].folded);
    assert_eq!(text, "[图片: 图] </a/b.png>");
    let width = u16::try_from(unicode_width::UnicodeWidthStr::width(text.as_str())).unwrap();
    assert_eq!(
        drawn[0].folded.links,
        vec![(0, width, "/a/b.png".to_string())],
        "整个「[图片: 说明] <地址>」一条链接，悬停时下划线不断"
    );
}

#[test]
fn an_unfinished_mermaid_block_stays_source_until_it_closes() {
    // 回答还在流：代码块没收完，照代码写，不去画一张半截的图（不然每长一截画一遍、闪一下）。
    let open = draw("```mermaid\ngraph TD\nA-->B", 40);
    assert!(open.iter().all(|l| l.figure.is_none()));
    assert!(lines("```mermaid\ngraph TD\nA-->B", 40)[0].starts_with("╭─ code mermaid"));
    let closed = draw("```mermaid\ngraph TD\nA-->B\n```", 40);
    assert!(closed.iter().any(|l| l.figure.is_some()));
    let tilde = draw("~~~mermaid\nA\n~~~\n", 40);
    assert!(tilde.iter().any(|l| l.figure.is_some()));
}

#[test]
fn an_image_without_words_shows_its_file_name() {
    // 2026-09-30 项目主人：原来方括号里空着。
    let text = |md: &str| lines(md, 100).join("\n");
    assert!(
        text("![](/tmp/imgs/wall_01.png)").contains("[图片: wall_01.png] </tmp/imgs/wall_01.png>")
    );
    assert!(text("![](https://example.invalid/a/cat.jpg?w=200#top)").contains("[图片: cat.jpg]"));
    assert!(
        text("![猫](/tmp/cat.png)").contains("[图片: 猫]"),
        "写了说明的照写"
    );
    assert!(text("<img src=\"/tmp/dog.webp\">").contains("[图片: dog.webp]"));
}

#[test]
fn a_link_across_a_line_break_after_a_local_picture_does_not_crash() {
    // review A1（2026-10-11）：一段话里先有本机的图，后面的链接文字跨了换行。换行时先把收了的字排掉、接着画图，
    // 链接记着的起点不能还指着排掉以前的位置（原来越界崩溃，重开会话照样崩）。
    let got = lines("看图 ![](a.png) 详见 [说明\n文档](https://x.y)", 60);
    assert!(got.iter().any(|l| l.contains("文档")), "{got:?}");
}

#[test]
fn a_tab_indented_code_block_keeps_its_indent() {
    // review C1（2026-10-11）：画面库把制表符丢掉，Go、Makefile 的缩进全没了。照制表位展开成空格。
    let shown = lines("```go\nfunc f() {\n\tg()\n}\n```", 40);
    assert!(shown.iter().all(|l| !l.contains('\t')), "{shown:?}");
    assert!(shown.iter().any(|l| l.contains("    g()")), "{shown:?}");
}

#[test]
fn other_ways_of_writing_display_math_are_figures_too() {
    // 2026-10-19 项目主人：她写 ```math、\[…\] 的原来照字写；```latex 照旧是代码。
    use super::FigureKind;
    let math = |text: &str| -> Vec<String> {
        draw(text, 40)
            .into_iter()
            .filter_map(|l| l.figure)
            .filter(|f| f.kind == FigureKind::Math)
            .map(|f| f.source)
            .collect()
    };
    assert_eq!(math("```math\nx^2\n```"), ["x^2"]);
    assert_eq!(math("看：\n\\[\n\\frac{a}{b}\n\\]\n完"), ["\\frac{a}{b}"]);
    assert!(math("```latex\nx^2\n```").is_empty(), "latex 代码块照代码");
    assert!(math("```math\nx^2").is_empty(), "没收完的照代码");
    let inline: String = draw("面积 \\(\\pi r^2\\)", 40)
        .iter()
        .flat_map(|l| l.folded.spans.iter().map(|s| s.content.to_string()))
        .collect();
    assert!(
        inline.contains("πr²") || inline.contains("π r²"),
        "{inline}"
    );
}

#[test]
fn local_paths_in_text_and_inline_code_open_but_keep_their_look() {
    // 第 10a 条（2026-10-22 项目主人）：文件真在的路径点了能开；样子不变，悬停才加下划线。
    let dir = std::env::temp_dir().join(format!("miyu-tui-md-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("b.rs");
    std::fs::write(&file, "x").unwrap();
    // Windows 的 canonicalize 交回 `\\?\C:\…`：正文里 Markdown 会把 `\\`、`\?` 当转义吃掉，回答里也不这样写，去掉这截。
    let file = file.canonicalize().unwrap().display().to_string();
    let file = file.trim_start_matches(r"\\?\").to_string();
    let drawn = draw(&format!("改了 {file}，见 `{file}:3`。"), 200);
    let links: Vec<_> = drawn[0].folded.links.iter().map(|l| l.2.clone()).collect();
    assert_eq!(links, [file.clone(), file.clone()]);
    let plain = draw("改了 那个文件，见 `代码`。", 200);
    let styles = |d: &[MdLine]| -> Vec<_> { d[0].folded.spans.iter().map(|s| s.style).collect() };
    assert_eq!(
        styles(&drawn)
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        styles(&plain)
            .into_iter()
            .collect::<std::collections::HashSet<_>>(),
        "不另上色"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_code_block_that_opens_a_list_item_carries_the_marker_on_its_top_rule() {
    // 2026-10-23 review：原来记号单独占一行，代码框在下一行才开始。
    let drawn = lines("- ```rust\n  let x = 1;\n  ```", 40);
    assert!(drawn[0].starts_with("▪ ╭─ code rust"), "{drawn:?}");
    assert_eq!(drawn[1], "  let x = 1;");
}

#[test]
fn footnotes_show_their_mark_and_their_note() {
    // 2026-10-23 review：原来没开脚注，`[^1]` 成了「a^1」、定义成了一条链接。
    assert_eq!(lines("a[^1]\n\n[^1]: note", 40), ["a[1]", "", "[1] note"]);
}

#[test]
fn a_dollar_in_a_link_address_does_not_make_a_formula() {
    // 2026-10-26 项目主人报：文件名带价格 `$0.11` 的链接，链接文字和地址里的 `$` 凑成一对行内公式，照原样露了出来。
    let text = "交付：**[z-$0.11.html](/home/s/z-$0.11.html)**（30.6 KB）";
    let drawn = draw(text, 120);
    let links: Vec<_> = drawn[0].folded.links.iter().map(|l| l.2.clone()).collect();
    assert_eq!(links, ["/home/s/z-$0.11.html"], "点开的还是原路径");
    assert_eq!(
        lines(text, 120),
        ["交付：z-$0.11.html </home/s/z-$0.11.html>（30.6 KB）"]
    );
}
