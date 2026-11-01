use std::collections::HashMap;

use crate::import::{RawFont, RawGlyph};

const ACCELERATORS: u32 = 1 << 1;
const METRICS: u32 = 1 << 2;
const BITMAPS: u32 = 1 << 3;
const BDF_ENCODINGS: u32 = 1 << 5;
const BDF_ACCELERATORS: u32 = 1 << 8;

const FORMAT_BYTE_MSB: u32 = 1 << 2;
const FORMAT_BIT_MSB: u32 = 1 << 3;
const FORMAT_COMPRESSED_METRICS: u32 = 1 << 8;

fn at(body: &[u8], pos: usize, n: usize) -> Result<&[u8], String> {
    body.get(pos..pos + n)
        .ok_or_else(|| "table is shorter than its contents".to_string())
}

fn read_u32(body: &[u8], pos: usize) -> Result<u32, String> {
    let b = at(body, pos, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_i32(body: &[u8], pos: usize, msb: bool) -> Result<i32, String> {
    let b = at(body, pos, 4)?;
    let a = [b[0], b[1], b[2], b[3]];
    Ok(if msb {
        i32::from_be_bytes(a)
    } else {
        i32::from_le_bytes(a)
    })
}

fn read_i16(body: &[u8], pos: usize, msb: bool) -> Result<i16, String> {
    let b = at(body, pos, 2)?;
    let a = [b[0], b[1]];
    Ok(if msb {
        i16::from_be_bytes(a)
    } else {
        i16::from_le_bytes(a)
    })
}

fn read_u16(body: &[u8], pos: usize, msb: bool) -> Result<u16, String> {
    let b = at(body, pos, 2)?;
    let a = [b[0], b[1]];
    Ok(if msb {
        u16::from_be_bytes(a)
    } else {
        u16::from_le_bytes(a)
    })
}

/// Bit `x` of a glyph row, honouring the byte order, scan unit and bit order of the format.
fn row_bit(row: &[u8], x: usize, unit: usize, byte_msb: bool, bit_msb: bool) -> bool {
    let b = x / 8;
    let phys = if unit > 1 && !byte_msb {
        (b / unit) * unit + (unit - 1 - b % unit)
    } else {
        b
    };
    let byte = row.get(phys).copied().unwrap_or(0);
    let shift = if bit_msb { 7 - x % 8 } else { x % 8 };
    byte >> shift & 1 == 1
}

pub(crate) fn parse(data: &[u8]) -> Result<RawFont, String> {
    let count = read_u32(data, 4)? as usize;
    if count > 64 {
        return Err(format!("table of contents lists {count} tables"));
    }
    let mut tables: HashMap<u32, &[u8]> = HashMap::new();
    for k in 0..count {
        let entry = 8 + 16 * k;
        let kind = read_u32(data, entry)?;
        let size = read_u32(data, entry + 8)? as usize;
        let offset = read_u32(data, entry + 12)? as usize;
        let body = offset
            .checked_add(size)
            .and_then(|end| data.get(offset..end))
            .ok_or_else(|| format!("table at offset {offset} is outside the file"))?;
        tables.insert(kind, body);
    }
    let table = |kind: u32| -> Result<&[u8], String> {
        tables
            .get(&kind)
            .copied()
            .ok_or_else(|| format!("missing table {kind:#X}"))
    };

    let acc = tables
        .get(&ACCELERATORS)
        .or_else(|| tables.get(&BDF_ACCELERATORS))
        .copied()
        .ok_or_else(|| "missing accelerator table".to_string())?;
    let acc_msb = read_u32(acc, 0)? & FORMAT_BYTE_MSB != 0;
    let ascent = read_i32(acc, 12, acc_msb)?;
    let descent = read_i32(acc, 16, acc_msb)?;

    let metrics = table(METRICS)?;
    let mfmt = read_u32(metrics, 0)?;
    let mmsb = mfmt & FORMAT_BYTE_MSB != 0;
    // (left bearing, right bearing, character width, ascent, descent) per glyph
    let mut infos: Vec<[i32; 5]> = Vec::new();
    if mfmt & FORMAT_COMPRESSED_METRICS != 0 {
        let n = read_u16(metrics, 4, mmsb)? as usize;
        for k in 0..n {
            let b = at(metrics, 6 + 5 * k, 5)?;
            infos.push([0, 1, 2, 3, 4].map(|j| i32::from(b[j]) - 0x80));
        }
    } else {
        let n = read_i32(metrics, 4, mmsb)?.max(0) as usize;
        for k in 0..n {
            let base = 8 + 12 * k;
            let mut info = [0i32; 5];
            for (slot, off) in info.iter_mut().zip([0usize, 2, 4, 6, 8]) {
                *slot = i32::from(read_i16(metrics, base + off, mmsb)?);
            }
            infos.push(info);
        }
    }

    let bitmaps = table(BITMAPS)?;
    let bfmt = read_u32(bitmaps, 0)?;
    let bmsb = bfmt & FORMAT_BYTE_MSB != 0;
    let bit_msb = bfmt & FORMAT_BIT_MSB != 0;
    let unit = 1usize << ((bfmt >> 4) & 3);
    let pad = 1usize << (bfmt & 3);
    let glyph_count = read_i32(bitmaps, 4, bmsb)?.max(0) as usize;
    if glyph_count != infos.len() {
        return Err(format!("{glyph_count} bitmaps but {} metrics", infos.len()));
    }
    let data_start = 8 + 4 * glyph_count + 16;

    let encodings = table(BDF_ENCODINGS)?;
    let emsb = read_u32(encodings, 0)? & FORMAT_BYTE_MSB != 0;
    let min_b2 = i32::from(read_i16(encodings, 4, emsb)?);
    let max_b2 = i32::from(read_i16(encodings, 6, emsb)?);
    let min_b1 = i32::from(read_i16(encodings, 8, emsb)?);
    let max_b1 = i32::from(read_i16(encodings, 10, emsb)?);
    let default_char = read_i16(encodings, 12, emsb)? as u16 as u32;
    if max_b2 < min_b2 || max_b1 < min_b1 {
        return Err("encoding range is empty or reversed".to_string());
    }
    let cols = (max_b2 - min_b2 + 1) as usize;

    let mut glyphs = Vec::new();
    for b1 in min_b1..=max_b1 {
        for b2 in min_b2..=max_b2 {
            let k = (b1 - min_b1) as usize * cols + (b2 - min_b2) as usize;
            let index = i32::from(read_i16(encodings, 14 + 2 * k, emsb)?);
            if index < 0 {
                continue;
            }
            let index = index as usize;
            let info = infos
                .get(index)
                .ok_or_else(|| format!("encoding points at missing glyph {index}"))?;
            let [lsb, rsb, advance, gascent, gdescent] = *info;
            if rsb < lsb || gascent < 0 || gdescent < 0 {
                return Err(format!("glyph {index} has inconsistent metrics"));
            }
            let width = (rsb - lsb) as u32;
            let height = (gascent + gdescent) as u32;
            let row_bytes = (width as usize).div_ceil(8).div_ceil(pad) * pad;
            let start = data_start + read_i32(bitmaps, 8 + 4 * index, bmsb)?.max(0) as usize;
            let mut bits = Vec::with_capacity((width * height) as usize);
            for r in 0..height as usize {
                let row = at(bitmaps, start + r * row_bytes, row_bytes)?;
                for x in 0..width as usize {
                    bits.push(row_bit(row, x, unit, bmsb, bit_msb));
                }
            }
            glyphs.push(RawGlyph {
                code: ((b1 as u32) << 8) | b2 as u32,
                advance,
                width,
                height,
                x: lsb,
                y: -gdescent,
                bits,
            });
        }
    }

    Ok(RawFont {
        ascent: Some(ascent),
        descent: Some(descent),
        bbox_width: None,
        bbox_height: None,
        bbox_y: None,
        default_char: Some(default_char),
        glyphs,
    })
}
