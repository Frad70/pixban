//! Flames for the burn animation. The look follows the PlayStation Doom fire that brazier
//! uses: heat falls with the height above a surface, tongues of different height stand side
//! by side, and heat is cut into a few palette steps. The simulation there is chaotic and
//! never repeats, so heat here is a pure function of position and frame instead: noise whose
//! time axis is wrapped to the loop, plus a texture that scrolls up by exactly one of its
//! own periods per loop. All arithmetic is integer, so every platform gives the same cells.

use crate::color::Rgb;
use crate::effects::Mask;

pub(crate) const LEVELS: usize = 7;
const HEAT_MIN: i32 = 110;
const HEAT_MAX: i32 = 1000;

/// The tongue noise has one lattice point every this many half cells.
const TONGUE_HALF_CELLS: i32 = 3;
/// Cells of the scrolling texture along the flame, and lattice points per period of it.
const TEXTURE_PERIOD: i32 = 12;
const TEXTURE_LATTICE: i32 = 4;
/// Lattice points of the tongue height noise per loop.
const TIME_LATTICE: i32 = 4;

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub(crate) fn hash(seed: u32, a: i32, b: i32, c: i32) -> u32 {
    let mut z = mix64(u64::from(seed).wrapping_add(0x9E37_79B9_7F4A_7C15));
    for v in [a, b, c] {
        z = mix64(z ^ u64::from(v as u32).wrapping_mul(0xD6E8_FEB8_6659_FD93));
    }
    (z >> 32) as u32
}

fn smooth(t: i32) -> i32 {
    (t * t * (768 - 2 * t)) >> 16
}

fn lattice(seed: u32, i: i32, j: i32) -> i32 {
    (hash(seed, i, j, 0) % 1001) as i32
}

/// Value noise from 0 to 1000. Coordinates are lattice units times 256; a positive `wrap_v`
/// repeats the noise every `wrap_v` lattice rows.
fn noise(seed: u32, u: i32, v: i32, wrap_v: i32) -> i32 {
    let (iu, fu) = (u.div_euclid(256), u.rem_euclid(256));
    let (iv, fv) = (v.div_euclid(256), v.rem_euclid(256));
    let row = |j: i32| if wrap_v > 0 { j.rem_euclid(wrap_v) } else { j };
    let (v0, v1) = (row(iv), row(iv + 1));
    let (a, b) = (lattice(seed, iu, v0), lattice(seed, iu + 1, v0));
    let (c, d) = (lattice(seed, iu, v1), lattice(seed, iu + 1, v1));
    let su = smooth(fu);
    let sv = smooth(fv);
    let top = a + (b - a) * su / 256;
    let bottom = c + (d - c) * su / 256;
    top + (bottom - top) * sv / 256
}

/// Palette steps from cold to hot, built from the palette's own colors: the shadow color at
/// the cold end, then the darker two thirds of the palette by brightness. The brightest
/// stops stay with the lettering, so the letters read as the hottest thing in the picture.
pub(crate) fn ramp(stops: &[Rgb], shadow: Rgb) -> Vec<Rgb> {
    let mut sorted = stops.to_vec();
    sorted.sort_by_key(|c| (c.luma(), c.0, c.1, c.2));
    let mut anchors = vec![shadow];
    anchors.extend(sorted[..=sorted.len() * 2 / 3].iter().copied());
    let last = anchors.len() - 1;
    (0..LEVELS)
        .map(|k| {
            let at = k * last * 100 / (LEVELS - 1);
            let (lo, frac) = (at / 100, (at % 100) as u32);
            if lo >= last {
                anchors[last]
            } else {
                anchors[lo].mix(anchors[lo + 1], frac)
            }
        })
        .collect()
}

/// Free cells within reach of a surface, with their distance above it in cells.
pub(crate) struct FlameMap {
    cells: Vec<(u32, u32, u32)>,
    height: u32,
}

impl FlameMap {
    /// Flames rise from the topmost solid cell of every column, never into counters. A cell's
    /// distance is the number of steps down to that cell, or one more than the same for a
    /// neighbouring column, so flames lean a cell sideways.
    pub fn new(body: &Mask, height: u32) -> FlameMap {
        let (w, h) = (body.w as usize, body.h as usize);
        const FAR: u32 = u32::MAX / 2;
        let mut column = vec![FAR; w * h];
        let mut covered = vec![false; w * h];
        for x in 0..w {
            let top = (0..h).find(|&y| body.get(x as i64, y as i64));
            for y in 0..h {
                match top {
                    Some(t) if y < t => column[y * w + x] = (t - y) as u32,
                    Some(_) => covered[y * w + x] = true,
                    None => {}
                }
            }
        }
        let mut cells = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if covered[y * w + x] {
                    continue;
                }
                let mut d = column[y * w + x];
                if x > 0 {
                    d = d.min(column[y * w + x - 1] + 1);
                }
                if x + 1 < w {
                    d = d.min(column[y * w + x + 1] + 1);
                }
                if d <= height {
                    cells.push((x as u32, y as u32, d));
                }
            }
        }
        FlameMap { cells, height }
    }

    /// Colored flame cells of frame `i` out of `n`.
    pub fn frame(
        &self,
        ramp: &[Rgb],
        contact: Rgb,
        seed: u32,
        i: u32,
        n: u32,
    ) -> Vec<(u32, u32, Rgb)> {
        let i = i64::from(i % n);
        let n = i64::from(n);
        let tip_top = i64::from(self.height) * 256;
        let time = (i * i64::from(TIME_LATTICE) * 256 / n) as i32;
        let scroll = (i * i64::from(TEXTURE_PERIOD) * 256 / n) as i32;
        let per_lattice = TEXTURE_PERIOD / TEXTURE_LATTICE;
        let mut out = Vec::with_capacity(self.cells.len());
        for &(x, y, d) in &self.cells {
            let xi = x as i32;
            let raw = noise(seed, xi * 256 * 2 / TONGUE_HALF_CELLS, time, TIME_LATTICE);
            let tongue = (500 + (raw - 500) * 3 / 2).clamp(0, 1000);
            let tip = tip_top * i64::from(150 + 850 * tongue / 1000) / 1000;
            let at = i64::from(d) * 256 - 128;
            let power = 350 + 650 * i64::from(tongue) / 1000;
            let base = if at >= tip {
                0
            } else {
                (power * (tip - at) / tip) as i32
            };
            let texture = noise(
                seed ^ 0x5bd1_e995,
                xi * 256 * 2 / TONGUE_HALF_CELLS,
                (d as i32 * 256 - 128 - scroll).div_euclid(per_lattice),
                TEXTURE_LATTICE,
            );
            let heat = (base + (texture - 500) * 8 / 10).min(HEAT_MAX);
            if heat < HEAT_MIN {
                continue;
            }
            let level = ((heat - HEAT_MIN) * LEVELS as i32 / (HEAT_MAX - HEAT_MIN + 1)) as usize;
            let color = ramp[level.min(LEVELS - 1)];
            out.push((
                x,
                y,
                if d == 1 {
                    color.mix(contact, 45)
                } else {
                    color
                },
            ));
        }
        out
    }
}
