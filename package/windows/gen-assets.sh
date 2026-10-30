#!/usr/bin/env bash
# 由 package/windows/*.svg 生成安装包用的派生资源（.ico / .bmp）。
#
# 依赖：rsvg-convert（librsvg）、ImageMagick（magick）。
#   MSYS2：pacman -S mingw-w64-x86_64-librsvg mingw-w64-x86_64-imagemagick
#
# 产出的 ico/bmp 已入库，只有改安装包视觉素材时才需要重跑。
# 说明：
# - installer.ico / uninstall.ico：多尺寸（16/24/32/48/64/128/256）供 NSIS 使用；
#   24px 这一档是给老式任务栏/资源管理器的小图标用的，不要删。
# - install-header.bmp / install-sidebar.bmp：MUI2 的页眉(150×57)与侧栏(164×314)设计稿，
#   这里按 2× 光栅化；MUI2 默认 *_STRETCH=FitControl，会缩放到控件，高分屏下更清晰。
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

DIR=package/windows
SIZES="16 24 32 48 64 128 256"
WORK=build/gen-assets
tmp="$WORK/tmp"

rm -rf "$WORK"
mkdir -p "$tmp"

for name in installer uninstall; do
	inputs=()
	for s in $SIZES; do
		out="$tmp/$name-$s.png"
		rsvg-convert -w "$s" -h "$s" "$DIR/$name.svg" -o "$out"
		inputs+=("$out")
	done
	magick "${inputs[@]}" "$DIR/$name.ico"
done

rsvg-convert -w 300 -h 114 "$DIR/install-header.svg" -o "$tmp/header.png"
magick "$tmp/header.png" -type truecolor "BMP3:$DIR/install-header.bmp"

rsvg-convert -w 328 -h 628 "$DIR/install-sidebar.svg" -o "$tmp/sidebar.png"
magick "$tmp/sidebar.png" -type truecolor "BMP3:$DIR/install-sidebar.bmp"

rm -rf "$WORK"
echo "regenerated: $DIR/{installer,uninstall}.ico $DIR/install-{header,sidebar}.bmp"
