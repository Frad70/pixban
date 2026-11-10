# pixban

Pixel-art logos and banners from text.

pixban is a command line tool that renders text as pixel art. It writes SVG, PNG, GIF and APNG files and can animate the result.

## Install

Requires a Rust toolchain with Cargo.

From a clone of this repository:

```sh
cargo install --locked --path crates/pixban-cli
```

This installs the `pixban` binary into Cargo's bin directory.

To build without installing:

```sh
cargo build --release -p pixban-cli
./target/release/pixban --help
```

Run the test suite with `cargo test --workspace`. At this commit it runs 223 tests, all passing.

## Quick start

The images in this README, and the script that generates them (`examples/gen.sh`), live in the `attachments` branch. They are generated with `PIXBAN=<path to binary> sh examples/gen.sh`.

```sh
pixban brazier -f block -p fire --shadow 1,1 -o logo.svg
```

Writes an SVG with the word set in the block font, filled with the fire palette and a one-cell shadow. Without `-o`, the SVG goes to standard output, or the terminal drawing when standard output is a terminal (see Terminal output).

![BRAZIER in the block font with a fire gradient and a dark one-cell shadow](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/logo.svg)

```sh
pixban "Pixel\nBanner" --align center --outline 1 --extrude 4 -o banner.png
```

Writes a 416 by 168 pixel PNG with two centered lines. A literal `\n` in the text starts a new line. The block font has only capitals, so "Pixel" is drawn as PIXEL.

![PIXEL above BANNER, centered, with a dark outline and an extruded side](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/banner.png)

```sh
pixban hi -p "#ff0066,#ffcc00/#330011" --plate --bevel 30 -o plate.png
```

Writes a 160 by 104 pixel PNG. The palette has two colors, and the shadow color follows the slash. The text sits on a plate, and the fill has a bevel.

![HI in pink and yellow on a dark plate with a yellow border](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/plate.png)

```sh
pixban wave -f tiny --anim wave --frames 12 --frame-ms 80 -o wave.gif
```

Writes an animated GIF. `--frames` and `--frame-ms` set the loop length and the delay per frame.

![WAVE in the tiny font, letters bobbing up and down](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/wave.gif)

```sh
pixban brazier -f block -p fire --shadow 1,1 --dump-spec > logo.toml
pixban --spec logo.toml -o logo.svg
```

The first command saves the settings as TOML. The second reads them and writes the same SVG as the first example, byte for byte.

![The same BRAZIER logo, produced from the settings file](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/logo.svg)

## Fonts

`pixban fonts` lists the built-in fonts. The default font is `block`. Select a font with `-f NAME`. `--font-file FILE` loads a font from a file instead and replaces `-f`. It accepts .pxf, BDF and PCF files, optionally gzipped.

| Font  | Height (cells) | Glyphs | Letters |
|-------|----------------|--------|---------|
| block | 7              | 107    | Latin and Cyrillic capitals, digits |
| slim  | 7              | 166    | Latin and Cyrillic, both cases, digits |
| tiny  | 5              | 107    | Latin and Cyrillic capitals, digits |
| mega  | 10             | 107    | Latin and Cyrillic capitals, digits |

`pixban lint-font NAME` (or a font file path) prints the glyph coverage of a font.

Each font renders the same two lines.

```sh
pixban "PIXEL\nПИКСЕЛЬ" -f block --align center -p fire -o font-block.svg
```

![PIXEL above ПИКСЕЛЬ in the block font](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/font-block.svg)

```sh
pixban "PIXEL\nПИКСЕЛЬ" -f slim --align center -p fire -o font-slim.svg
```

![PIXEL above ПИКСЕЛЬ in the slim font](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/font-slim.svg)

```sh
pixban "PIXEL\nПИКСЕЛЬ" -f tiny --align center -p fire -o font-tiny.svg
```

![PIXEL above ПИКСЕЛЬ in the tiny font](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/font-tiny.svg)

```sh
pixban "PIXEL\nПИКСЕЛЬ" -f mega --align center -p fire -o font-mega.svg
```

![PIXEL above ПИКСЕЛЬ in the mega font](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/font-mega.svg)

## Palettes

`pixban palettes` lists the default palettes: fire, ice, toxic, neon, gold, synthwave, mono, rainbow. `pixban palettes --all` adds the extended group: pico8-warm, pico8-cool, gameboy, nes, c64, cga, lava, ocean, forest, sunset, pastel, vaporwave, ember.

`-p` takes a palette name, or a list of colors as `"#hex,#hex,..."` with an optional `/#shadow` at the end.

`--per-letter` puts one palette stop on each letter, spread over the whole text.

```sh
pixban PIXEL -f block -p fire --per-letter -o palette-fire.svg
```

![PIXEL in the fire palette, one color per letter](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/palette-fire.svg)

```sh
pixban PIXEL -f block -p ice --per-letter -o palette-ice.svg
```

![PIXEL in the ice palette, one color per letter](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/palette-ice.svg)

```sh
pixban PIXEL -f block -p ocean --per-letter -o palette-ocean.svg
```

![PIXEL in the ocean palette, one color per letter](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/palette-ocean.svg)

Other fill options:

- `--gradient vertical|horizontal|diagonal` sets the direction of the palette gradient.
- `--dither none|bayer2|bayer4` applies ordered dithering between neighboring palette stops.
- `--color #rrggbb` sets a solid fill.
- `--letter-colors "#hex,#hex,..."` cycles up to 64 colors over the letters.

## Effects

Effects can be combined. The shadow effect is shown in the quick start. Each group below has one example.

### Outline and extrusion

```sh
pixban PIXEL -f block -p gold --outline 2 --outline-color "#111111" --extrude 3 --extrude-color "#b03a2e" -o effect-outline-extrude.svg
```

![PIXEL in gold with a black outline and a red extruded side](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/effect-outline-extrude.svg)

- `--outline N` sets the width in cells, 1 to 4. `--outline-color` defaults to the palette's shadow color.
- `--outline-conn 4|8` sets the neighborhood: 4 gives a diamond, 8 a square (default 8).
- `--extrude N` sets the length in cells, 1 to 32. `--extrude-dir DX,DY` takes values from -1 to 1 (default 1,1).
- `--extrude-color` is the color the extrusion darkens toward. `--extrude-step PCT` sets how far each step moves toward it, 1 to 100 (default 30).
- `--shadow DX,DY` is a hard offset in cells, and `--shadow-color` sets its color.

### Bevel

```sh
pixban PIXEL -f block -p synthwave --bevel 40 -o effect-bevel.svg
```

![PIXEL in the synthwave palette with a beveled fill](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/effect-bevel.svg)

`--bevel PCT`, 1 to 100, lightens the top-left edge and darkens the bottom-right edge.

### Texture

```sh
pixban PIXEL -f block -p ice --texture noise --texture-strength 40 --seed 7 -o effect-texture.svg
```

![PIXEL in the ice palette with a noise texture](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/effect-texture.svg)

- `--texture scanlines|checker|noise` sets the pattern.
- `--texture-strength PCT`, 1 to 100 (default 25).
- `--texture-period N` for scanlines: every Nth row is darker, 2 to 16 (default 2).
- `--seed N` fixes the noise. The same seed always gives the same file.

### Plate

```sh
pixban PIXEL -f block -p neon --plate --cut-corners --plate-border 2 -o effect-plate.svg
```

![PIXEL in neon colors on a dark plate with cut corners](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/effect-plate.svg)

- `--plate` puts the text on a pixel plate. `--plate-color` defaults to #0d1117.
- `--plate-border N`, 0 to 8 cells (default 1). `--plate-border-color` defaults to the middle color of the fill.
- `--cut-corners` removes the four corner cells of the plate.

### Padding, background and layout

- `--padding N` sets empty cells around the artwork, 0 to 64 (2 with a plate).
- `--bg #rrggbb` sets a solid background. The default is transparent.
- `--align left|center|right` sets the alignment of lines.
- `--line-gap N` sets empty rows between lines, 0 to 32 (default 1).
- `--spacing N` sets letter spacing in cells, -2 to 32, and replaces the font's own.
- `--scale N` sets pixels per cell.

## Animations

`--anim KIND` turns the output into an animation. The kinds are burn, shimmer, wave, type, glitch, rainbow and sparkle. The wave animation is in the quick start. The output is an animated SVG, GIF or APNG, depending on the file extension.

The examples below use the block font at `--scale 4` to keep the files small.

```sh
pixban PIXEL -f block -p fire --anim burn --flame 6 --frames 16 --frame-ms 60 --scale 4 -o burn.gif
```

![PIXEL in the fire palette with flames rising from the letters](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/burn.gif)

```sh
pixban PIXEL -f block --anim shimmer --band 4 --frames 16 --frame-ms 60 --scale 4 -o shimmer.gif
```

![PIXEL with a light band moving across the letters](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/shimmer.gif)

```sh
pixban PIXEL -f block --anim glitch --count 4 --amplitude 3 --frames 16 --frame-ms 80 --scale 4 -o glitch.gif
```

![PIXEL with horizontal slices shifted sideways in short bursts](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/glitch.gif)

```sh
pixban PIXEL -f block --anim rainbow --frames 12 --frame-ms 80 --scale 4 -o rainbow.gif
```

![PIXEL with the fill color cycling through hues](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/rainbow.gif)

```sh
pixban PIXEL -f block --anim sparkle --count 8 --frames 12 --frame-ms 100 --scale 4 -o sparkle.gif
```

![PIXEL with small bright points twinkling on the letters](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/sparkle.gif)

```sh
pixban PIXEL -f block --anim type --frames 12 --frame-ms 120 --scale 4 -o type.gif
```

![PIXEL appearing letter by letter with a cursor](https://raw.githubusercontent.com/Frad70/pixban/attachments/examples/type.gif)

Animation flags:

- `--frames N`: frames in one loop, 2 to 120. The default depends on the animation.
- `--frame-ms MS`: milliseconds per frame, 20 to 2000. The default depends on the animation.
- `--flame N` (burn): flame height in cells, 1 to 32.
- `--amplitude N` (wave): how far letters bob. (glitch): the longest slice shift. Both in cells, 1 to 16.
- `--band N` (shimmer): width of the light band in cells, 1 to 32.
- `--count N` (glitch): bursts per loop. (sparkle): twinkles per loop. Both 1 to 64.
- `--hues N` (rainbow): distinct hues in one turn, 3 to 36 (default 12).
- `--frames-dir DIR`: also writes every frame as a numbered PNG, such as `frame-000.png`.

## Export formats

The output format follows the extension of `-o`: `.svg`, `.png`, `.gif` or `.apng`. `--format svg|png|gif|apng` overrides the extension and also works on standard output. Without `-o`, the output is SVG on standard output, or the terminal drawing when standard output is a terminal (see Terminal output).

## Terminal output

When standard output is a terminal and there is no `-o`, pixban prints the logo as text. The default `half` mode draws it with half-block characters (`▀`, `▄`, `█`): each logo cell is one column, and each text line holds two rows of cells. `--blocks full` draws with full blocks, and `--blocks ascii` draws with `#` characters. Both take two columns per cell and one row per line.

```sh
pixban hi -f tiny --blocks ascii --color-mode none
```

Output, exactly as printed:

```
##  ##  ######
##  ##    ##  
######    ##  
##  ##    ##  
##  ##  ######
```

`--color-mode` accepts `auto` (the default), `truecolor`, `256`, `16` and `none`. Any value other than `auto` overrides the environment. In `auto` mode pixban reads the environment in this order:

1. `NO_COLOR` is set: no color.
2. `COLORTERM` is `truecolor` or `24bit`: truecolor.
3. `TERM` names a 256-color terminal, such as `xterm-256color`: 256 colors.
4. `TERM` is `xterm` or `linux`: 16 colors.
5. `TERM` is unset, empty or `dumb`: no color.

In truecolor mode each character has its own 24-bit foreground and background colors. In 256 mode the colors are 256-color palette entries. In 16 mode they are the 16 ANSI colors. In none mode the characters have no color.

The drawing must fit in the terminal width. pixban reads the width from the `COLUMNS` variable when it is set, and from the terminal otherwise. A drawing wider than that is not printed. pixban exits with status 1 and writes an error to standard error, for example on a terminal 40 columns wide:

```
error: the drawing is 54 columns wide but the terminal has 40; use shorter text, a smaller font or a wider terminal
```

When standard output is not a terminal and `COLUMNS` is not set, the drawing is not checked against a width.

Examples at 80 columns, in half mode unless noted:

- `pixban brazier -f block` is 54 columns and fits. With `--blocks full` or `--blocks ascii` it is 108 columns and is refused.
- `pixban "Hello world" -f block` is 85 columns and is refused. With `-f tiny` it fits.
- `pixban "PIXEL ПИКСЕЛЬ" -f block` is 101 columns and is refused.
- `pixban specimen -f block` is 95 columns and is refused. With `-f tiny` it is 80 columns and fits.

With `--anim` on a terminal, the animation plays in place. Each frame is drawn over the previous one, and the cursor is hidden until playback ends. `--loops N` sets how many times the animation plays, 1 by default. `--loops 0` plays until Ctrl-C. Ctrl-C ends playback with exit status 0, and the cursor is shown again. This was tested with SIGINT sent to the process.

When standard output is not a terminal, an animation with `--ansi` is written as one frame, without cursor control, and `--loops` has no effect.

Previews:

- `pixban fonts --preview` draws each font name in its own font.
- `pixban palettes --preview` draws a color strip next to each name. With `--color-mode none` the strips are plain block characters:

```
█████████████████████ ███ fire
█████████████████████ ███ ice
█████████████████████ ███ toxic
█████████████████████ ███ neon
█████████████████████ ███ gold
█████████████████████ ███ synthwave
█████████████████████ ███ mono
█████████████████████ ███ rainbow
```

- `pixban palettes --all --preview` also shows the extended palettes. The widest line is 35 columns.
- `pixban specimen -f FONT` with no `-o` draws the font's glyphs in the terminal. With `-o FILE.png` it writes the PNG as before.

Snippet:

`pixban snippet ARGS` prints a short program that draws the same logo. `ARGS` are the usual text and flags. `--for shell` is the default. `--for python` and `--for rust` select the other languages. The first line is a comment with the command that generated it.

```sh
pixban snippet hi -f tiny --blocks ascii --color-mode none
```

Output, exactly as printed:

```
# regenerate: pixban hi -f tiny --blocks ascii --color-mode none --ansi
printf '%s' $'##  ##  ######\n##  ##    ##  \n######    ##  \n##  ##    ##  \n##  ##  ######\n'
```

Limits of the snippet:

- It always adds `--ansi`. Without `--color-mode` it writes truecolor escapes whatever the environment says, including `NO_COLOR`.
- For an animation, the snippet contains one frame.

Limit of `-o` with `--ansi`: the text drawing is written into the file whatever its name, so `-o logo.svg --ansi` produces text, not SVG.

## Settings file

`--dump-spec` prints the settings for the given command as TOML and exits. For a multi-line text:

```toml
text = """
Pixel
Banner"""
font = "block"
align = "center"

[fill]
direction = "vertical"

[outline]
width = 1
connectivity = "8"

[extrude]
length = 4
dx = 1
dy = 1
step = 30
```

`--spec FILE` loads settings from a TOML file. Flags on the command line override values from the file, including the text. For example, `pixban --spec banner.toml -f tiny` renders the file's text in the tiny font.

Animation settings use an `[anim]` section:

```toml
[anim]
kind = "wave"
frames = 12
frame_ms = 80
amplitude = 2
```

## License

The code is licensed under the MIT License (see `LICENSE`). Copyright (c) 2026 Frad70.

The bundled fonts in `crates/pixban-fonts/fonts` are licensed under the SIL Open Font License, Version 1.1 (see `crates/pixban-fonts/fonts/OFL.txt`). Copyright 2026 Frad70.
