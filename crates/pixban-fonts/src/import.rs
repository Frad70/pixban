use std::fmt;
use std::io::Read;
use std::path::Path;

use pixban_core::font::{self, CaseMode, Cell, FontMeta, Glyph, DEFAULT_SCALE};
use pixban_core::limits::{MAX_FONT_HEIGHT, MAX_GLYPH_WIDTH};
use pixban_core::Font;

use crate::{bdf, pcf};

const MAX_FILE_BYTES: u64 = 16 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Io(String),
    Malformed {
        format: &'static str,
        reason: String,
    },
    TooTall {
        rows: u32,
        max: u32,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(msg) => write!(f, "{msg}"),
            LoadError::Malformed { format, reason } => {
                write!(f, "malformed {format} font: {reason}")
            }
            LoadError::TooTall { rows, max } => {
                write!(f, "font is {rows} rows tall, the limit is {max}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

#[derive(Clone, Debug, Default)]
pub(crate) struct RawGlyph {
    pub code: u32,
    pub advance: i32,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    /// Row-major, top row first, `width * height` cells.
    pub bits: Vec<bool>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RawFont {
    pub ascent: Option<i32>,
    pub descent: Option<i32>,
    /// Declared maximum width, from the bounding box of a BDF file.
    pub bbox_width: Option<u32>,
    pub bbox_height: Option<i32>,
    pub bbox_y: Option<i32>,
    pub default_char: Option<u32>,
    pub glyphs: Vec<RawGlyph>,
}

fn malformed(format: &'static str, reason: impl Into<String>) -> LoadError {
    LoadError::Malformed {
        format,
        reason: reason.into(),
    }
}

/// Turns a parsed bitmap font into a pixban font. Font ascent and descent give the
/// band; glyph bitmaps that reach past it become declared extra rows.
pub(crate) fn build(raw: RawFont, name: &str) -> Result<Font, LoadError> {
    let ascent = raw
        .ascent
        .or(raw.bbox_height.zip(raw.bbox_y).map(|(h, y)| y + h))
        .ok_or_else(|| malformed("BDF or PCF", "no ascent or bounding box"))?;
    let descent = raw
        .descent
        .or(raw.bbox_y.map(|y| -y))
        .ok_or_else(|| malformed("BDF or PCF", "no descent or bounding box"))?;
    if ascent < 0 || descent < 0 {
        return Err(malformed("BDF or PCF", "negative ascent or descent"));
    }
    let band = ascent + descent;
    if band == 0 {
        return Err(malformed("BDF or PCF", "font has no rows"));
    }

    let mut max_top = 0i64;
    let mut max_bottom = 0i64;
    for g in &raw.glyphs {
        max_top = max_top.max(i64::from(g.y) + i64::from(g.height) - i64::from(ascent));
        max_bottom = max_bottom.max(i64::from(-descent) - i64::from(g.y));
    }
    let rows = max_top + i64::from(band) + max_bottom;
    if rows > i64::from(MAX_FONT_HEIGHT) {
        return Err(LoadError::TooTall {
            rows: rows as u32,
            max: MAX_FONT_HEIGHT,
        });
    }

    let mut glyphs: Vec<(char, Glyph)> = Vec::with_capacity(raw.glyphs.len());
    let mut fallback = None;
    let default = raw.default_char;
    for g in &raw.glyphs {
        if g.width > MAX_GLYPH_WIDTH {
            return Err(malformed(
                "BDF or PCF",
                format!(
                    "glyph {:#X} is {} wide, the limit is {MAX_GLYPH_WIDTH}",
                    g.code, g.width
                ),
            ));
        }
        let top = (i64::from(g.y) + i64::from(g.height) - i64::from(ascent)).max(0) as u32;
        let bottom = (i64::from(-descent) - i64::from(g.y)).max(0) as u32;
        let total = top + band as u32 + bottom;
        let mut bits = vec![false; (g.width * total) as usize];
        for r in 0..total {
            let k = i64::from(ascent) - 1 + i64::from(top) - i64::from(r);
            let (y0, y1) = (i64::from(g.y), i64::from(g.y) + i64::from(g.height));
            if k < y0 || k >= y1 {
                continue;
            }
            let br = (y1 - 1 - k) as u32;
            for c in 0..g.width {
                bits[(r * g.width + c) as usize] = g.bits[(br * g.width + c) as usize];
            }
        }
        let cell = Cell {
            left: g.x,
            advance: g.advance.max(0) as u32,
            width: g.width,
            top,
            bottom,
        };
        let glyph = Glyph::new(band as u32, cell, bits)
            .ok_or_else(|| malformed("BDF or PCF", "glyph shape does not match its bitmap"))?;
        if Some(g.code) == default {
            fallback = Some(glyph.clone());
        }
        if let Some(c) = char::from_u32(g.code) {
            glyphs.push((c, glyph));
        }
    }

    let max_width = raw
        .bbox_width
        .unwrap_or_else(|| raw.glyphs.iter().map(|g| g.width).max().unwrap_or(1).max(1));
    let meta = FontMeta {
        name: name.to_string(),
        height: band as u32,
        baseline: ascent as u32,
        spacing: 0,
        case: CaseMode::Mixed,
        default_scale: DEFAULT_SCALE,
        max_width,
    };
    Font::new(meta, glyphs, fallback, Vec::new()).map_err(|m| malformed("BDF or PCF", m))
}

fn font_name(path: &Path) -> String {
    let mut stem = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    for ext in [".gz", ".pcf", ".bdf", ".pxf"] {
        if let Some(rest) = stem.strip_suffix(ext) {
            stem = rest.to_string();
        }
    }
    if stem.is_empty() {
        "imported".to_string()
    } else {
        stem
    }
}

/// Loads a font from bytes: gzip, PCF, BDF or `.pxf` text, detected by content.
pub fn load_bytes(data: &[u8], name: &str) -> Result<Font, LoadError> {
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        let limit = MAX_FILE_BYTES * 4;
        flate2::read::GzDecoder::new(data)
            .take(limit + 1)
            .read_to_end(&mut out)
            .map_err(|e| malformed("gzip", e.to_string()))?;
        if out.len() as u64 > limit {
            return Err(malformed("gzip", "decompressed font is too large"));
        }
        return load_bytes(&out, name);
    }
    if data.starts_with(&[1, b'f', b'c', b'p']) {
        return build(pcf::parse(data).map_err(|m| malformed("PCF", m))?, name);
    }
    let text = std::str::from_utf8(data).map_err(|_| malformed("text", "not UTF-8"))?;
    if text.trim_start().starts_with("STARTFONT") {
        return build(bdf::parse(text).map_err(|m| malformed("BDF", m))?, name);
    }
    font::parse(text).map_err(|e| malformed("pxf", e.to_string()))
}

/// Loads a font file: a `.pxf` source, BDF, PCF, or either of them gzipped.
pub fn load_file(path: &Path) -> Result<Font, LoadError> {
    let meta = std::fs::metadata(path)
        .map_err(|e| LoadError::Io(format!("cannot read {}: {e}", path.display())))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(LoadError::Io(format!(
            "{} is larger than {} MiB",
            path.display(),
            MAX_FILE_BYTES >> 20
        )));
    }
    let data = std::fs::read(path)
        .map_err(|e| LoadError::Io(format!("cannot read {}: {e}", path.display())))?;
    load_bytes(&data, &font_name(path))
}
