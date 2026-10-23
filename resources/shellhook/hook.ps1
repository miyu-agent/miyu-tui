# miyu shellhook 的 PowerShell 脚本（pwsh 和 Windows PowerShell 5.1 共用）：每次 PowerShell 起来时由
# `miyu shellhook script --shell pwsh`（或 `powershell`）现印。回车先判是命令还是一句话，话当场交给她（设计 22 第七节）。
# @PROGRAM@ 在印的时候换成装的时候记下的程序，@SHELL@ 换成 shell 的名字，@SPOOL@ 换成后台回报的信箱目录。
# 要 PSReadLine（5.1 自带 2.0，pwsh 自带新的）：没有的不接管回车，照 PowerShell 原样。

if (-not (Get-Module PSReadLine)) {
    return
}

$global:__miyu_editing = $false
$global:__miyu_esc = [char]27

# 调程序：它印的字照 UTF-8 读（PowerShell 照 `[Console]::OutputEncoding` 解码，5.1 默认是系统代码页）。
function global:__miyu_capture {
    $saved = [Console]::OutputEncoding
    try {
        [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
        $out = & @PROGRAM@ shellhook @args
        return , @($out)
    } finally {
        [Console]::OutputEncoding = $saved
    }
}

# 调程序，让它直接用这个控制台，交回退出码。不能用 `&`：按键处理、提示符函数里 `&` 调的程序输出被收进管道，画面不上
# 控制台；它拉起的核心继承了那根管道，PowerShell 等管道关上，一直卡着（2026-10-23 在 Windows 虚拟机里实测）。
function global:__miyu_run([string[]] $arguments) {
    $quoted = $arguments | ForEach-Object { '"' + ($_ -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"' }
    $info = [Diagnostics.ProcessStartInfo]::new(@PROGRAM@, ((@('shellhook') + $quoted) -join ' '))
    $info.UseShellExecute = $false
    $process = [Diagnostics.Process]::Start($info)
    $process.WaitForExit()
    return $process.ExitCode
}

# 这一段是不是一句话：照 PowerShell 自己的词法，头一个词（跳过注释、换行）是裸词、当命令名用、`Get-Command` 又认不出的是话
# （「你好」「解释一下这个报错」）。单独一个 `-` 不算（同别的 shell）。变量、表达式、关键字、路径、认得出的命令都照原样执行。
function global:__miyu_is_talk([string] $buffer) {
    if ([string]::IsNullOrWhiteSpace($buffer)) {
        return $false
    }
    $tokens = $null
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseInput($buffer, [ref]$tokens, [ref]$errors)
    $head = $tokens | Where-Object { $_.Kind -ne 'NewLine' -and $_.Kind -ne 'Comment' -and $_.Kind -ne 'LineContinuation' } | Select-Object -First 1
    if ($null -eq $head -or $head.Kind -eq 'EndOfInput') {
        return $false
    }
    $bare = ($head.Kind -eq 'Generic' -or $head.Kind -eq 'Identifier') -and
        ($head.TokenFlags -band [System.Management.Automation.Language.TokenFlags]::CommandName)
    if (-not $bare -or $head.Text -eq '-') {
        return $false
    }
    return -not (Get-Command -Name $head.Text -ErrorAction Ignore)
}

# 回车：是话的当场交给她。命令行先清掉（PSReadLine 照它记着的重画），那句话照报错色重画在提示符后面，折下来的几行缩两格
# （同 fish）；答完在新的一行重画提示符，提前敲的字放回命令行。
Set-PSReadLineKeyHandler -Key Enter -BriefDescription MiyuAcceptLine -ScriptBlock {
    $buffer = $null
    $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$buffer, [ref]$cursor)
    if (-not (__miyu_is_talk $buffer)) {
        $global:__miyu_editing = $false
        [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
        return
    }
    $redo = $global:__miyu_editing
    $global:__miyu_editing = $false
    [Microsoft.PowerShell.PSConsoleReadLine]::AddToHistory($buffer)
    [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
    $e = $global:__miyu_esc
    $lines = $buffer -split "`r?`n"
    [Console]::Write("$e[31m" + $lines[0])
    foreach ($line in ($lines | Select-Object -Skip 1)) {
        [Console]::Write("`n  " + $line)
    }
    [Console]::Write("$e[0m`n")
    # 那句话写进文件交给程序（不走管道：5.1 照 `$OutputEncoding` 编码，默认 ASCII，中文全成问号）。
    $sentence = [IO.Path]::GetTempFileName()
    $leftover = [IO.Path]::GetTempFileName()
    [IO.File]::WriteAllText($sentence, $buffer, [Text.UTF8Encoding]::new($false))
    $arguments = @('say', '--shell', '@SHELL@', '--shell-pid', "$PID", '--input', $sentence, '--leftover', $leftover)
    if ($redo) {
        $arguments += '--redo'
    }
    [void](__miyu_run $arguments)
    # 程序收工时不亮光标（亮早了停在最后一行末尾闪一下），这里亮；收尾行和下一个提示符之间空一行。
    [Console]::Write("$e[?25h`n")
    __miyu_report_now
    $back = ''
    if ((Test-Path -LiteralPath $leftover) -and (Get-Item -LiteralPath $leftover).Length -gt 0) {
        $back = [IO.File]::ReadAllText($leftover, [Text.UTF8Encoding]::new($false))
    }
    # /edit 放回来的：旁边有一份 .redo 标记，记着「在编辑」，下一次回车替换上一轮重来。
    $edit = Test-Path -LiteralPath "$leftover.redo"
    Remove-Item -LiteralPath $sentence, $leftover, "$leftover.redo" -ErrorAction Ignore
    [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt($null, [Console]::CursorTop)
    if ($back) {
        [Microsoft.PowerShell.PSConsoleReadLine]::Insert($back)
    }
    # 重画提示符时外面包的那一层把「在编辑」清了：画完再记。
    $global:__miyu_editing = $edit
}

# Ctrl+V（同 fish）：程序自己读剪贴板，复制的文件、图收成块（`[图片 1]`），大段的字收成 `[已粘贴 14 行]`，插进命令行；
# 说这句话时换回原文、带上附件。读不到剪贴板的照 PowerShell 原来的粘贴。
Set-PSReadLineKeyHandler -Chord Ctrl+v -BriefDescription MiyuPaste -ScriptBlock {
    $got = __miyu_capture paste --shell-pid $PID
    if ($LASTEXITCODE -eq 0 -and $got.Count -gt 0) {
        [Microsoft.PowerShell.PSConsoleReadLine]::Insert(($got -join "`n"))
    } else {
        [Microsoft.PowerShell.PSConsoleReadLine]::Paste()
    }
}

# Ctrl+J 在命令行里换行（同 fish），敲多行的话。
Set-PSReadLineKeyHandler -Chord Ctrl+j -Function AddLine

# 后台回报（设计 22 第七节「后台任务和回报」）：PowerShell 没有信号可等，每次画提示符时看一眼这个 shell 的信箱，有的印出来。
# 停在提示符上来的回报要等下一次画提示符（回车、Ctrl+C）才印（同 bash）。
function global:__miyu_report_now {
    $box = Join-Path @SPOOL@ "$PID.jsonl"
    if ((Test-Path -LiteralPath $box) -and (Get-Item -LiteralPath $box).Length -gt 0) {
        if ((__miyu_run @('report', '--shell', '@SHELL@', '--shell-pid', "$PID")) -eq 0) {
            [Console]::Write("`n")
        }
        [Console]::Write("$($global:__miyu_esc)[?25h")
    }
}

# 提示符外面包一层：画新的提示符就是执行了别的命令、Ctrl+C 清了命令行，不算在编辑了；顺带印后台回报。
$global:__miyu_original_prompt = $function:prompt
function global:prompt {
    $global:__miyu_editing = $false
    __miyu_report_now
    & $global:__miyu_original_prompt
}
