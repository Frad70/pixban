//! GIF89a writer. At most 256 colors, one of which stands for transparent when any frame
//! has transparent cells. When the frames use no more colors than that, the palette is exact;
//! otherwise a median cut picks the palette (see `median_cut`).

use std::collections::HashMap;

use crate::anim::Animated;
use crate::color::{Rgb, Rgba};
use crate::error::Error;
use crate::render::Grid;

/// GIF counts time in hundredths of a second and browsers treat anything under 20 ms as
/// 100 ms, so times are rounded to 10 ms and kept at 20 ms or more.
pub(crate) fn delay_ms(frame_ms: u32) -> u32 {
    ((frame_ms + 5) / 10 * 10).max(20)
}

struct Palette {
    colors: Vec<Rgb>,
    transparent: Option<u8>,
    map: HashMap<Rgb, u8>,
}

fn collect_colors(frames: &[Grid]) -> (Vec<Rgb>, HashMap<Rgb, u64>, bool) {
    let mut order = Vec::new();
    let mut counts: HashMap<Rgb, u64> = HashMap::new();
    let mut has_alpha = false;
    for grid in frames {
        for y in 0..grid.height() {
            for cell in grid.row(y) {
                if cell.is_transparent() {
                    has_alpha = true;
                } else {
                    let c = cell.rgb();
                    let n = counts.entry(c).or_insert_with(|| {
                        order.push(c);
                        0
                    });
                    *n += 1;
                }
            }
        }
    }
    (order, counts, has_alpha)
}

fn build_palette(frames: &[Grid]) -> Palette {
    let (order, counts, has_alpha) = collect_colors(frames);
    let room = 256 - usize::from(has_alpha);
    let (colors, map) = if order.len() <= room {
        let map = order
            .iter()
            .enumerate()
            .map(|(i, c)| (*c, i as u8))
            .collect();
        (order, map)
    } else {
        let weighted: Vec<(Rgb, u64)> = order.iter().map(|c| (*c, counts[c])).collect();
        let colors = median_cut(&weighted, room);
        let map = order
            .iter()
            .map(|c| (*c, nearest(&colors, *c) as u8))
            .collect();
        (colors, map)
    };
    let transparent = has_alpha.then_some(colors.len() as u8);
    Palette {
        colors,
        transparent,
        map,
    }
}

fn nearest(palette: &[Rgb], c: Rgb) -> usize {
    let dist = |p: &Rgb| {
        let d = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        d(p.0, c.0) + d(p.1, c.1) + d(p.2, c.2)
    };
    palette
        .iter()
        .enumerate()
        .min_by_key(|(i, p)| (dist(p), *i))
        .map_or(0, |(i, _)| i)
}

/// Median cut: repeatedly split the box with the widest channel range at the weighted
/// median of that channel, then average every box. Ties break on lower channel and lower
/// box number, so the palette depends only on the colors and their counts.
fn median_cut(colors: &[(Rgb, u64)], target: usize) -> Vec<Rgb> {
    fn channel(c: &Rgb, ch: usize) -> u8 {
        [c.0, c.1, c.2][ch]
    }
    fn widest(items: &[(Rgb, u64)]) -> (usize, u32) {
        let mut best = (0, 0);
        for ch in 0..3 {
            let lo = items.iter().map(|(c, _)| channel(c, ch)).min().unwrap_or(0);
            let hi = items.iter().map(|(c, _)| channel(c, ch)).max().unwrap_or(0);
            let range = u32::from(hi - lo);
            if range > best.1 {
                best = (ch, range);
            }
        }
        best
    }
    let mut boxes: Vec<Vec<(Rgb, u64)>> = vec![colors.to_vec()];
    while boxes.len() < target {
        let pick = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .map(|(i, b)| (widest(b).1, i))
            .max_by_key(|&(range, i)| (range, std::cmp::Reverse(i)));
        let Some((_, at)) = pick else { break };
        let mut items = std::mem::take(&mut boxes[at]);
        let (ch, _) = widest(&items);
        items.sort_by_key(|(c, _)| (channel(c, ch), c.0, c.1, c.2));
        let total: u64 = items.iter().map(|(_, n)| n).sum();
        let mut seen = 0;
        let mut cut = items.len() - 1;
        for (k, (_, n)) in items.iter().enumerate() {
            seen += n;
            if seen * 2 >= total {
                cut = (k + 1).min(items.len() - 1);
                break;
            }
        }
        let tail = items.split_off(cut);
        boxes[at] = items;
        boxes.push(tail);
    }
    boxes
        .iter()
        .map(|b| {
            let total: u64 = b.iter().map(|(_, n)| n).sum::<u64>().max(1);
            let mean = |f: fn(&Rgb) -> u8| {
                ((b.iter().map(|(c, n)| u64::from(f(c)) * n).sum::<u64>() + total / 2) / total)
                    as u8
            };
            Rgb(mean(|c| c.0), mean(|c| c.1), mean(|c| c.2))
        })
        .collect()
}

struct Bits {
    out: Vec<u8>,
    acc: u32,
    len: u32,
}

impl Bits {
    fn put(&mut self, code: u32, width: u32) {
        self.acc |= code << self.len;
        self.len += width;
        while self.len >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.len -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.len > 0 {
            self.out.push(self.acc as u8);
        }
        self.out
    }
}

const TABLE: usize = 1 << 13;

fn lzw(min_size: u32, pixels: &[u8]) -> Vec<u8> {
    let clear = 1u32 << min_size;
    let end = clear + 1;
    let mut bits = Bits {
        out: Vec::new(),
        acc: 0,
        len: 0,
    };
    let mut keys = vec![u32::MAX; TABLE];
    let mut codes = vec![0u16; TABLE];
    let mut width = min_size + 1;
    let mut next = end + 1;
    bits.put(clear, width);
    let Some((&first, rest)) = pixels.split_first() else {
        bits.put(end, width);
        return bits.finish();
    };
    let mut prefix = u32::from(first);
    for &p in rest {
        let key = (prefix << 8) | u32::from(p);
        let mut slot = (key.wrapping_mul(0x9E37_79B1) >> 19) as usize & (TABLE - 1);
        let mut found = None;
        while keys[slot] != u32::MAX {
            if keys[slot] == key {
                found = Some(u32::from(codes[slot]));
                break;
            }
            slot = (slot + 1) & (TABLE - 1);
        }
        if let Some(code) = found {
            prefix = code;
            continue;
        }
        bits.put(prefix, width);
        if next < 4096 {
            keys[slot] = key;
            codes[slot] = next as u16;
            next += 1;
            if next > (1 << width) && width < 12 {
                width += 1;
            }
        } else {
            bits.put(clear, width);
            keys.fill(u32::MAX);
            width = min_size + 1;
            next = end + 1;
        }
        prefix = u32::from(p);
    }
    bits.put(prefix, width);
    bits.put(end, width);
    bits.finish()
}

fn put_u16(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&(v as u16).to_le_bytes());
}

/// Bounding box (x, y, w, h) in cells of the cells where `keep` is true.
fn bbox(grid: &Grid, keep: impl Fn(u32, u32) -> bool) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            if keep(x, y) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    (x0 != u32::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

pub(crate) fn to_gif(a: &Animated) -> Result<Vec<u8>, Error> {
    let frames = a.frames();
    let scale = a.scale();
    let pal = build_palette(frames);
    let entries = pal.colors.len() + usize::from(pal.transparent.is_some());
    let depth = (entries.max(2) as u32 - 1).ilog2() + 1;
    let (w, h) = a.pixel_size();

    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");
    put_u16(&mut out, w);
    put_u16(&mut out, h);
    out.push(0x80 | 0x70 | (depth as u8 - 1));
    out.extend_from_slice(&[0, 0]);
    for i in 0..1usize << depth {
        match pal.colors.get(i) {
            Some(c) => out.extend_from_slice(&[c.0, c.1, c.2]),
            None => out.extend_from_slice(&[0, 0, 0]),
        }
    }
    out.extend_from_slice(b"\x21\xff\x0bNETSCAPE2.0\x03\x01\x00\x00\x00");

    let index = |cell: Rgba| match pal.transparent {
        Some(t) if cell.is_transparent() => t,
        _ => pal.map[&cell.rgb()],
    };
    let dispose = if pal.transparent.is_some() { 2 } else { 1 };
    let delay = delay_ms(a.frame_ms()) / 10;
    let min_size = depth.max(2);
    for (k, grid) in frames.iter().enumerate() {
        let (x, y, cw, ch) = if pal.transparent.is_some() {
            bbox(grid, |x, y| !grid.get(x, y).is_transparent()).unwrap_or((0, 0, 1, 1))
        } else if k == 0 {
            (0, 0, grid.width(), grid.height())
        } else {
            let before = &frames[k - 1];
            bbox(grid, |x, y| grid.get(x, y) != before.get(x, y)).unwrap_or((0, 0, 1, 1))
        };
        let mut pixels = Vec::with_capacity((cw * ch * scale * scale) as usize);
        let mut line = Vec::with_capacity((cw * scale) as usize);
        for yy in y..y + ch {
            line.clear();
            for xx in x..x + cw {
                let v = index(grid.get(xx, yy));
                line.extend(std::iter::repeat_n(v, scale as usize));
            }
            for _ in 0..scale {
                pixels.extend_from_slice(&line);
            }
        }
        out.extend_from_slice(&[0x21, 0xf9, 0x04]);
        out.push((dispose << 2) | u8::from(pal.transparent.is_some()));
        put_u16(&mut out, delay);
        out.push(pal.transparent.unwrap_or(0));
        out.push(0);
        out.push(0x2c);
        put_u16(&mut out, x * scale);
        put_u16(&mut out, y * scale);
        put_u16(&mut out, cw * scale);
        put_u16(&mut out, ch * scale);
        out.push(0);
        out.push(min_size as u8);
        for chunk in lzw(min_size, &pixels).chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
    }
    out.push(0x3b);
    Ok(out)
}
