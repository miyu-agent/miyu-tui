# miyu shellhook 的 bash 脚本：每次 bash 起来时由 `miyu shellhook script --shell bash` 现印。bash 找不到命令时把这一行交给
# 她（设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

__miyu_shellhook() {
    @PROGRAM@ shellhook "$@"
}

command_not_found_handle() {
    case $- in
        *i*) ;;
        *)
            printf 'bash: %s: command not found\n' "$1" >&2
            return 127
            ;;
    esac
    # $$ 是这个 bash（这里在子 shell 里也不变）：后台回报交回它。
    printf '%s' "$*" | __miyu_shellhook say --shell bash --shell-pid $$
    # 程序收工时不亮光标（亮早了停在最后一行末尾闪一下），这里亮。
    printf '\e[?25h'
    # 收尾行和下一个提示符之间空一行（fish 的提示符自带，bash 的不一定：2026-10-20 验收贴在一起）。
    printf '\n'
    return 127
}

# 回车（同 fish 的 __miyu_accept_line）：回车绑成「先跑 __miyu_accept_line，再执行这一行」两个键。首词是普通词、又不是任何
# 命令（`type` 认不出）的是一句话，当场在这里交给她，把第二个键换成什么都不做（这一下不执行）；多行的交给程序判。别的照
# bash 原样执行。bash 跑之前擦掉了输入的那一行（只擦提示符最后一行）：照报错色重画，答完把提示符前面几行补上（bash 只重画
# 最后一行），提前敲的字放回命令行（实测 bash 5.3）。
__miyu_busy=
__miyu_editing=
__miyu_report_pending=

# 首词：跳过前面的 `变量=值`。
__miyu_head() {
    local -a words
    read -r -a words <<<"$1"
    local word
    for word in "${words[@]}"; do
        [[ $word =~ ^[A-Za-z_][A-Za-z0-9_]*= ]] && continue
        printf '%s' "$word"
        return 0
    done
    return 1
}

# 首词是不是「展开后还是它自己」的普通词：引号、`$`、括号这些会把它换成别的，判不出来，照 bash 原样执行。
__miyu_plain() {
    [[ -n $1 && $1 != *[\'\"\$\(\)~\{\}%\;\&\|\<\>\#^\!\\[:space:]]* ]]
}

__miyu_is_talk() {
    local buffer=$1 head
    [[ -z ${buffer//[[:space:]]/} ]] && return 1
    head=$(__miyu_head "$buffer")
    if [[ $buffer != *$'\n'* ]]; then
        __miyu_plain "$head" && ! type -t -- "$head" >/dev/null 2>&1
        return
    fi
    [[ -n $head ]] && __miyu_plain "$head" && type -t -- "$head" >/dev/null 2>&1 && return 1
    # 多行：交给程序判是命令还是一句话。0 是命令，1 是话，别的（出错）照命令。
    printf '%s' "$buffer" | __miyu_shellhook classify --shell bash 2>/dev/null
    (( $? == 1 ))
}

__miyu_accept_line() {
    if ! __miyu_is_talk "$READLINE_LINE"; then
        __miyu_editing=
        bind '"\C-x\C-b": accept-line'
        return
    fi
    bind '"\C-x\C-b": ""'
    local buffer=$READLINE_LINE redo=
    [[ -n $__miyu_editing ]] && redo=--redo
    __miyu_editing=
    history -s -- "$buffer"
    local prompt=${PS1@P}
    prompt=${prompt//$'\001'/}
    prompt=${prompt//$'\002'/}
    # 交给她的那句话照报错色重画在提示符最后一行后面，折下来的几行缩两格（同 fish）。
    local first=${buffer%%$'\n'*} line
    printf '%s\e[31m%s' "${prompt##*$'\n'}" "$first"
    if [[ $buffer == *$'\n'* ]]; then
        while IFS= read -r line; do
            printf '\n  %s' "$line"
        done <<<"${buffer#*$'\n'}"
    fi
    printf '\e[0m\n'
    local leftover
    leftover=$(mktemp -t miyu-shellhook.XXXXXX)
    __miyu_busy=1
    printf '%s' "$buffer" | __miyu_shellhook say --shell bash --shell-pid $$ --leftover "$leftover" $redo
    printf '\e[?25h\n'
    # 答话时来的后台回报：答完印。
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        __miyu_shellhook report --shell bash --shell-pid $$ && printf '\n'
        printf '\e[?25h'
    fi
    __miyu_busy=
    # bash 只重画提示符最后一行：前面几行补上。
    [[ $prompt == *$'\n'* ]] && printf '%s\n' "${prompt%$'\n'*}"
    READLINE_LINE=
    [[ -s $leftover ]] && READLINE_LINE=$(<"$leftover")
    READLINE_POINT=${#READLINE_LINE}
    # /edit 放回来的：旁边有一份 .redo 标记，记着「在编辑」，下一次回车替换上一轮重来。
    if [[ -e $leftover.redo ]]; then
        __miyu_editing=1
        rm -f -- "$leftover.redo"
    fi
    rm -f -- "$leftover"
}

# Ctrl+V（同 fish）：程序自己读剪贴板，复制的文件、图收成块（`[图片 1]`），大段的字收成 `[已粘贴 14 行]`，插进命令行；
# 说这句话时换回原文、带上附件。读不到剪贴板的不动。
__miyu_paste() {
    local got
    got=$(__miyu_shellhook paste --shell-pid $$ 2>/dev/null) || return
    READLINE_LINE=${READLINE_LINE:0:READLINE_POINT}$got${READLINE_LINE:READLINE_POINT}
    READLINE_POINT=$((READLINE_POINT + ${#got}))
}

# 在编辑的那一句：Ctrl+C 清了命令行、执行了别的命令（都会画新的提示符），就不算在编辑了。
__miyu_prompt() {
    __miyu_editing=
}
PROMPT_COMMAND="__miyu_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

for __miyu_map in emacs vi-insert; do
    bind -m "$__miyu_map" -x '"\C-x\C-a": __miyu_accept_line'
    bind -m "$__miyu_map" '"\C-x\C-b": accept-line'
    bind -m "$__miyu_map" '"\C-m": "\C-x\C-a\C-x\C-b"'
    bind -m "$__miyu_map" '"\C-j": "\C-x\C-a\C-x\C-b"'
    bind -m "$__miyu_map" -x '"\C-v": __miyu_paste'
done
unset __miyu_map

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 bash 发 SIGUSR1。bash 停在提示符上等你敲的时候不办
# 信号，等你回车、这一行读完、命令还没跑的时候才办；前台在跑命令（vim、她正在答）的，等它结束才办（实测 bash 5.3）。
# 那时都不在画提示符：直接印。所以停在提示符上来的回报，要等你下一次回车，印在那条命令前面。
__miyu_report() {
    # 她正在答（回车那一下里）：答完再印。
    if [[ -n $__miyu_busy ]]; then
        __miyu_report_pending=1
        return
    fi
    __miyu_shellhook report --shell bash --shell-pid $$ && printf '\n'
    printf '\e[?25h'
}
trap '__miyu_report' USR1
