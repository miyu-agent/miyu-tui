#!/bin/sh
# 把终端打成软件源用的包文件（`.miyu`，核心施工 G-1，设计 37 第二节）：终端 `miyu-tui`（带程序，一种系统一个文件）和
# 两个主题包 `miyu-tui-theme-default`、`miyu-tui-theme-ascii`（只有数据，哪种系统都能装）。打包交给核心的 `miyu pkg pack`，这里只摆文件夹。
#
#   packaging/pack.sh --miyu <核心程序> --out <目录> [--target <系统>] [--bin <编好的 miyu-tui>]
#
# --target 默认照本机算（linux-x86_64、linux-aarch64、macos-aarch64）；windows-x86_64 要 --bin 给编好的 miyu-tui.exe。
# 不给 --bin 的先 `cargo build --release`。每个包印核心 `pack` 的三行：路径、SHA-256、大小。
set -eu

die() { echo "pack.sh: $*" >&2; exit 1; }

repo=$(cd "$(dirname "$0")/.." && pwd)
miyu='' out='' target='' bin=''
while [ $# -gt 0 ]; do
  case "$1" in
    --miyu) miyu=$2; shift 2;;
    --out) out=$2; shift 2;;
    --target) target=$2; shift 2;;
    --bin) bin=$2; shift 2;;
    -h|--help) sed -n '2,8p' "$0"; exit 0;;
    *) die "不认识的参数 $1";;
  esac
done
[ -n "$miyu" ] || die "要 --miyu：核心的程序（跑 \`pkg pack\` 用）"
[ -n "$out" ] || die "要 --out：包文件写到哪个目录"
[ -x "$miyu" ] || die "$miyu 不是能跑的程序"

if [ -z "$target" ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=linux-x86_64;;
    Linux-aarch64) target=linux-aarch64;;
    Darwin-arm64) target=macos-aarch64;;
    *) die "认不出本机是哪种系统，用 --target 写";;
  esac
fi
case "$target" in
  windows-*) exe='miyu-tui.exe';;
  *) exe='miyu-tui';;
esac

if [ -z "$bin" ]; then
  (cd "$repo" && cargo build --release)
  bin="$repo/target/release/$exe"
fi
[ -f "$bin" ] || die "$bin 不存在"

# 清单里的 targets 要写了这一种，核心才打（写了 targets 的必须挑一种）。
grep -q "\"$target\"" "$repo/miyu-tui/package.toml" || die "miyu-tui/package.toml 的 targets 没写 $target"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/miyu-tui/bin" "$out"
cp "$repo/miyu-tui/package.toml" "$stage/miyu-tui/"
cp "$bin" "$stage/miyu-tui/bin/$exe"
chmod 755 "$stage/miyu-tui/bin/$exe"
cp -R "$repo/packages/miyu-tui-theme-default" "$repo/packages/miyu-tui-theme-ascii" "$stage/"

"$miyu" pkg pack "$stage/miyu-tui" --target "$target" -o "$out"
for theme in miyu-tui-theme-default miyu-tui-theme-ascii; do
  "$miyu" pkg pack "$stage/$theme" -o "$out"
done
