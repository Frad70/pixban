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
