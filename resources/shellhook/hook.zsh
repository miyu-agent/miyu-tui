# miyu shellhook 的 zsh 脚本：每次 zsh 起来时由 `miyu shellhook script --shell zsh` 现印。zsh 找不到命令时把这一行交给她
# （设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

__miyu_shellhook() {
    @PROGRAM@ shellhook "$@"
}

command_not_found_handler() {
    if [[ ! -o interactive ]]; then
        print -u2 "zsh: command not found: $1"
        return 127
    fi
    # 她答话的时候提前敲的字：程序写进这个文件，答完放到下一个提示符的命令行上。$$ 告诉它是哪个 shell：后台回报交回这里。
    local leftover
    leftover=$(mktemp -t miyu-shellhook.XXXXXX)
    printf '%s' "$*" | __miyu_shellhook say --shell zsh --shell-pid $$ --leftover "$leftover"
    # 程序收工时不亮光标（亮早了停在最后一行末尾闪一下），这里亮。
    printf '\e[?25h'
    # 收尾行和下一个提示符之间空一行（fish 的提示符自带，zsh、bash 的不一定：2026-10-20 验收贴在一起）。
    print
    [[ -s $leftover ]] && print -z -- "$(<$leftover)"
    rm -f -- "$leftover"
    return 127
}

# 回车（同 fish 的 __miyu_accept_line）：首词是普通词、又不是任何命令（外部命令、函数、别名、内建、保留字）的是一句话，
# 当场在这个小部件里交给她（同 fzf 的小部件：先 `zle -I`，程序自己用终端）；多行的交给程序判。别的照 zsh 原样执行。
# 交给她的那一行留在屏幕上、照报错色，进历史；她答完，提前敲的字放回命令行。
typeset -g __miyu_busy= __miyu_editing=

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
    local head
    head=$(__miyu_head "$buffer")
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
    # 留在屏幕上的那一行照报错色。
    region_highlight=("0 ${#BUFFER} fg=red")
    zle -R
    zle -I
    print
    local leftover
    leftover=$(mktemp -t miyu-shellhook.XXXXXX)
    __miyu_busy=1
    printf '%s' "$buffer" | __miyu_shellhook say --shell zsh --shell-pid $$ --leftover "$leftover" $redo
    printf '\e[?25h'
    print
    # 答话时来的后台回报：答完印。
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        __miyu_shellhook report --shell zsh --shell-pid $$ && print
        printf '\e[?25h'
    fi
    __miyu_busy=
    region_highlight=()
    BUFFER=
    [[ -s $leftover ]] && BUFFER=$(<"$leftover")
    CURSOR=${#BUFFER}
    # /edit 放回来的：旁边有一份 .redo 标记，记着「在编辑」，下一次回车替换上一轮重来。
    if [[ -e $leftover.redo ]]; then
        __miyu_editing=1
        rm -f -- "$leftover.redo"
    fi
    rm -f -- "$leftover"
}
zle -N accept-line __miyu_accept_line

# 在编辑的那一句：Ctrl+C 清了命令行、执行了别的命令（都开新的一行），就不算在编辑了。
__miyu_line_init() {
    __miyu_editing=
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
        __miyu_shellhook report --shell zsh --shell-pid $$ --above $rows && print
        printf '\e[?25h'
    else
        __miyu_report_pending=1
    fi
    return 0
}

__miyu_precmd() {
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        __miyu_shellhook report --shell zsh --shell-pid $$ && print
        printf '\e[?25h'
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd __miyu_precmd
