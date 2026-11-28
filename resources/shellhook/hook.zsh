# miyu shellhook 的 zsh 脚本：每次 zsh 起来时由 `miyu shellhook script --shell zsh` 现印。zsh 找不到命令时把这一行交给她
# （设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序，
# @HOOK@ 换成这一版脚本的记号（开着的 shell 升级以后还是旧脚本：程序看记号对不上的说一声重开）。

__miyu_shellhook() {
    @PROGRAM@ shellhook --hook @HOOK@ "$@"
}

# 找不到命令、后台回报这几条路上提前敲的字（2026-11-28 审计 F3）：程序先写进这个 shell 在信箱目录里的一份，回到命令行时
# （开新的一行、停在提示符上印完回报、答完）由 __miyu_put_back 放回去。找不到命令的处理函数跑在为这条命令分出来的子进程里，
# 原来的 `print -z` 推进的是子进程的缓冲栈，字丢了。认不出信箱目录的（@SPOOL@ 是空的）不收。起来时清掉同一个进程号留下的旧的。
typeset -g __miyu_typed_file=
[[ -n @SPOOL@ ]] && __miyu_typed_file=@SPOOL@/$$-typed
[[ -e $__miyu_typed_file ]] && command rm -f -- $__miyu_typed_file

# 给程序的「提前敲的字写到哪」，信箱目录不在的先建。
__miyu_typed_args() {
    reply=()
    [[ -n $__miyu_typed_file ]] || return 0
    [[ -d ${__miyu_typed_file:h} ]] || mkdir -p -- ${__miyu_typed_file:h}
    reply=(--leftover "$__miyu_typed_file")
}

# 放回那一份里的字：接在光标前面（小部件里调）。
__miyu_put_back() {
    [[ -e $__miyu_typed_file ]] || return 0
    LBUFFER+=$(<$__miyu_typed_file)
    command rm -f -- $__miyu_typed_file
}

command_not_found_handler() {
    # 只接人在提示符上敲、回车交给 zsh 执行的那一行（`__miyu_ran`），找不到的是那一行的首词、输出直接到终端的才交给她：
    # 启动文件、提示符、precmd、命令替换、管道、函数和脚本里找不到的命令照 zsh 原样报错（2026-11-28 审计 F1：原来只看是不是
    # 交互式，提示符里调了没装的命令的每画一次提示符就交一轮）。
    if [[ ! -o interactive || ! -t 1 || -z $__miyu_ran || $1 != "${(Q)$(__miyu_head "$__miyu_ran")}" ]]; then
        print -u2 "zsh: command not found: $1"
        return 127
    fi
    # 她答话的时候提前敲的字：程序写进这个 shell 的那一份（上面），开新的一行时放回去。$$ 告诉它是哪个 shell：后台回报交回这里。
    local -a reply
    __miyu_typed_args
    printf '%s' "$*" | __miyu_shellhook say --shell zsh --shell-pid $$ "${reply[@]}"
    # 程序收工时不亮光标（亮早了停在最后一行末尾闪一下），这里亮。
    printf '\e[?25h'
    # 收尾行和下一个提示符之间空一行（fish 的提示符自带，zsh、bash 的不一定：2026-10-20 验收贴在一起）。
    print
    return 127
}

# 回车（同 fish 的 __miyu_accept_line）：首词是普通词、又不是任何命令（外部命令、函数、别名、内建、保留字）的是一句话，
# 当场在这个小部件里交给她（同 fzf 的小部件：先 `zle -I`，程序自己用终端）；多行的交给程序判。别的照 zsh 原样执行。
# 交给她的那一行留在屏幕上、照普通的字色，进历史；她答完，提前敲的字放回命令行。
typeset -g __miyu_busy= __miyu_editing=
# 回车交给 zsh 执行的那一行：找不到命令时照它认是不是人敲的（precmd 里清掉）。
typeset -g __miyu_ran=

# 首词：跳过前面的 `变量=值`。
__miyu_head() {
    local -a words=(${(z)1})
    local word
    for word in $words; do
        [[ $word == [A-Za-z_]*=* ]] && continue
        print -r -- $word
        return 0
    done
    return 1
}

# 首词是不是「展开后还是它自己」的普通词：引号、`$`、括号这些会把它换成别的，判不出来，照 zsh 原样执行。通配符故意不在
# 名单里：命令位上出现通配符本来就说明这不是命令名（`[图片 1]` 这类块就是这样认出来的）。
__miyu_plain() {
    [[ -n $1 && $1 != *[\'\"\$\(\)~\{\}%\;\&\|\<\>\#^\!\\[:space:]]* ]]
}

# 首词是不是命令：`whence` 认得出。单独一个 `-` 不算（zsh 当它是前缀修饰，fish 不认；2026-10-22 验收：「- 说的……」
# 这种多行话被当成脚本一行行跑了）。多行的，`time`、`test` 这几个常出现在话里的也不算，交给程序判（同 fish）。
__miyu_is_command() {
    [[ $1 == - ]] && return 1
    [[ -n $2 && " time test date which type command history " == *" $1 "* ]] && return 1
    whence -- "$1" >/dev/null 2>&1
}

__miyu_accept_line() {
    local buffer=$BUFFER
    if [[ -z ${buffer//[[:space:]]/} ]]; then
        zle .accept-line
        return
    fi
    # /edit 放回来的那一句（在编辑）：不管首词像不像命令都交给她（2026-11-28 审计 F2：在全屏里说的「touch 报错日志 之后……」
    # 放回来，原来先判成命令、回车就执行了）。
    if [[ -n $__miyu_editing ]]; then
        __miyu_hand_to_ai "$buffer"
        return
    fi
    local head
    head=$(__miyu_head "$buffer")
    __miyu_ran=$buffer
    if [[ $buffer != *$'\n'* ]]; then
        if ! __miyu_plain "$head" || __miyu_is_command "$head"; then
            zle .accept-line
            return
        fi
    else
        if [[ -n $head ]] && __miyu_plain "$head" && __miyu_is_command "$head" multi; then
            zle .accept-line
            return
        fi
        # 多行：交给程序判是命令还是一句话。0 是命令，1 是话，别的（出错）照命令。
        printf '%s' "$buffer" | __miyu_shellhook classify --shell zsh 2>/dev/null
        if (( $? != 1 )); then
            zle .accept-line
            return
        fi
    fi
    __miyu_hand_to_ai "$buffer"
}

__miyu_hand_to_ai() {
    local buffer=$1 redo=
    [[ -n $__miyu_editing ]] && redo=--redo
    __miyu_editing=
    print -rs -- "$buffer"
    # 留在屏幕上的那一行照普通的字色（2026-10-23 项目主人：「让文字别回车后全部变成红色了」，原来照报错色）：语法高亮插件标的颜色也盖掉。
    region_highlight=("0 ${#BUFFER} fg=default")
    zle -R
    zle -I
    print
    local leftover
    leftover=$(mktemp -t miyu-shellhook.XXXXXX)
    __miyu_busy=1
    printf '%s' "$buffer" | __miyu_shellhook say --shell zsh --shell-pid $$ --leftover "$leftover" $redo
    printf '\e[?25h'
    print
    # 答话时来的后台回报：答完印。zsh 办完那个信号会把提示符和这一行重画一遍（画在她答的下面），再让出一次画面，印的和
    # 放回去的字才接得上（2026-11-28 审计 F3 实测：不让的，放回去的字画在没有提示符的一行上）。
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        zle -I
        local -a reply
        __miyu_typed_args
        __miyu_shellhook report --shell zsh --shell-pid $$ "${reply[@]}" && print
        printf '\e[?25h'
    fi
    __miyu_busy=
    region_highlight=()
    BUFFER=
    [[ -s $leftover ]] && BUFFER=$(<"$leftover")
    CURSOR=${#BUFFER}
    # 印回报时敲的：接在后面。
    __miyu_put_back
    # /edit 放回来的：旁边有一份 .redo 标记，记着「在编辑」，下一次回车替换上一轮重来。
    if [[ -e $leftover.redo ]]; then
        __miyu_editing=1
        rm -f -- "$leftover.redo"
    fi
    rm -f -- "$leftover"
}
zle -N accept-line __miyu_accept_line
zle -N __miyu_put_back

# 在编辑的那一句：Ctrl+C 清了命令行、执行了别的命令（都开新的一行），就不算在编辑了。
__miyu_line_init() {
    __miyu_editing=
    __miyu_put_back
}
autoload -Uz add-zle-hook-widget
add-zle-hook-widget line-init __miyu_line_init

# Ctrl+V（同 fish）：程序自己读剪贴板，复制的文件、图收成块（`[图片 1]`），大段的字收成 `[已粘贴 14 行]`，插进命令行；
# 说这句话时换回原文、带上附件。读不到剪贴板的不动。
__miyu_paste() {
    local got
    got=$(__miyu_shellhook paste --shell-pid $$ 2>/dev/null) || return
    LBUFFER+=$got
}
zle -N __miyu_paste
bindkey '^V' __miyu_paste
bindkey -M viins '^V' __miyu_paste

# Ctrl+J 在命令行里换行（同 fish），敲多行的话。
__miyu_newline() {
    LBUFFER+=$'\n'
}
zle -N __miyu_newline
bindkey '^J' __miyu_newline
bindkey -M viins '^J' __miyu_newline

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 zsh 发 SIGUSR1。停在提示符上的（zle 开着）当场印：
# 信箱里有要印的才 `zle -I`（提示符画到印的下面），程序往上擦掉旧的提示符在那里印；前台在跑命令、她正在答的，zsh 等它
# 结束才办这个信号，那时 zle 没开，记着，回到提示符（画提示符以前）再印（实测 zsh 5.9）。
typeset -g __miyu_report_pending=
TRAPUSR1() {
    if zle && [[ -z $__miyu_busy ]]; then
        __miyu_shellhook report --shell zsh --shell-pid $$ --pending || return 0
        zle -I
        # 提示符占几行：先拆成数组再数（只有一行时 `${#${(f)…}}` 数的是字数，往上擦多了）。
        local -a lines=("${(@f)${(%%)PS1}}")
        local rows=${#lines}
        local -a reply
        __miyu_typed_args
        __miyu_shellhook report --shell zsh --shell-pid $$ --above $rows "${reply[@]}" && print
        printf '\e[?25h'
        # 印的时候敲的：放回命令行（信号里调小部件，同 `zle reset-prompt` 那种用法）。
        zle __miyu_put_back
    else
        __miyu_report_pending=1
    fi
    return 0
}

__miyu_precmd() {
    __miyu_ran=
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        local -a reply
        __miyu_typed_args
        __miyu_shellhook report --shell zsh --shell-pid $$ "${reply[@]}" && print
        printf '\e[?25h'
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd __miyu_precmd
