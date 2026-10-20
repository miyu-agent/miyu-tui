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
    [[ -s $leftover ]] && print -z -- "$(<$leftover)"
    rm -f -- "$leftover"
    return 127
}

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 zsh 发 SIGUSR1。停在提示符上的（zle 开着）当场印：
# 信箱里有要印的才 `zle -I`（提示符画到印的下面），程序往上擦掉旧的提示符在那里印；前台在跑命令、她正在答的，zsh 等它
# 结束才办这个信号，那时 zle 没开，记着，回到提示符（画提示符以前）再印（实测 zsh 5.9）。
typeset -g __miyu_report_pending=
TRAPUSR1() {
    if zle; then
        __miyu_shellhook report --shell zsh --shell-pid $$ --pending || return 0
        zle -I
        # 提示符占几行：先拆成数组再数（只有一行时 `${#${(f)…}}` 数的是字数，往上擦多了）。
        local -a lines=("${(@f)${(%%)PS1}}")
        local rows=${#lines}
        __miyu_shellhook report --shell zsh --shell-pid $$ --above $rows
    else
        __miyu_report_pending=1
    fi
    return 0
}

__miyu_precmd() {
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        __miyu_shellhook report --shell zsh --shell-pid $$
    fi
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd __miyu_precmd
