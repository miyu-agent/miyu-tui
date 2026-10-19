//! 多行的一段是命令还是一句话（照旧版 `miyu --shell-classify`）：单行由脚本自己看首词，多行才问这里。

use std::path::{Path, PathBuf};

use super::shells::Shell;

/// 是不是一条 shell 命令：首词是关键字、内建命令、真能执行的路径，或者 PATH 上找得到。
pub fn is_shell_command(input: &str, shell: Shell) -> bool {
    let Some((command, rest)) = first_command(input) else {
        return false;
    };
    if ambiguous_tail_reads_like_a_sentence(&command, rest) {
        return false;
    }
    is_keyword_or_builtin(&command, shell)
        || is_executable_path(&command)
        || in_path(&command, std::env::var_os("PATH").as_deref())
}

/// 第一个不是 `名字=值` 的词，和它后面剩下的。
fn first_command(input: &str) -> Option<(String, &str)> {
    let mut offset = 0;
    while let Some(token) = next_token(input, &mut offset) {
        if is_assignment(&token) {
            continue;
        }
        return Some((token, input.get(offset..).unwrap_or("")));
    }
    None
}

/// `time`、`test` 这几个既是命令、也常出现在话的开头：后面带问号、中文的当成话（「time 是什么命令？」）。
fn ambiguous_tail_reads_like_a_sentence(command: &str, rest: &str) -> bool {
    if !matches!(
        command,
        "time" | "test" | "date" | "which" | "type" | "command" | "history"
    ) {
        return false;
    }
    let rest = rest.trim();
    !rest.is_empty() && rest.chars().any(|c| c == '?' || c == '？' || is_cjk(c))
}

fn is_cjk(c: char) -> bool {
    matches!(
        c,
        '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2CEAF}'
    )
}

/// 照 fish 的样子切下一个词：跳过空白和注释，认单双引号、反斜杠；`; | & < >` 断词。
fn next_token(input: &str, offset: &mut usize) -> Option<String> {
    let mut index = *offset;
    loop {
        let c = input.get(index..)?.chars().next()?;
        if c.is_whitespace() {
            index += c.len_utf8();
        } else if c == '#' {
            index += input[index..]
                .find(['\n', '\r'])
                .map_or(input.len() - index, |n| n + 1);
        } else {
            break;
        }
    }
    let mut token = String::new();
    let (mut single, mut double, mut escaped) = (false, false, false);
    let mut consumed = input.len();
    for (relative, c) in input[index..].char_indices() {
        if escaped {
            token.push(c);
            escaped = false;
        } else if c == '\\' && !single {
            escaped = true;
        } else if c == '\'' && !double {
            single = !single;
        } else if c == '"' && !single {
            double = !double;
        } else if !single
            && !double
            && (c.is_whitespace() || matches!(c, ';' | '|' | '&' | '<' | '>'))
        {
            consumed = index + relative + c.len_utf8();
            if token.is_empty() {
                token.push(c);
            }
            break;
        } else {
            token.push(c);
        }
    }
    *offset = consumed;
    (!token.is_empty()).then_some(token)
}

fn is_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn is_keyword_or_builtin(command: &str, shell: Shell) -> bool {
    const COMMON: &[&str] = &[
        "alias",
        "bg",
        "break",
        "builtin",
        "case",
        "cd",
        "command",
        "continue",
        "else",
        "end",
        "exec",
        "exit",
        "false",
        "fg",
        "for",
        "function",
        "functions",
        "history",
        "if",
        "jobs",
        "not",
        "or",
        "and",
        "read",
        "return",
        "set",
        "source",
        "status",
        "switch",
        "test",
        "time",
        "true",
        "while",
    ];
    const FISH: &[&str] = &[
        "abbr", "argparse", "begin", "bind", "block", "contains", "count", "disown", "emit",
        "eval", "math", "random", "string", "type", "ulimit",
    ];
    COMMON.contains(&command) || (shell == Shell::Fish && FISH.contains(&command))
}

/// 写出来的路径要真能执行才算命令：粘进来的图片路径打头的多行一段是话（旧版 08-26 实测：整段被 fish 逐行执行，图全丢了）。
fn is_executable_path(command: &str) -> bool {
    let shaped = ["/", "./", "../", "~/"]
        .iter()
        .any(|p| command.starts_with(p));
    if !shaped {
        return false;
    }
    let path = match command.strip_prefix("~/") {
        Some(rest) => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(rest),
            None => return false,
        },
        None => PathBuf::from(command),
    };
    is_executable(&path)
}

/// PATH 上找不找得到这个名字。
fn in_path(command: &str, path: Option<&std::ffi::OsStr>) -> bool {
    if command.is_empty() || command.contains('/') {
        return false;
    }
    path.is_some_and(|path| {
        std::env::split_paths(path).any(|dir| is_executable(&dir.join(command)))
    })
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

#[cfg(test)]
mod tests;
