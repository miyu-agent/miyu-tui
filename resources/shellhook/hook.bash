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
    return 127
}

# 后台回报（设计 22 第七节「后台任务和回报」）：盯后台的进程给这个 bash 发 SIGUSR1。bash 停在提示符上等你敲的时候不办
# 信号，要等你回车（实测 bash 5.3），所以只记着，回到提示符（画提示符以前）再印。
__miyu_report_pending=
trap '__miyu_report_pending=1' USR1

__miyu_prompt() {
    local status=$?
    if [[ -n $__miyu_report_pending ]]; then
        __miyu_report_pending=
        __miyu_shellhook report --shell bash --shell-pid $$
    fi
    return $status
}

case ";${PROMPT_COMMAND-};" in
    *";__miyu_prompt;"*) ;;
    *) PROMPT_COMMAND="__miyu_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
esac
