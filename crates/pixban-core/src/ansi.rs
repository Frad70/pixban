use crate::color::{Rgb, Rgba};
use crate::render::Grid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
    Ansi16,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocks {
    Half,
    Full,
    Ascii,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnsiCaps {
    pub color: ColorMode,
    pub blocks: Blocks,
}

impl AnsiCaps {
    /// Terminal columns one logo cell takes: full and ascii use two so that cells stay square.
    pub fn columns_per_cell(self) -> u32 {
        match self.blocks {
            Blocks::Half => 1,
            Blocks::Full | Blocks::Ascii => 2,
        }
    }
}

const BASIC16: [(i32, i32, i32); 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];
const CUBE: [i32; 6] = [0, 95, 135, 175, 215, 255];

fn dist(a: (i32, i32, i32), c: Rgb) -> i32 {
    let (dr, dg, db) = (a.0 - c.0 as i32, a.1 - c.1 as i32, a.2 - c.2 as i32);
    dr * dr + dg * dg + db * db
}

fn nearest16(c: Rgb) -> usize {
    (0..16).min_by_key(|&i| dist(BASIC16[i], c)).unwrap_or(0)
}

fn nearest256(c: Rgb) -> u8 {
    let level = |v: u8| {
        (0..6)
            .min_by_key(|&i| (CUBE[i] - v as i32).abs())
            .unwrap_or(0)
    };
    let (r, g, b) = (level(c.0), level(c.1), level(c.2));
    let cube = (CUBE[r], CUBE[g], CUBE[b]);
    let gi = (0..24)
        .min_by_key(|&i| dist((8 + 10 * i, 8 + 10 * i, 8 + 10 * i), c))
        .unwrap_or(0);
    let gray = 8 + 10 * gi;
    if dist((gray, gray, gray), c) < dist(cube, c) {
        232 + gi as u8
    } else {
        16 + (36 * r + 6 * g + b) as u8
    }
}

fn sgr(mode: ColorMode, c: Rgb, bg: bool) -> String {
    match mode {
        ColorMode::TrueColor => format!(
            "\x1b[{};2;{};{};{}m",
            if bg { 48 } else { 38 },
            c.0,
            c.1,
            c.2
        ),
        ColorMode::Ansi256 => format!("\x1b[{};5;{}m", if bg { 48 } else { 38 }, nearest256(c)),
        _ => {
            let i = nearest16(c);
            let base = match (i < 8, bg) {
                (true, false) => 30,
                (true, true) => 40,
                (false, false) => 90,
                (false, true) => 100,
            };
            format!("\x1b[{}m", base + i % 8)
        }
    }
}

type Px = Option<Rgb>;

fn px(c: Rgba) -> Px {
    (!c.is_transparent()).then(|| c.rgb())
}

/// Draws the grid as terminal text. Every colored line ends with a reset; transparent cells are
/// plain spaces.
pub fn render_ansi(grid: &Grid, caps: AnsiCaps) -> String {
    let half = caps.blocks == Blocks::Half;
    let colored = caps.color != ColorMode::None;
    let mut out = String::new();
    let mut y = 0;
    while y < grid.height() {
        let mut cur: (Px, Px) = (None, None);
        for x in 0..grid.width() {
            let at = |y| px(grid.get(x, y)).filter(|c| colored || Some(*c) != grid.background());
            let top = at(y);
            let bottom = if half && y + 1 < grid.height() {
                at(y + 1)
            } else {
                None
            };
            let (glyph, fg, bg) = match caps.blocks {
                Blocks::Half => match (top, bottom) {
                    (None, None) => (" ", None, None),
                    (Some(t), None) => ("\u{2580}", Some(t), None),
                    (None, Some(b)) => ("\u{2584}", Some(b), None),
                    (Some(t), Some(b)) if t == b || !colored => ("\u{2588}", Some(t), None),
                    (Some(t), Some(b)) => ("\u{2580}", Some(t), Some(b)),
                },
                Blocks::Full => {
                    top.map_or(("  ", None, None), |c| ("\u{2588}\u{2588}", Some(c), None))
                }
                Blocks::Ascii => top.map_or(("  ", None, None), |c| ("##", Some(c), None)),
            };
            if colored {
                if (cur.0.is_some() && fg.is_none()) || (cur.1.is_some() && bg.is_none()) {
                    out.push_str("\x1b[0m");
                    cur = (None, None);
                }
                if fg != cur.0 {
                    out.push_str(&sgr(caps.color, fg.unwrap_or(Rgb::BLACK), false));
                }
                if bg != cur.1 {
                    out.push_str(&sgr(caps.color, bg.unwrap_or(Rgb::BLACK), true));
                }
                cur = (fg, bg);
            }
            out.push_str(glyph);
        }
        if colored {
            out.push_str("\x1b[0m");
        }
        out.push('\n');
        y += if half { 2 } else { 1 };
    }
    out
}
