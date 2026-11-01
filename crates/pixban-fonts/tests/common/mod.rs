#![allow(dead_code)]

use std::fmt::Write;

use pixban_core::{Font, Glyph};

const DEFAULT_CODE: u32 = 0xFFFD;

/// Glyphs to write, by code point, with the fallback added under the default code.
fn entries(font: &Font) -> Vec<(u32, &Glyph)> {
    let mut v: Vec<(u32, &Glyph)> = font
        .chars()
        .into_iter()
        .map(|c| (c as u32, font.glyph(c).expect("listed glyph exists")))
        .collect();
    if !v.iter().any(|(c, _)| *c == DEFAULT_CODE) {
        v.push((DEFAULT_CODE, font.fallback()));
    }
    v.sort_by_key(|(c, _)| *c);
    v
}

pub fn to_bdf(font: &Font) -> String {
    let band = font.height() as i32;
    let ascent = font.baseline() as i32;
    let descent = band - ascent;
    let items = entries(font);
    let mut out = String::new();
    writeln!(out, "STARTFONT 2.1").unwrap();
    writeln!(
        out,
        "FONT -pixban-{}-medium-r-normal--{band}-0-0-0-c-0-iso10646-1",
        font.name()
    )
    .unwrap();
    writeln!(out, "SIZE 10 72 72").unwrap();
    writeln!(
        out,
        "FONTBOUNDINGBOX {} {band} 0 {}",
        font.max_width(),
        -descent
    )
    .unwrap();
    writeln!(out, "STARTPROPERTIES 3").unwrap();
    writeln!(out, "FONT_ASCENT {ascent}").unwrap();
    writeln!(out, "FONT_DESCENT {descent}").unwrap();
    writeln!(out, "DEFAULT_CHAR {DEFAULT_CODE}").unwrap();
    writeln!(out, "ENDPROPERTIES").unwrap();
    writeln!(out, "CHARS {}", items.len()).unwrap();
    for (code, g) in &items {
        writeln!(out, "STARTCHAR U{code:04X}").unwrap();
        writeln!(out, "ENCODING {code}").unwrap();
        writeln!(out, "SWIDTH 0 0").unwrap();
        writeln!(out, "DWIDTH {} 0", g.advance() + font.spacing()).unwrap();
        let yoff = -descent - g.bottom() as i32;
        writeln!(out, "BBX {} {} {} {yoff}", g.width(), g.height(), g.left()).unwrap();
        writeln!(out, "BITMAP").unwrap();
        for y in 0..g.height() {
            let bytes = (g.width() as usize).div_ceil(8);
            for b in 0..bytes {
                let mut byte = 0u8;
                for bit in 0..8 {
                    let x = b * 8 + bit;
                    if x < g.width() as usize && g.get(x as u32, y) {
                        byte |= 0x80 >> bit;
                    }
                }
                write!(out, "{byte:02X}").unwrap();
            }
            writeln!(out).unwrap();
        }
        writeln!(out, "ENDCHAR").unwrap();
    }
    writeln!(out, "ENDFONT").unwrap();
    out
}

/// Minimal PCF writer: uncompressed metrics, 4-byte glyph padding, one byte scan unit.
/// `msb` selects most-significant-byte and most-significant-bit order for the tables.
pub fn to_pcf(font: &Font, msb: bool) -> Vec<u8> {
    let items = entries(font);
    let band = font.height() as i32;
    let base = font.baseline() as i32;
    let desc = band - base;
    let flags: u32 = if msb { 4 } else { 0 };
    let bit_flag: u32 = if msb { 8 } else { 0 };
    let le32 = |v: u32| v.to_le_bytes().to_vec();
    let bo32 = |v: i32| {
        if msb {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };
    let bo16 = |v: i16| {
        if msb {
            v.to_be_bytes()
        } else {
            v.to_le_bytes()
        }
    };

    let mut acc = Vec::new();
    acc.extend(le32(flags));
    acc.extend([0u8; 8]);
    acc.extend(bo32(base));
    acc.extend(bo32(desc));
    acc.extend(bo32(0));
    acc.extend([0u8; 24]);

    let mut met = Vec::new();
    met.extend(le32(flags));
    met.extend(bo32(items.len() as i32));
    for (_, g) in &items {
        let lsb = g.left();
        let rsb = g.left() + g.width() as i32;
        let width = (g.advance() + font.spacing()) as i32;
        let ascent = g.top() as i32 + base;
        let descent = desc + g.bottom() as i32;
        for v in [lsb, rsb, width, ascent, descent] {
            met.extend(bo16(v as i16));
        }
        met.extend(bo16(0));
    }

    let mut data = Vec::new();
    let mut offsets = Vec::new();
    for (_, g) in &items {
        offsets.push(data.len() as i32);
        let w = g.width() as usize;
        let stride = w.div_ceil(8).div_ceil(4) * 4;
        for y in 0..g.height() {
            let mut row = vec![0u8; stride];
            for x in 0..w {
                if g.get(x as u32, y) {
                    let bit = if msb { 7 - x % 8 } else { x % 8 };
                    row[x / 8] |= 1 << bit;
                }
            }
            data.extend(row);
        }
    }
    let bmp_format = 2 | flags | bit_flag;
    let mut bmp = Vec::new();
    bmp.extend(le32(bmp_format));
    bmp.extend(bo32(items.len() as i32));
    for o in &offsets {
        bmp.extend(bo32(*o));
    }
    for _ in 0..4 {
        bmp.extend(bo32(data.len() as i32));
    }
    bmp.extend(data);

    let max_b1 = items.iter().map(|(c, _)| c >> 8).max().unwrap_or(0);
    let mut enc = Vec::new();
    enc.extend(le32(flags));
    enc.extend(bo16(0));
    enc.extend(bo16(255));
    enc.extend(bo16(0));
    enc.extend(bo16(max_b1 as i16));
    enc.extend(bo16(DEFAULT_CODE as u16 as i16));
    let mut map = vec![-1i16; (max_b1 as usize + 1) * 256];
    for (idx, (code, _)) in items.iter().enumerate() {
        map[(code >> 8) as usize * 256 + (code & 0xFF) as usize] = idx as i16;
    }
    for v in map {
        enc.extend(bo16(v));
    }

    let tables: [(u32, u32, Vec<u8>); 4] = [
        (2, flags, acc),
        (4, flags, met),
        (8, bmp_format, bmp),
        (32, flags, enc),
    ];
    let mut out = vec![1, b'f', b'c', b'p'];
    out.extend(le32(tables.len() as u32));
    let mut offset = 8 + 16 * tables.len() as u32;
    for (kind, format, body) in &tables {
        out.extend(le32(*kind));
        out.extend(le32(*format));
        out.extend(le32(body.len() as u32));
        out.extend(le32(offset));
        offset += body.len() as u32;
    }
    for (_, _, body) in &tables {
        out.extend(body);
    }
    out
}

pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(bytes).unwrap();
    enc.finish().unwrap()
}
