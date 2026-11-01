#![allow(dead_code)]

use pixban_core::{font, Font};

pub const SRC: &str = "\
name: Test
height: 7
baseline: 7
spacing: 2
case: upper
kern: AI -1
---
char: space
...
...
...
...
...
...
...

char: A
.####.
##..##
##..##
######
##..##
##..##
##..##

char: I
######
..##..
..##..
..##..
..##..
..##..
######

char: Д
.#####
.##.##
.##.##
.##.##
.##.##
######
##...#
";

pub fn test_font() -> Font {
    font::parse(SRC).unwrap()
}

pub fn font_of(src: &str) -> Font {
    font::parse(src).unwrap()
}

/// One-cell glyphs: `a` is 1x1, `b` is 2x1, `c` is 3x1.
pub fn dot_font() -> Font {
    font_of(
        "name: Dot\nheight: 1\nbaseline: 1\nspacing: 1\ncase: mixed\n---\n\
         char: space\n.\n\nchar: a\n#\n\nchar: b\n##\n\nchar: c\n###\n",
    )
}

/// A solid 3x3 block as `b`.
pub fn block_font() -> Font {
    font_of(
        "name: Block3\nheight: 3\nbaseline: 3\nspacing: 1\ncase: mixed\n---\n\
         char: b\n###\n###\n###\n",
    )
}

/// A 4x5 block as `b`.
pub fn tall_font() -> Font {
    font_of(
        "name: Tall\nheight: 5\nbaseline: 5\nspacing: 1\ncase: mixed\n---\n\
         char: b\n####\n####\n####\n####\n####\n",
    )
}

pub fn art(out: &pixban_core::Rendered, legend: &[(pixban_core::Rgb, char)]) -> Vec<String> {
    (0..out.grid.height())
        .map(|y| {
            (0..out.grid.width())
                .map(|x| {
                    let cell = out.grid.get(x, y);
                    if cell.is_transparent() {
                        '.'
                    } else {
                        legend
                            .iter()
                            .find(|(c, _)| *c == cell.rgb())
                            .map_or('?', |(_, ch)| *ch)
                    }
                })
                .collect()
        })
        .collect()
}
