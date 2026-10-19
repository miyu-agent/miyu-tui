# miyu shellhook 的 fish 脚本：每次 fish 起来时由 `miyu shellhook script --shell fish` 现印，conf.d 里只有调它的那一行，
# 所以升级以后自动是新的。底子是旧版的 fish 配置（设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

function __miyu_shellhook
    @PROGRAM@ shellhook $argv
end

function __miyu_insert_newline
    commandline -f expand-abbr
    commandline -i \n
end

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
    set -q __miyu_pending_buffer; or return

    set -l buffer $__miyu_pending_buffer
    set -e __miyu_pending_buffer

    trap __miyu_restore_cursor INT TERM EXIT
    __miyu_replay_buffer "$buffer"
    # 清空输入行、重放都画完了：一次交给终端（开头在 __miyu_hand_to_ai）。
    printf '\e[?2026l'
    printf '%s' "$buffer" | __miyu_shellhook say --shell fish
    set -l miyu_status $status
    trap - INT TERM EXIT
    __miyu_restore_cursor
    return $miyu_status
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
            printf '%s' "$current_line" | __miyu_shellhook say --shell fish
            return 127
        end
    end

    set -l command $argv
    test (count $command) -eq 0; and return 127
    set -l text (string join ' ' -- $command)
    string match -qr '[\n\r]' -- $text; and return 127
    printf '%s' "$text" | __miyu_shellhook say --shell fish
    return 127
end
