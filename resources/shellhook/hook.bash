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

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 bash 发 SIGUSR1。bash 停在提示符上等你敲的时候不办
# 信号，等你回车、这一行读完、命令还没跑的时候才办；前台在跑命令（vim、她正在答）的，等它结束才办（实测 bash 5.3）。
# 那时都不在画提示符：直接印。所以停在提示符上来的回报，要等你下一次回车，印在那条命令前面。
__miyu_report() {
    __miyu_shellhook report --shell bash --shell-pid $$ && printf '\n'
    printf '\e[?25h'
}
trap '__miyu_report' USR1
