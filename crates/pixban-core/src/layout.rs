use crate::error::Error;
use crate::font::{CaseMode, Font, Glyph};
use crate::limits::{MAX_GRID_SIDE, MAX_TEXT_CHARS};

/// Reserved for line spacing and alignment; construct with `Default`.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct LayoutOptions {}

#[derive(Clone, Debug)]
pub struct PlacedGlyph<'f> {
    pub ch: char,
    pub x: u32,
    /// Row of the glyph's first row, measured from the top of the layout.
    pub y: u32,
    pub glyph: &'f Glyph,
    pub is_fallback: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutLine<'f> {
    pub y: u32,
    pub width: u32,
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

pub fn layout<'f>(
    font: &'f Font,
    text: &str,
    _options: &LayoutOptions,
) -> Result<Layout<'f>, Error> {
    let count = text.chars().count();
    if count > MAX_TEXT_CHARS {
        return Err(Error::TextTooLong {
            chars: count,
            max: MAX_TEXT_CHARS,
        });
    }
    if text.contains('\n') {
        return Err(Error::MultiLine);
    }
    let chars: Vec<char> = match font.case() {
        CaseMode::Upper => text.chars().flat_map(char::to_uppercase).collect(),
        CaseMode::Mixed => text.chars().collect(),
    };
    if chars.is_empty() {
        return Err(Error::EmptyText);
    }

    let mut missing: Vec<char> = Vec::new();
    let mut placed: Vec<(i64, char, &Glyph, bool)> = Vec::with_capacity(chars.len());
    let mut pen: i64 = 0;
    let mut prev: Option<char> = None;
    for c in chars {
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
        pen += i64::from(glyph.advance()) + i64::from(font.spacing());
        prev = Some(c);
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
    let width = width as u32;

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

    Ok(Layout {
        width,
        height,
        lines: vec![LayoutLine {
            y: 0,
            width,
            glyphs,
        }],
        missing,
    })
}
