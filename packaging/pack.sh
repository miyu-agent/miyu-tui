#!/bin/sh
# 把终端打成软件源用的包文件（`.miyu`，核心施工 G-1，设计 37 第二节）：终端 `tui`（带程序，一种系统一个文件）。默认主题、
# 纯 ASCII 主题都编进了程序，不另打（终端蓝图「主题」第 4、6 条）。打包交给核心的 `miyu pkg pack`，这里只摆文件夹。
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

# 包的版本跟程序走（核心 VR-1，2026-11-06：本体和 Cargo.toml 同号，「年.月.这个月第几版」）。
version_of() { sed -n '/^\[package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$1" | head -n 1; }
want=$(version_of "$repo/Cargo.toml")
for manifest in tui/package.toml; do
  got=$(version_of "$repo/$manifest")
  [ "$got" = "$want" ] || die "$manifest 的版本 $got 和程序的 $want 不一样"
done

# 清单里的 targets 要写了这一种，核心才打（写了 targets 的必须挑一种）。
grep -q "\"$target\"" "$repo/tui/package.toml" || die "tui/package.toml 的 targets 没写 $target"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/tui/bin" "$out"
cp "$repo/tui/package.toml" "$stage/tui/"
cp "$bin" "$stage/tui/bin/$exe"
chmod 755 "$stage/tui/bin/$exe"

"$miyu" pkg pack "$stage/tui" --target "$target" -o "$out"
