use crate::color::Rgb;
use crate::error::Error;
use crate::font::Font;
use crate::layout::{layout, Layout, LayoutOptions};
use crate::limits::{MAX_OUTPUT_PIXELS, MAX_OUTPUT_SIDE, MAX_SCALE};
use crate::lint::Script;
use crate::palette::{self, stop_index};
use crate::render::{Grid, Rendered};

const BACKGROUND: Rgb = Rgb(0x0d, 0x11, 0x17);
const TEXT: Rgb = Rgb(0xe6, 0xed, 0xf3);
const TITLE: Rgb = Rgb(0x58, 0xa6, 0xff);
const SHEET_MAX_PX: u32 = 1600;
const MARGIN: u32 = 3;
const GAP: u32 = 2;
const EN_UPPER: &str = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG";
const EN_LOWER: &str = "the quick brown fox jumps over the lazy dog";
const RU_UPPER: &str = "СЪЕШЬ ЖЕ ЕЩЁ ЭТИХ МЯГКИХ ФРАНЦУЗСКИХ БУЛОК, ДА ВЫПЕЙ ЧАЮ";
const RU_LOWER: &str = "съешь же ещё этих мягких французских булок, да выпей чаю";

#[derive(Clone)]
enum Ink {
    Solid(Rgb),
    Gradient(&'static [Rgb]),
}

struct Block<'f> {
    layout: Layout<'f>,
    ink: Ink,
    gap_after: u32,
}

fn wrap(font: &Font, text: &str, max_cells: u32) -> Result<Vec<String>, Error> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ').filter(|w| !w.is_empty()) {
        if cur.is_empty() {
            cur = word.to_string();
            continue;
        }
        let candidate = format!("{cur} {word}");
        if layout(font, &candidate, &LayoutOptions::default())?.width <= max_cells {
            cur = candidate;
        } else {
            out.push(std::mem::replace(&mut cur, word.to_string()));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// A sheet with the font name, its character set by script, two pangrams and BRAZIER in fire.
pub fn specimen(font: &Font, scale: u32) -> Result<Rendered, Error> {
    if !(1..=MAX_SCALE).contains(&scale) {
        return Err(Error::InvalidScale {
            scale,
            max: MAX_SCALE,
        });
    }
    sheet(font, scale, (SHEET_MAX_PX / scale).max(40))
}

/// The same sheet at one pixel per cell, wrapped to at most `columns` cells wide where the
/// glyphs allow it.
pub fn specimen_fit(font: &Font, columns: u32) -> Result<Rendered, Error> {
    sheet(font, 1, columns.saturating_sub(2 * MARGIN).max(1))
}

fn sheet(font: &Font, scale: u32, max_cells: u32) -> Result<Rendered, Error> {
    let fire = palette::find("fire").expect("fire palette exists").stops;
    let mixed = font.case() == crate::font::CaseMode::Mixed;

    let mut sources: Vec<(String, Ink, u32)> =
        vec![(font.name().to_string(), Ink::Solid(TITLE), 2 * GAP)];
    for script in Script::ALL {
        let chars: Vec<String> = script
            .expected(font.case())
            .into_iter()
            .filter(|c| *c != ' ' && font.glyph(*c).is_some())
            .map(|c| c.to_string())
            .collect();
        if !chars.is_empty() {
            sources.push((chars.join(" "), Ink::Solid(TEXT), GAP));
        }
    }
    let last_set = sources.len() - 1;
    sources[last_set].2 = 2 * GAP;
    sources.push((EN_UPPER.to_string(), Ink::Solid(TEXT), GAP));
    if mixed {
        sources.push((EN_LOWER.to_string(), Ink::Solid(TEXT), GAP));
    }
    sources.push((RU_UPPER.to_string(), Ink::Solid(TEXT), GAP));
    if mixed {
        sources.push((RU_LOWER.to_string(), Ink::Solid(TEXT), GAP));
    }
    sources.push(("BRAZIER".to_string(), Ink::Gradient(fire), 0));

    let mut blocks: Vec<Block<'_>> = Vec::new();
    let mut missing: Vec<char> = Vec::new();
    for (text, ink, gap) in sources {
        let pieces = wrap(font, &text, max_cells)?;
        let last = pieces.len().saturating_sub(1);
        for (i, piece) in pieces.iter().enumerate() {
            let lay = layout(font, piece, &LayoutOptions::default())?;
            for c in &lay.missing {
                if !missing.contains(c) {
                    missing.push(*c);
                }
            }
            blocks.push(Block {
                layout: lay,
                ink: ink.clone(),
                gap_after: if i == last { gap } else { GAP },
            });
        }
    }

    let content_w = blocks.iter().map(|b| b.layout.width).max().unwrap_or(1);
    let content_h: u32 = blocks
        .iter()
        .map(|b| b.layout.height + b.gap_after)
        .sum::<u32>()
        .saturating_sub(blocks.last().map_or(0, |b| b.gap_after));
    let width = content_w + 2 * MARGIN;
    let height = content_h + 2 * MARGIN;
    let (out_w, out_h) = (
        u64::from(width) * u64::from(scale),
        u64::from(height) * u64::from(scale),
    );
    if out_w > MAX_OUTPUT_SIDE || out_h > MAX_OUTPUT_SIDE || out_w * out_h > MAX_OUTPUT_PIXELS {
        return Err(Error::OutputTooLarge {
            width: out_w,
            height: out_h,
        });
    }

    let mut grid = Grid::new(width, height, Some(BACKGROUND));
    let mut y0 = MARGIN;
    for block in &blocks {
        let lay = &block.layout;
        for p in &lay.lines[0].glyphs {
            for gy in 0..p.glyph.height() {
                for gx in 0..p.glyph.width() {
                    if !p.glyph.get(gx, gy) {
                        continue;
                    }
                    let row = p.y + gy;
                    let color = match block.ink {
                        Ink::Solid(c) => c,
                        Ink::Gradient(stops) => stops[stop_index(row, lay.height, stops.len())],
                    };
                    grid.set(MARGIN + p.x + gx, y0 + row, color);
                }
            }
        }
        y0 += lay.height + block.gap_after;
    }

    Ok(Rendered {
        grid,
        scale,
        title: format!("{} specimen", font.name()),
        missing,
    })
}
