#!/bin/sh
set -eu
cd "$(dirname "$0")"
P=${PIXBAN:-pixban}
TEXT="PIXEL\nПИКСЕЛЬ"

$P brazier -f block -p fire --shadow 1,1 -o logo.svg
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

$P PIXEL -f block -p fire --anim burn --flame 6 --frames 16 --frame-ms 60 --scale 4 -o burn.gif
$P PIXEL -f block --anim shimmer --band 4 --frames 16 --frame-ms 60 --scale 4 -o shimmer.gif
$P PIXEL -f block --anim glitch --count 4 --amplitude 3 --frames 16 --frame-ms 80 --scale 4 -o glitch.gif
$P PIXEL -f block --anim rainbow --frames 12 --frame-ms 80 --scale 4 -o rainbow.gif
$P PIXEL -f block --anim sparkle --count 8 --frames 12 --frame-ms 100 --scale 4 -o sparkle.gif
$P PIXEL -f block --anim type --frames 12 --frame-ms 120 --scale 4 -o type.gif
