//! 回答里写的本机路径（蓝图「她的回答：Markdown」第 10a 条）：正文、行内代码里像路径、文件又真在的几截，点了用系统的
//! 打开方式开。像不像只看写法（带 `/`、或者 `名字.后缀`），真不真在问文件系统（`local::existing`）。

/// 断开路径的字：空白、括号、引号、中文标点（中文句子里不空格，「改好了（src/a.rs），另见」照这些断）。路径里的汉字照留。
fn breaks(c: char) -> bool {
    c.is_whitespace() || "()[]<>\"'`「」『』（）《》“”‘’，。；：！？、".contains(c)
}

/// 末尾要剥掉的西文句读：`见 a.rs.`、`a.rs:` 的点、冒号不算路径（`:12` 这样带行号的留着）。
const TRAIL: &[char] = &[',', '.', ';', ':', '!', '?'];

/// 一段字里是路径的几截：字节的起、止，打开时交的绝对路径。
pub fn local_paths(text: &str) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    let mut start = 0;
    for (at, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        if !breaks(c) {
            continue;
        }
        let body = text[start..at].trim_end_matches(TRAIL);
        if looks_like_path(body)
            && let Some(file) = crate::local::existing(body)
        {
            out.push((start, start + body.len(), file));
        }
        start = at + c.len_utf8();
    }
    out
}

/// 写法像路径：带 `/` 或 `\`（不是地址；Windows 的路径是反斜杠，带行号的 `C:\a\b.rs:3` 也认），或者 `名字.后缀`
/// （`Cargo.toml`）。别的（一个词、版本号 `1.2`）不去问文件系统。
fn looks_like_path(word: &str) -> bool {
    if word.is_empty() || word.contains("://") {
        return false;
    }
    if word.contains(['/', '\\']) {
        return word.chars().any(char::is_alphanumeric);
    }
    match word.rsplit_once('.') {
        Some((name, ext)) => {
            name.chars().any(char::is_alphabetic)
                && !ext.is_empty()
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && ext.chars().any(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{local_paths, looks_like_path};

    #[test]
    fn only_words_written_like_paths_are_asked_about() {
        for yes in [
            "src/main.rs",
            "~/a.txt",
            "/etc/hosts",
            "Cargo.toml",
            "./run.sh",
            r"C:\Users\a\notes.md:3",
            r"\\?\C:\Temp\b.rs",
        ] {
            assert!(looks_like_path(yes), "{yes}");
        }
        for no in ["hello", "1.2", "https://a.invalid/x", "/", "..."] {
            assert!(!looks_like_path(no), "{no}");
        }
    }

    #[test]
    fn paths_that_exist_are_found_with_punctuation_trimmed() {
        let dir = std::env::temp_dir().join(format!("miyu-tui-md-paths-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("notes.md");
        std::fs::write(&file, "x").unwrap();
        let file = file.canonicalize().unwrap();
        let shown = file.display().to_string();
        let text = format!("改好了（{shown}），另见 {shown}:3。还有 {shown}.gone 不在");
        let got = local_paths(&text);
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(&text[got[0].0..got[0].1], shown);
        assert_eq!(&text[got[1].0..got[1].1], format!("{shown}:3"));
        assert!(got.iter().all(|(_, _, f)| *f == shown));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
