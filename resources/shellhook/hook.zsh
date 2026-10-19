# miyu shellhook 的 zsh 脚本：每次 zsh 起来时由 `miyu shellhook script --shell zsh` 现印。zsh 第一版只做找不到命令时
# 把这一行交给她（设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

__miyu_shellhook() {
    @PROGRAM@ shellhook "$@"
}

command_not_found_handler() {
    if [[ ! -o interactive ]]; then
        print -u2 "zsh: command not found: $1"
        return 127
    fi
    printf '%s' "$*" | __miyu_shellhook say --shell zsh
    return 127
}
