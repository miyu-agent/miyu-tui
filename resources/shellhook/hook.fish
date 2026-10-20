# miyu shellhook 的 fish 脚本：每次 fish 起来时由 `miyu shellhook script --shell fish` 现印，conf.d 里只有调它的那一行，
# 所以升级以后自动是新的。底子是旧版的 fish 配置（设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

function __miyu_shellhook
    @PROGRAM@ shellhook $argv
end

function __miyu_insert_newline
    commandline -f expand-abbr
    commandline -i \n
end

# Ctrl+V：程序自己读剪贴板，复制的文件、图收成块（`[图片 1]`），大段的字收成 `[已粘贴 14 行]`，插进命令行；说这句话时
# 换回原文、带上附件（同终端界面「输入框」第 11、12 条）。读不到剪贴板的照 fish 自己的粘贴。
function __miyu_paste
    # 命令替换按行拆开：拼回去（`set` 之后的 $status 是命令替换的）。
    set -l got (__miyu_shellhook paste --shell-pid $fish_pid 2>/dev/null)
    if test $status -eq 0
        commandline -i -- (string join \n -- $got)
    else
        fish_clipboard_paste
    end
end

bind ctrl-v __miyu_paste
bind \cv __miyu_paste
bind -M insert ctrl-v __miyu_paste
bind -M insert \cv __miyu_paste

bind ctrl-j __miyu_insert_newline
bind \cj __miyu_insert_newline
bind -M insert ctrl-j __miyu_insert_newline
bind -M insert \cj __miyu_insert_newline

function __miyu_wrap_fish_prompt
    functions -q __miyu_original_fish_prompt; and return
    functions -q fish_prompt; or fish_prompt >/dev/null 2>/dev/null
    functions -q fish_prompt; or return

    functions -c fish_prompt __miyu_original_fish_prompt
    function fish_prompt
        if set -q __miyu_pending_buffer
            printf '\e[?25l'
        end
        __miyu_original_fish_prompt
    end
end

# 交给她的那句话在提示符那一行原样重放一遍（照 fish 的报错色），上面留着你说了什么。
function __miyu_replay_buffer
    set -l buffer $argv[1]
    set -l lines (string split \n -- "$buffer")
    if test (count $lines) -gt 0
        set -l prompt (fish_prompt | string collect -N)
        set -l prompt_lines (string split \n -- "$prompt")
        set -l prompt_col (math (string length --visible -- "$prompt_lines[-1]") + 1)
        printf '\e[?25l'
        printf '\e[1A\e[%sG' $prompt_col
        if not set -q fish_color_error; or not set_color $fish_color_error 2>/dev/null
            set_color red
        end
        printf '%s\n' "$lines[1]"
        for line in $lines[2..-1]
            printf '  %s\n' "$line"
        end
        set_color normal
    end
end

function __miyu_restore_cursor
    # 交给她时开的同步输出：出错、被打断也要关上，不然认它的终端会一直等。
    printf '\e[?2026l\e[?25h'
end

function __miyu_on_prompt --on-event fish_prompt
    set -l miyu_status 0
    # 这一整段都算「在忙」：她答完时来的信号 fish 在 say 退出后的第一个空当办，那时还没画提示符（实测）。
    set -g __miyu_busy 1
    if set -q __miyu_pending_buffer
        set -l buffer $__miyu_pending_buffer
        set -e __miyu_pending_buffer

        trap __miyu_restore_cursor INT TERM EXIT
        __miyu_replay_buffer "$buffer"
        # 清空输入行、重放都画完了：一次交给终端（开头在 __miyu_hand_to_ai）。
        printf '\e[?2026l'
        # 她答话的时候提前敲的字：程序写进这个文件，答完放回命令行。$fish_pid 告诉它是哪个 shell：后台回报交回这里。
        set -l leftover (mktemp -t miyu-shellhook.XXXXXX)
        printf '%s' "$buffer" | __miyu_shellhook say --shell fish --shell-pid $fish_pid --leftover $leftover
        set miyu_status $status
        trap - INT TERM EXIT
        __miyu_restore_cursor
        if test -s "$leftover"
            commandline -r -- (string collect < $leftover)
        end
        rm -f -- $leftover
    end
    # 前台跑着命令的时候来的后台回报：回到提示符再印（提示符还没画，不用擦）。
    while set -q __miyu_report_pending
        __miyu_report
    end
    set -e __miyu_busy
    # 刚才那一下空当里来的。
    while set -q __miyu_report_pending
        __miyu_report
    end
    return $miyu_status
end

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 fish 发 SIGUSR1。停在提示符上的当场印；前台在跑
# 命令的（vim、一行里的几条命令）、她正在答的先记着，回到提示符再印。她答话时来的信号 fish 等答完才办，那时
# current-command 已经是 fish，只看它会当成停在提示符上、往上擦掉她刚印的收尾行（2026-10-20 验收）：答话期间记着
# __miyu_busy。
function __miyu_report_signal --on-signal SIGUSR1
    if not set -q __miyu_busy; and test (status current-command) = fish
        # 提示符已经画着：有要印的，程序回到提示符的第一行、往下清掉，在那里印。fish 重画时从光标所在往上回「提示符行数 - 1」
        # 行：印完补上这么多个换行，它才画在印的下面，不盖掉（实测）。没有要印的什么都不动。敲到一半的字照旧在。
        set -l rows (math (count (fish_prompt)) - 1)
        __miyu_report $rows; or return
        # 印的时候又来的：接着印。
        while set -q __miyu_report_pending
            __miyu_report
        end
        test $rows -gt 0; and printf '%.0s\n' (seq $rows)
        commandline -f repaint
    else
        set -g __miyu_report_pending 1
    end
end

# 印信箱里的。给了行数的是停在提示符上：程序有要印的才擦掉提示符；没有要印的交回 1。
function __miyu_report
    set -e __miyu_report_pending
    set -l above
    test (count $argv) -gt 0; and set above --above $argv[1]
    # 印的时候敲的字：接在命令行后面（原来敲到一半的 fish 自己留着）。
    set -l leftover (mktemp -t miyu-shellhook.XXXXXX)
    set -l was_busy (set -q __miyu_busy; and echo 1)
    set -g __miyu_busy 1
    __miyu_shellhook report --shell fish --shell-pid $fish_pid --leftover $leftover $above
    set -l printed $status
    test -n "$was_busy"; or set -e __miyu_busy
    if test -s "$leftover"
        commandline -i -- (string collect < $leftover)
    end
    rm -f -- $leftover
    return $printed
end

function __miyu_execute_or_continue
    commandline --is-valid
    if test $status -eq 2
        commandline -i \n
        commandline -f repaint
    else
        commandline -f execute
    end
end

function __miyu_buffer_is_multiline
    test (string split \n -- "$argv[1]" | count) -gt 1
end

function __miyu_first_command
    set -l tokens (commandline --input="$argv[1]" --tokens-expanded 2>/dev/null)
    while test (count $tokens) -gt 0
        set -l token $tokens[1]
        if string match -qr '^[A-Za-z_][A-Za-z0-9_]*=' -- "$token"
            set -e tokens[1]
            continue
        end
        printf '%s' "$token"
        return 0
    end
    return 1
end

# 只看首词长什么样，所以取没展开的原文：--tokens-expanded 会真的跑命令替换，放在每次回车上按不得。老版本 fish 不认
# --tokens-raw，拿不到就当判不出来。
function __miyu_first_token_raw
    set -l tokens (commandline --input="$argv[1]" --tokens-raw 2>/dev/null)
    while test (count $tokens) -gt 0
        set -l token $tokens[1]
        if string match -qr '^[A-Za-z_][A-Za-z0-9_]*=' -- "$token"
            set -e tokens[1]
            continue
        end
        printf '%s' "$token"
        return 0
    end
    return 1
end

# 首词是不是一个「展开后还是它自己」的普通词。$ ( ) ~ { } % 引号、反斜杠会把首词换成别的东西，判不出来就交回 fish；
# ; & | < > # ^ ! 和空白同理。通配符 * ? [ ] 故意不在名单里：命令位上出现通配符，本来就说明这不是命令名。不能用 \w
# 白名单：fish 的正则里 \w 只认 ASCII，中文会被当成元字符。
function __miyu_head_is_plain_word
    test -n "$argv[1]"; or return 1
    string match -qr '[\x27"$()~{}%;&|<>#^!\x5c\s]' -- "$argv[1]"; and return 1
    return 0
end

function __miyu_hand_to_ai
    __miyu_wrap_fish_prompt
    history append -- "$argv[1]"
    set -g __miyu_pending_buffer "$argv[1]"
    # 同步输出：fish 清空输入行会先擦掉长文，再由 __miyu_on_prompt 重放，中间那一下就是闪。从这里起终端先不画，重放完一起画。
    printf '\e[?2026h\e[?25l'
    commandline -b -- ""
    commandline -f execute
end

function __miyu_accept_line
    status is-interactive; or return

    commandline -f expand-abbr
    set -l buffer (commandline -b | string collect)
    set -l trimmed (string trim -- "$buffer")
    if test -z "$trimmed"
        __miyu_execute_or_continue
        return
    end

    if not __miyu_buffer_is_multiline "$buffer"
        # 单行本来靠 fish_command_not_found 兜底，但 fish 是先展开再找命令：话里带个没匹配上的通配符，fish 在展开时就报错，
        # 兜底永远不触发。首词是普通词又不是任何命令时提前接管。
        set -l head (__miyu_first_token_raw "$buffer")
        # Ctrl+V 贴的块（`[已粘贴 14 行]`、`[图片 1]`）带空格，fish 把方括号连同里面的空格算进一个词（实测）：方括号以前
        # 的才是首词，块打头的是话。
        if string match -q -- '*[*' "$head"; and string match -qr '\s' -- "$head"
            set head (string replace -r '\[.*' '' -- "$head")
            if test -z "$head"
                __miyu_hand_to_ai "$buffer"
                return
            end
        end
        if not __miyu_head_is_plain_word "$head"; or type -q -- "$head"
            __miyu_execute_or_continue
            return
        end
        __miyu_hand_to_ai "$buffer"
        return
    end

    set -l first_command (__miyu_first_command "$buffer")
    if test -n "$first_command"; and not contains -- "$first_command" time test date which type command history; and type -q -- "$first_command"
        __miyu_execute_or_continue
        return
    end

    # 多行：交给程序判是命令还是一句话。0 是命令，1 是话，别的（出错）照命令交回 fish。
    printf '%s' "$buffer" | __miyu_shellhook classify --shell fish 2>/dev/null
    if test $status -eq 1
        __miyu_hand_to_ai "$buffer"
        return
    end
    __miyu_execute_or_continue
end

bind enter __miyu_accept_line
bind \r __miyu_accept_line
bind -M insert enter __miyu_accept_line
bind -M insert \r __miyu_accept_line

function fish_command_not_found
    status is-interactive; or return 127

    set -l current_line (status current-commandline 2>/dev/null | string collect)
    if test -n "$current_line"; and not string match -qr '[\n\r]' -- "$current_line"
        set -l top_command (__miyu_first_command "$current_line")
        if test -z "$top_command"; or not type -q -- "$top_command"
            printf '%s' "$current_line" | __miyu_shellhook say --shell fish --shell-pid $fish_pid
            return 127
        end
    end

    set -l command $argv
    test (count $command) -eq 0; and return 127
    set -l text (string join ' ' -- $command)
    string match -qr '[\n\r]' -- $text; and return 127
    printf '%s' "$text" | __miyu_shellhook say --shell fish --shell-pid $fish_pid
    return 127
end
