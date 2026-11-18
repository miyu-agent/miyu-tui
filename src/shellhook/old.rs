//! 旧版（0.7）装的那一份（蓝图 `shellhook.md`「旧版那一份」）：bash、zsh 启动文件里旧记号之间的一段，fish 的
//! `conf.d/miyu.fish`。两份都接管回车，一句话会交两遍：`install` 先备份再删，`status`、`doctor` 只说一声。只认旧版自己
//! 写的记号和固定的位置，认不出的一律不碰；照字节处理，文件不是 UTF-8 的也一样。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use self::backup::{backup, free};
use super::install::{self, Done};
use super::shells::Shell;
use super::words::{Words, fill};

mod backup;

/// 旧版给每份脚本盖的戳的开头（第一行 `# miyu shell hook · v<版本> · <指纹>`）。
// 中点写成转义：代码里的记号字面量归主题管（`sheet/guard.rs`），这个是旧版文件里的字，不是画的记号。
const STAMP: &str = "# miyu shell hook \u{b7} v";

/// 给人看的一行：`trouble` 的是出错、提醒，命令行印到标准错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub text: String,
    pub trouble: bool,
}

impl Said {
    fn done(text: String) -> Self {
        Self {
            text,
            trouble: false,
        }
    }

    fn trouble(text: String) -> Self {
        Self {
            text,
            trouble: true,
        }
    }
}

/// 一处旧版装过的地方。
struct Spot {
    path: PathBuf,
    seen: io::Result<Seen>,
}

/// 那一处是什么样。
enum Seen {
    /// 启动文件（`text` 是读到的整份）：几段旧的，和只有开头没有结尾的那几行（原文件里的行号，从 1 起）。
    Blocks {
        text: Vec<u8>,
        blocks: Vec<Block>,
        orphans: Vec<usize>,
    },
    /// fish 的旧文件，有戳。
    File,
    /// fish 的旧文件是符号链接（原来指到哪）：指的文件有戳，或者已经不在了。
    Link(PathBuf),
    /// fish 的 `miyu.fish` 在，认不出是旧版的：不碰，只提醒。
    Foreign,
}

/// 启动文件里旧的一段：第几行到第几行（含两行记号），字节从哪到哪（含结尾记号那一行的换行）。
struct Block {
    first: usize,
    last: usize,
    start: usize,
    end: usize,
}

/// 装进 `shell`（`target` 是装在哪），先清掉这种 shell 旧版的那一份（别的 shell 的不动）。交回装成哪种、每清一处
/// 一行和出错、提醒的话。新的那一段和旧的在同一个文件里的（`.bashrc`、`.zshrc`）同一次写；备份不成的这个文件只装新的。
///
/// # Errors
///
/// 装新的那一份读写失败（这时别处的旧的没动）；备份成了、新的写不进去的，话里带着备份在哪。
pub fn install(
    shell: Shell,
    target: &Path,
    program: &Path,
    env: &impl Fn(&str) -> Option<String>,
    words: &Words,
) -> io::Result<(Done, Vec<Said>)> {
    let mut said = Vec::new();
    let done = match shell {
        Shell::Bash | Shell::Zsh => install_rc(shell, target, program, words, &mut said)?,
        Shell::Fish | Shell::Pwsh | Shell::WinPs => install::install_at(shell, target, program)?,
    };
    let mine = identity(target);
    for spot in find(shell, env) {
        if shell != Shell::Fish && identity(&spot.path) == mine {
            continue;
        }
        if done == Done::Foreign {
            said.extend(told(&spot, shell, words).into_iter().map(Said::trouble));
        } else {
            said.extend(clean(spot, shell, words));
        }
    }
    Ok((done, said))
}

/// `status`、`doctor` 说的：这种 shell 旧版那一份还在的，一处一行（只说不删）。
pub fn warnings(shell: Shell, env: &impl Fn(&str) -> Option<String>, words: &Words) -> Vec<String> {
    find(shell, env)
        .iter()
        .flat_map(|spot| told(spot, shell, words))
        .collect()
}

/// 装进 bash、zsh 的启动文件 `target`：里面有旧的先整份备份，去掉旧的、装上新的，一次写。
fn install_rc(
    shell: Shell,
    target: &Path,
    program: &Path,
    words: &Words,
    said: &mut Vec<Said>,
) -> io::Result<Done> {
    let Some(text) = install::read(target)? else {
        return install::install_at(shell, target, program);
    };
    let (blocks, orphans) = scan(&text, shell);
    let shown = target.display().to_string();
    said.extend(
        orphans
            .iter()
            .map(|&line| Said::trouble(unclosed(&shown, line, words))),
    );
    if blocks.is_empty() {
        return install::put(shell, target, program, Some(text));
    }
    let backup = match backup(target, &text, words) {
        Ok(backup) => backup,
        Err(e) => {
            said.push(Said::trouble(failed(&shown, &e, words)));
            said.extend(still(&shown, &blocks, shell, words).map(Said::trouble));
            return install::put(shell, target, program, Some(text));
        }
    };
    match install::put(shell, target, program, Some(strip(&text, &blocks))) {
        Ok(done) => {
            said.extend(cleaned(&shown, &blocks, &backup, words).map(Said::done));
            Ok(done)
        }
        Err(e) => {
            let kept = kept(&shown, &e, &backup, words);
            let lines: Vec<String> = std::iter::once(kept)
                .chain(still(&shown, &blocks, shell, words))
                .collect();
            Err(io::Error::new(e.kind(), lines.join("\n")))
        }
    }
}

/// 清一处（不是装新的那个文件的）：出了错的原样不动，说清原因、提醒旧的还在。
fn clean(spot: Spot, shell: Shell, words: &Words) -> Vec<Said> {
    let shown = spot.path.display().to_string();
    let seen = match spot.seen {
        Ok(seen) => seen,
        Err(e) => return vec![Said::trouble(unreadable(&shown, &e, words))],
    };
    match seen {
        Seen::Blocks {
            text,
            blocks,
            orphans,
        } => {
            let mut said: Vec<Said> = orphans
                .iter()
                .map(|&line| Said::trouble(unclosed(&shown, line, words)))
                .collect();
            if blocks.is_empty() {
                return said;
            }
            let trouble = match backup(&spot.path, &text, words) {
                Err(e) => failed(&shown, &e, words),
                Ok(backup) => match install::write(&spot.path, &strip(&text, &blocks)) {
                    Ok(()) => {
                        said.extend(cleaned(&shown, &blocks, &backup, words).map(Said::done));
                        return said;
                    }
                    Err(e) => kept(&shown, &e, &backup, words),
                },
            };
            said.push(Said::trouble(trouble));
            said.extend(still(&shown, &blocks, shell, words).map(Said::trouble));
            said
        }
        Seen::File => {
            let renamed = free(&spot.path).and_then(|backup| {
                fs::rename(&spot.path, &backup)?;
                Ok(backup)
            });
            match renamed {
                Ok(backup) => vec![Said::done(fill(
                    &words.old_cleaned_file,
                    &[("path", &shown), ("backup", &backup.display().to_string())],
                ))],
                Err(e) => fish_failed(&shown, &e, shell, words),
            }
        }
        Seen::Link(to) => match fs::remove_file(&spot.path) {
            Ok(()) => vec![Said::done(fill(
                &words.old_cleaned_link,
                &[("path", &shown), ("target", &to.display().to_string())],
            ))],
            Err(e) => fish_failed(&shown, &e, shell, words),
        },
        Seen::Foreign => vec![Said::trouble(fill(&words.old_hook, &[("path", &shown)]))],
    }
}

/// fish 的旧文件改名、删链接不成：说清原因，提醒旧的还在。
fn fish_failed(shown: &str, e: &io::Error, shell: Shell, words: &Words) -> Vec<Said> {
    vec![
        Said::trouble(failed(shown, e, words)),
        Said::trouble(fill(
            &words.old_file,
            &[("path", shown), ("shell", shell.name())],
        )),
    ]
}

/// 这种 shell 旧版装过的地方，有旧的（或者读不了）的才算。同一个文件（`ZDOTDIR` 就是家目录、指过去的链接）只算一次。
fn find(shell: Shell, env: &impl Fn(&str) -> Option<String>) -> Vec<Spot> {
    let mut known = Vec::new();
    let mut spots = Vec::new();
    for path in shell.old_hooks(env) {
        let seen = if shell == Shell::Fish {
            fish(&path).transpose()
        } else {
            let id = identity(&path);
            if known.contains(&id) {
                continue;
            }
            known.push(id);
            rc(&path, shell).transpose()
        };
        if let Some(seen) = seen {
            spots.push(Spot { path, seen });
        }
    }
    spots
}

/// 启动文件里有没有旧的：不在的、一处都没有的是 `None`。
fn rc(path: &Path, shell: Shell) -> io::Result<Option<Seen>> {
    let Some(text) = install::read(path)? else {
        return Ok(None);
    };
    let (blocks, orphans) = scan(&text, shell);
    Ok(
        (!blocks.is_empty() || !orphans.is_empty()).then_some(Seen::Blocks {
            text,
            blocks,
            orphans,
        }),
    )
}

/// fish 的 `miyu.fish`：不在的是 `None`。
fn fish(path: &Path) -> io::Result<Option<Seen>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let link = meta
        .file_type()
        .is_symlink()
        .then(|| fs::read_link(path))
        .transpose()?;
    let pointed = match fs::metadata(path) {
        Ok(pointed) => pointed,
        // 链接指的文件已经不在了（旧数据根挪走以后剩下的）。
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(link.map(Seen::Link));
        }
        Err(e) => return Err(e),
    };
    if !pointed.is_file() || !fs::read(path)?.starts_with(STAMP.as_bytes()) {
        return Ok(Some(Seen::Foreign));
    }
    Ok(Some(link.map_or(Seen::File, Seen::Link)))
}

/// 认同一个文件：解开链接以后的路径，解不开（不在）的照原样。
fn identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// 找出旧的几段：一段是一个整行（两头的空白、行尾的 `\r` 不算）的结尾记号和它前面最近的那个整行的开头记号之间。
/// 开头后面没等到结尾又来一个开头的，前一个算没有结尾（被人动过的文件里，不从它一路删到后面那一段的结尾，连着中间
/// 用户自己的行）；前面没有开着的开头的结尾不算。交回几段，和没有结尾的那几个开头在第几行。
fn scan(text: &[u8], shell: Shell) -> (Vec<Block>, Vec<usize>) {
    let begin = format!("# >>> miyu {} hook >>>", shell.name());
    let end = format!("# <<< miyu {} hook <<<", shell.name());
    let mut blocks = Vec::new();
    let mut orphans = Vec::new();
    let mut open: Option<(usize, usize)> = None;
    let mut at = 0;
    for (i, line) in text.split_inclusive(|&b| b == b'\n').enumerate() {
        let bare = line.trim_ascii();
        let next = at + line.len();
        if bare == begin.as_bytes() {
            if let Some((first, _)) = open {
                orphans.push(first);
            }
            open = Some((i + 1, at));
        } else if bare == end.as_bytes()
            && let Some((first, start)) = open.take()
        {
            blocks.push(Block {
                first,
                last: i + 1,
                start,
                end: next,
            });
        }
        at = next;
    }
    orphans.extend(open.map(|(first, _)| first));
    (blocks, orphans)
}

/// 去掉那几段，别的字节一个不动。
fn strip(text: &[u8], blocks: &[Block]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0;
    for block in blocks {
        out.extend_from_slice(&text[at..block.start]);
        at = block.end;
    }
    out.extend_from_slice(&text[at..]);
    out
}

/// `status` 的话：这一处还在。
fn told(spot: &Spot, shell: Shell, words: &Words) -> Vec<String> {
    let shown = spot.path.display().to_string();
    match &spot.seen {
        Err(e) => vec![unreadable(&shown, e, words)],
        Ok(Seen::Blocks {
            blocks, orphans, ..
        }) => still(&shown, blocks, shell, words)
            .chain(orphans.iter().map(|&line| unclosed(&shown, line, words)))
            .collect(),
        Ok(Seen::File | Seen::Link(_)) => vec![fill(
            &words.old_file,
            &[("path", &shown), ("shell", shell.name())],
        )],
        Ok(Seen::Foreign) => vec![fill(&words.old_hook, &[("path", &shown)])],
    }
}

/// 几段旧的还在：一段一行。
fn still<'a>(
    shown: &'a str,
    blocks: &'a [Block],
    shell: Shell,
    words: &'a Words,
) -> impl Iterator<Item = String> + 'a {
    blocks.iter().map(move |block| {
        fill(
            &words.old_block,
            &[
                ("path", shown),
                ("first", &block.first.to_string()),
                ("last", &block.last.to_string()),
                ("shell", shell.name()),
            ],
        )
    })
}

/// 清掉了几段：一段一行，带着备份在哪。
fn cleaned<'a>(
    shown: &'a str,
    blocks: &'a [Block],
    backup: &Path,
    words: &'a Words,
) -> impl Iterator<Item = String> + 'a {
    let backup = backup.display().to_string();
    blocks.iter().map(move |block| {
        fill(
            &words.old_cleaned_block,
            &[
                ("path", shown),
                ("first", &block.first.to_string()),
                ("last", &block.last.to_string()),
                ("backup", &backup),
            ],
        )
    })
}

fn unclosed(shown: &str, line: usize, words: &Words) -> String {
    fill(
        &words.old_unclosed,
        &[("path", shown), ("line", &line.to_string())],
    )
}

fn unreadable(shown: &str, e: &io::Error, words: &Words) -> String {
    fill(
        &words.old_unreadable,
        &[("path", shown), ("reason", &e.to_string())],
    )
}

fn failed(shown: &str, e: &io::Error, words: &Words) -> String {
    fill(
        &words.old_failed,
        &[("path", shown), ("reason", &e.to_string())],
    )
}

fn kept(shown: &str, e: &io::Error, backup: &Path, words: &Words) -> String {
    fill(
        &words.old_failed_kept,
        &[
            ("path", shown),
            ("reason", &e.to_string()),
            ("backup", &backup.display().to_string()),
        ],
    )
}

#[cfg(test)]
mod tests;
