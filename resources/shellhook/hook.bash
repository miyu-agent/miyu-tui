# miyu shellhook 的 bash 脚本：每次 bash 起来时由 `miyu shellhook script --shell bash` 现印。bash 第一版只做找不到命令时
# 把这一行交给她（设计 22 第七节）。@PROGRAM@ 在印的时候换成装的时候记下的程序。

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
    printf '%s' "$*" | __miyu_shellhook say --shell bash
    return 127
}
