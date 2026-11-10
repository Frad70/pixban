#!/bin/sh
set -eu
cd "$(dirname "$0")"
P=${PIXBAN:-pixban}
TEXT="PIXEL\nПИКСЕЛЬ"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT

$P PIXBAN -f block -p fire --shadow 1,1 --scale 9 --padding 1 -o logo.svg
$P brazier -f block -p fire --shadow 1,1 -o brazier.svg
$P "Pixel\nBanner" --align center --outline 1 --extrude 4 -o banner.png
$P hi -p "#ff0066,#ffcc00/#330011" --plate --bevel 30 -o plate.png
$P wave -f tiny --anim wave --frames 12 --frame-ms 80 -o wave.gif

for f in block slim tiny mega; do
  $P "$TEXT" -f "$f" --align center -p fire -o "font-$f.svg"
done

$P PIXEL -f block -p fire --per-letter -o palette-fire.svg
$P PIXEL -f block -p ice --per-letter -o palette-ice.svg
$P PIXEL -f block -p ocean --per-letter -o palette-ocean.svg

$P PIXEL -f block -p gold --outline 2 --outline-color "#111111" --extrude 3 --extrude-color "#b03a2e" -o effect-outline-extrude.svg
$P PIXEL -f block -p synthwave --bevel 40 -o effect-bevel.svg
$P PIXEL -f block -p ice --texture noise --texture-strength 40 --seed 7 -o effect-texture.svg
$P PIXEL -f block -p neon --plate --cut-corners --plate-border 2 -o effect-plate.svg

$P PIXEL -f block -p fire --anim burn --flame 6 --frames 16 --frame-ms 60 --scale 6 -o burn.gif
$P PIXEL -f block --anim shimmer --band 4 --frames 16 --frame-ms 60 --scale 6 -o shimmer.gif
$P PIXEL -f block --anim glitch --count 4 --amplitude 3 --frames 16 --frame-ms 80 --scale 6 -o glitch.gif
$P PIXEL -f block --anim rainbow --frames 12 --frame-ms 80 --scale 6 -o rainbow.gif
$P PIXEL -f block --anim sparkle --count 8 --frames 12 --frame-ms 100 --scale 6 -o sparkle.gif
$P PIXEL -f block --anim type --frames 12 --frame-ms 120 --scale 6 -o type.gif

$P BURN -f block -p fire --anim burn --flame 6 --frames 40 --frame-ms 100 --scale 6 -o "$T/burn.gif" --frames-dir "$T/burn" > /dev/null
$P RAINBOW -f tiny --anim rainbow --frames 40 --frame-ms 100 --scale 8 -o "$T/rainbow.gif" --frames-dir "$T/rainbow" > /dev/null
$P Shimmer -f slim --anim shimmer --band 4 --frames 40 --frame-ms 100 --scale 6 -o "$T/shimmer.gif" --frames-dir "$T/shimmer" > /dev/null

mkdir "$T/demo"
for i in $(seq 0 39); do
  n=$(printf 'frame-%03d.png' "$i")
  magick \( "$T/burn/$n" -background '#0d1117' -gravity center -extent 720x88 \) \
    \( "$T/rainbow/$n" -background '#0d1117' -gravity center -extent 720x60 \) \
    \( "$T/shimmer/$n" -background '#0d1117' -gravity center -extent 720x60 \) \
    -append +repage -background '#0d1117' -alpha remove -alpha off "$T/demo/$n"
done
magick -delay 10 -loop 0 "$T"/demo/frame-*.png -layers optimize -colors 64 demo.gif
