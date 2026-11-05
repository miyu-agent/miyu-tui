//! Windows 读复制的文件、图（蓝图 `tui.md`「输入框」第 12 条「从哪来」，2026-10-26 Windows 虚拟机里实测：原来 Windows 上
//! 一律当没有图，截图贴不进来）：问 Windows 自带的 PowerShell，一次问齐（起一次 PowerShell 要小半秒，冷启动一秒）：
//! 复制的文件交回路径，一行一个；有图的转成 PNG、交 base64（管道里的字节照控制台代码页转，原样的 PNG 会坏）。
//! 读剪贴板里的图要 STA，5.1 默认就是，`-Sta` 写明。
//!
//! 调用的先问复制的文件、再问图：问文件时读到的是图的，留给紧接着的那一问，不再起一次 PowerShell。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;

use super::read::{run_bytes, wait};

/// 问剪贴板的那段脚本：只用单引号（5.1 把参数交给别的程序时吞掉双引号）。输出 UTF-8（文件名里的中文），不带 BOM。
const SCRIPT: &str = "[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false; \
    Add-Type -AssemblyName System.Windows.Forms, System.Drawing; \
    $c = [Windows.Forms.Clipboard]; \
    if ($c::ContainsFileDropList()) { 'files'; $c::GetFileDropList() } \
    elseif ($c::ContainsImage()) { $m = New-Object IO.MemoryStream; \
    $c::GetImage().Save($m, [Drawing.Imaging.ImageFormat]::Png); 'image'; [Convert]::ToBase64String($m.ToArray()) }";

/// 读到的。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Grabbed {
    /// 复制的文件：路径，一行一个。
    Files(String),
    /// 图：PNG。
    Image(Vec<u8>),
}

/// 问文件时读到的图，留给紧接着问图的那一下；过了这么久的不算（剪贴板可能换了）。
const KEEP: Duration = Duration::from_secs(2);

static LAST_IMAGE: Mutex<Option<(Instant, Vec<u8>)>> = Mutex::new(None);

/// 复制的文件：路径，一行一个。读到的是图的记下，交回 `None`。
pub(super) fn files() -> Option<String> {
    match ask()? {
        Grabbed::Files(list) => Some(list),
        Grabbed::Image(png) => {
            if let Ok(mut last) = LAST_IMAGE.lock() {
                *last = Some((Instant::now(), png));
            }
            None
        }
    }
}

/// 剪贴板里的图（PNG）：刚问文件时读到的直接用，没有的再问一次。
pub(super) fn image() -> Option<Vec<u8>> {
    let kept = LAST_IMAGE.lock().ok().and_then(|mut last| last.take());
    if let Some((at, png)) = kept
        && at.elapsed() < KEEP
    {
        return Some(png);
    }
    match ask()? {
        Grabbed::Image(png) => Some(png),
        Grabbed::Files(_) => None,
    }
}

fn ask() -> Option<Grabbed> {
    let out = run_bytes(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Sta", "-Command", SCRIPT],
        wait(),
    )?;
    parse(&String::from_utf8_lossy(&out))
}

/// 读脚本的输出：头一行说是哪种，下面是内容。
pub(super) fn parse(out: &str) -> Option<Grabbed> {
    let mut lines = out.lines().map(str::trim).filter(|l| !l.is_empty());
    match lines.next()? {
        "files" => {
            let list: Vec<&str> = lines.collect();
            (!list.is_empty()).then(|| Grabbed::Files(list.join("\n")))
        }
        "image" => {
            let png = base64::engine::general_purpose::STANDARD
                .decode(lines.collect::<String>())
                .ok()?;
            (!png.is_empty()).then_some(Grabbed::Image(png))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Grabbed, parse};

    #[test]
    fn files_come_back_one_per_line_and_images_decode() {
        assert_eq!(
            parse("files\r\nC:\\Users\\me\\a.png\r\nC:\\图\\b.jpg\r\n"),
            Some(Grabbed::Files("C:\\Users\\me\\a.png\nC:\\图\\b.jpg".into()))
        );
        assert_eq!(
            parse("image\r\niVBORw0K\r\n"),
            Some(Grabbed::Image(b"\x89PNG\r\n".to_vec()))
        );
        assert_eq!(parse(""), None, "剪贴板里是字、空的：什么都不印");
        assert_eq!(parse("image\r\n!!\r\n"), None);
    }
}
