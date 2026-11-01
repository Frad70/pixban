use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::font::{CaseMode, Font, Glyph};
use crate::limits::{MAX_GRID_SIDE, MAX_LINES, MAX_LINE_GAP, MAX_TEXT_CHARS, MIN_SPACING};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Text flow settings; build with `Default` and set the fields you need.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LayoutOptions {
    pub align: Align,
    /// Empty rows between lines.
    pub line_gap: u32,
    /// Replaces the font's own letter spacing when set.
    pub spacing: Option<i32>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            align: Align::Left,
            line_gap: 1,
            spacing: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlacedGlyph<'f> {
    pub ch: char,
    /// Column of the glyph's first column, measured from the left of the layout.
    pub x: u32,
    /// Row of the glyph's first row, measured from the top of the layout.
    pub y: u32,
    pub glyph: &'f Glyph,
    pub is_fallback: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutLine<'f> {
    /// Column where the line starts, after alignment.
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub glyphs: Vec<PlacedGlyph<'f>>,
}

#[derive(Clone, Debug)]
pub struct Layout<'f> {
    pub width: u32,
    /// Rows used by the glyphs in this text: the band plus the extra rows that are in use.
    pub height: u32,
    pub lines: Vec<LayoutLine<'f>>,
    pub missing: Vec<char>,
}

struct Row<'f> {
    width: u32,
    height: u32,
    glyphs: Vec<PlacedGlyph<'f>>,
}

fn set_row<'f>(
    font: &'f Font,
    chars: &[char],
    spacing: i64,
    missing: &mut Vec<char>,
) -> Result<Row<'f>, Error> {
    let mut placed: Vec<(i64, char, &Glyph, bool)> = Vec::with_capacity(chars.len());
    let mut pen: i64 = 0;
    let mut prev: Option<char> = None;
    for &c in chars {
        let (glyph, is_fallback) = match font.glyph(c) {
            Some(g) => (g, false),
            None => {
                if !missing.contains(&c) {
                    missing.push(c);
                }
                (font.fallback(), true)
            }
        };
        if let Some(p) = prev {
            pen += i64::from(font.kerning(p, c));
        }
        placed.push((pen + i64::from(glyph.left()), c, glyph, is_fallback));
        pen += i64::from(glyph.advance()) + spacing;
        prev = Some(c);
    }

    if placed.is_empty() {
        return Ok(Row {
            width: 0,
            height: font.height(),
            glyphs: Vec::new(),
        });
    }
    let min_x = placed.iter().map(|p| p.0).min().unwrap_or(0);
    let max_x = placed
        .iter()
        .map(|p| p.0 + i64::from(p.2.width()))
        .max()
        .unwrap_or(0);
    let width = (max_x - min_x) as u64;

    let top = placed.iter().map(|p| p.2.top()).max().unwrap_or(0);
    let bottom = placed.iter().map(|p| p.2.bottom()).max().unwrap_or(0);
    let height = top + font.height() + bottom;

    if width > u64::from(MAX_GRID_SIDE) {
        return Err(Error::CanvasTooLarge {
            width,
            height: u64::from(height),
        });
    }

    let glyphs = placed
        .into_iter()
        .map(|(x, ch, glyph, is_fallback)| PlacedGlyph {
            ch,
            x: (x - min_x) as u32,
            y: top - glyph.top(),
            glyph,
            is_fallback,
        })
        .collect();
    Ok(Row {
        width: width as u32,
        height,
        glyphs,
    })
}

pub fn layout<'f>(
    font: &'f Font,
    text: &str,
    options: &LayoutOptions,
) -> Result<Layout<'f>, Error> {
    let count = text.chars().count();
    if count > MAX_TEXT_CHARS {
        return Err(Error::TextTooLong {
            chars: count,
            max: MAX_TEXT_CHARS,
        });
    }
    if options.line_gap > MAX_LINE_GAP {
        return Err(Error::out_of_range(
            "line gap",
            i64::from(options.line_gap),
            0,
            i64::from(MAX_LINE_GAP),
        ));
    }
    let spacing = match options.spacing {
        Some(s) if !(MIN_SPACING..=crate::limits::MAX_SPACING as i32).contains(&s) => {
            return Err(Error::out_of_range(
                "letter spacing",
                i64::from(s),
                i64::from(MIN_SPACING),
                i64::from(crate::limits::MAX_SPACING),
            ));
        }
        Some(s) => i64::from(s),
        None => i64::from(font.spacing()),
    };

    let normalized = text.replace("\r\n", "\n");
    let trimmed = normalized.trim_end_matches('\n');
    if trimmed.is_empty() {
        return Err(Error::EmptyText);
    }
    let raw_lines: Vec<&str> = trimmed.split('\n').collect();
    if raw_lines.len() > MAX_LINES {
        return Err(Error::TooManyLines {
            lines: raw_lines.len(),
            max: MAX_LINES,
        });
    }

    let mut missing: Vec<char> = Vec::new();
    let mut rows: Vec<Row<'f>> = Vec::with_capacity(raw_lines.len());
    for line in &raw_lines {
        let chars: Vec<char> = match font.case() {
            CaseMode::Upper => line.chars().flat_map(char::to_uppercase).collect(),
            CaseMode::Mixed => line.chars().collect(),
        };
        rows.push(set_row(font, &chars, spacing, &mut missing)?);
    }

    let width = rows.iter().map(|r| r.width).max().unwrap_or(0);
    let gaps = u64::from(options.line_gap) * (rows.len() as u64 - 1);
    let height = rows.iter().map(|r| u64::from(r.height)).sum::<u64>() + gaps;
    if height > u64::from(MAX_GRID_SIDE) {
        return Err(Error::CanvasTooLarge {
            width: u64::from(width),
            height,
        });
    }

    let mut y = 0u32;
    let mut lines = Vec::with_capacity(rows.len());
    for row in rows {
        let x = match options.align {
            Align::Left => 0,
            Align::Center => (width - row.width) / 2,
            Align::Right => width - row.width,
        };
        let glyphs = row
            .glyphs
            .into_iter()
            .map(|mut g| {
                g.x += x;
                g.y += y;
                g
            })
            .collect();
        lines.push(LayoutLine {
            x,
            y,
            width: row.width,
            height: row.height,
            glyphs,
        });
        y += row.height + options.line_gap;
    }

    Ok(Layout {
        width,
        height: height as u32,
        lines,
        missing,
    })
}
