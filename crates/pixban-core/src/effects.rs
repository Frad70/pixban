//! Cell-level building blocks of the render pipeline. Everything here is a pure function of
//! its inputs; nothing reads global state or the clock.

use crate::color::Rgb;
use crate::spec::{Connectivity, Dither, TextureKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mask {
    pub w: u32,
    pub h: u32,
    bits: Vec<bool>,
}

impl Mask {
    pub fn new(w: u32, h: u32) -> Mask {
        Mask {
            w,
            h,
            bits: vec![false; w as usize * h as usize],
        }
    }

    /// Reads a cell; anything outside the mask is empty.
    pub fn get(&self, x: i64, y: i64) -> bool {
        x >= 0
            && y >= 0
            && x < i64::from(self.w)
            && y < i64::from(self.h)
            && self.bits[y as usize * self.w as usize + x as usize]
    }

    pub fn set(&mut self, x: u32, y: u32) {
        self.bits[y as usize * self.w as usize + x as usize] = true;
    }

    pub fn union_with(&mut self, other: &Mask) {
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a |= *b;
        }
    }

    pub fn minus(&self, other: &Mask) -> Mask {
        Mask {
            w: self.w,
            h: self.h,
            bits: self
                .bits
                .iter()
                .zip(&other.bits)
                .map(|(a, b)| *a && !*b)
                .collect(),
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        let w = self.w as usize;
        self.bits
            .iter()
            .enumerate()
            .filter(|(_, on)| **on)
            .map(move |(i, _)| ((i % w) as u32, (i / w) as u32))
    }

    /// The same cells moved by (dx, dy); cells pushed past the edge are dropped.
    pub fn shifted(&self, dx: i64, dy: i64) -> Mask {
        let mut out = Mask::new(self.w, self.h);
        for (x, y) in self.cells() {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if nx >= 0 && ny >= 0 && nx < i64::from(self.w) && ny < i64::from(self.h) {
                out.set(nx as u32, ny as u32);
            }
        }
        out
    }
}

/// Grows the mask by `width` cells: a diamond for 4-connected, a square for 8-connected.
pub(crate) fn dilate(mask: &Mask, width: u32, conn: Connectivity) -> Mask {
    let mut cur = mask.clone();
    for _ in 0..width {
        let mut next = cur.clone();
        for (x, y) in cur.cells() {
            for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                put(&mut next, i64::from(x) + dx, i64::from(y) + dy);
            }
            if conn == Connectivity::Eight {
                for (dx, dy) in [(-1i64, -1i64), (1, -1), (-1, 1), (1, 1)] {
                    put(&mut next, i64::from(x) + dx, i64::from(y) + dy);
                }
            }
        }
        cur = next;
    }
    cur
}

fn put(mask: &mut Mask, x: i64, y: i64) {
    if x >= 0 && y >= 0 && x < i64::from(mask.w) && y < i64::from(mask.h) {
        mask.set(x as u32, y as u32);
    }
}

/// For every cell the smallest step `k` in `1..=length` whose copy of `body`, moved by
/// `k * (dx, dy)`, covers it; 0 where no copy reaches. Row-major, `mask.w` wide.
pub(crate) fn extrusion_steps(body: &Mask, dx: i32, dy: i32, length: u32) -> Vec<u8> {
    let mut steps = vec![0u8; body.w as usize * body.h as usize];
    for k in (1..=length).rev() {
        let (ox, oy) = (i64::from(dx) * i64::from(k), i64::from(dy) * i64::from(k));
        for (x, y) in body.cells() {
            let (nx, ny) = (i64::from(x) + ox, i64::from(y) + oy);
            if nx >= 0 && ny >= 0 && nx < i64::from(body.w) && ny < i64::from(body.h) {
                steps[ny as usize * body.w as usize + nx as usize] = k as u8;
            }
        }
    }
    steps
}

/// Color of extrusion step `k`: `step` percent more of the way to `target` each step.
pub(crate) fn extrusion_color(base: Rgb, target: Rgb, step: u32, k: u32) -> Rgb {
    base.mix(target, step.saturating_mul(k))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Light,
    Dark,
    None,
}

/// Bevel class of a filled cell: lit on the top-left edge, shaded on the bottom-right edge,
/// and untouched when both sides are open (thin strokes, top-right and bottom-left corners).
pub(crate) fn bevel_edge(mask: &Mask, x: u32, y: u32) -> Edge {
    let (x, y) = (i64::from(x), i64::from(y));
    let open = |dx: i64, dy: i64| !mask.get(x + dx, y + dy);
    let light = u8::from(open(0, -1)) + u8::from(open(-1, 0));
    let dark = u8::from(open(0, 1)) + u8::from(open(1, 0));
    match light.cmp(&dark) {
        std::cmp::Ordering::Greater => Edge::Light,
        std::cmp::Ordering::Less => Edge::Dark,
        std::cmp::Ordering::Equal => Edge::None,
    }
}

pub(crate) fn apply_bevel(color: Rgb, edge: Edge, amount: u32) -> Rgb {
    match edge {
        Edge::Light => color.mix(Rgb::WHITE, amount),
        Edge::Dark => color.mix(Rgb::BLACK, amount),
        Edge::None => color,
    }
}

const BAYER2: [[u32; 2]; 2] = [[0, 2], [3, 1]];
const BAYER4: [[u32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// True when the fraction `rem / den` of the way to the next stop beats the Bayer threshold
/// at (x, y). A fraction of 0 never does, so cells sitting on a stop stay pure.
pub(crate) fn bayer_pick(dither: Dither, rem: u64, den: u64, x: u32, y: u32) -> bool {
    let (cell, levels) = match dither {
        Dither::None => return false,
        Dither::Bayer2 => (BAYER2[(y % 2) as usize][(x % 2) as usize], 4),
        Dither::Bayer4 => (BAYER4[(y % 4) as usize][(x % 4) as usize], 16),
    };
    2 * rem * levels > (2 * u64::from(cell) + 1) * den
}

/// Gradient color at `pos` of `len` positions. With dither on, only the two stops around the
/// exact position are mixed; without it the nearest stop wins.
pub(crate) fn gradient_color(
    stops: &[Rgb],
    pos: u32,
    len: u32,
    dither: Dither,
    x: u32,
    y: u32,
) -> Rgb {
    let n = stops.len();
    if dither == Dither::None || len <= 1 || n <= 1 {
        return stops[crate::palette::stop_index(pos, len, n)];
    }
    let num = u64::from(pos.min(len - 1)) * (n as u64 - 1);
    let den = u64::from(len - 1);
    let lo = (num / den) as usize;
    if lo >= n - 1 {
        return stops[n - 1];
    }
    if bayer_pick(dither, num % den, den, x, y) {
        stops[lo + 1]
    } else {
        stops[lo]
    }
}

fn hash(seed: u32, x: u32, y: u32) -> u64 {
    let mut z =
        (u64::from(seed) << 40) ^ (u64::from(x) << 20) ^ u64::from(y) ^ 0x9E37_79B9_7F4A_7C15;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Texture applied to a fill color. Noise picks one of three shades per cell, so a texture
/// at most triples the number of colors.
pub(crate) fn texture_color(
    color: Rgb,
    kind: TextureKind,
    strength: u32,
    period: u32,
    seed: u32,
    x: u32,
    y: u32,
) -> Rgb {
    let darker = match kind {
        TextureKind::Scanlines => y % period == period - 1,
        TextureKind::Checker => (x + y) % 2 == 1,
        TextureKind::Noise => match (hash(seed, x, y) >> 33) % 3 {
            0 => return color.mix(Rgb::BLACK, strength),
            1 => return color,
            _ => return color.mix(Rgb::WHITE, strength),
        },
    };
    if darker {
        color.mix(Rgb::BLACK, strength)
    } else {
        color
    }
}

/// Color of a plate cell, or `None` for a cut corner.
pub(crate) fn plate_cell(
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    border: u32,
    cut_corners: bool,
) -> Option<PlateCell> {
    let (right, bottom) = (x == w - 1, y == h - 1);
    if cut_corners && (x == 0 || right) && (y == 0 || bottom) {
        return None;
    }
    let edge = x < border || y < border || x + border >= w || y + border >= h;
    Some(if edge {
        PlateCell::Border
    } else {
        PlateCell::Body
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlateCell {
    Border,
    Body,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(rows: &[&str]) -> Mask {
        let mut m = Mask::new(rows[0].len() as u32, rows.len() as u32);
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                if c == '#' {
                    m.set(x as u32, y as u32);
                }
            }
        }
        m
    }

    fn rows(m: &Mask) -> Vec<String> {
        (0..m.h)
            .map(|y| {
                (0..m.w)
                    .map(|x| {
                        if m.get(i64::from(x), i64::from(y)) {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn dilation_four_connected_is_a_diamond() {
        let m = mask(&[".....", ".....", "..#..", ".....", "....."]);
        assert_eq!(
            rows(&dilate(&m, 1, Connectivity::Four)),
            [".....", "..#..", ".###.", "..#..", "....."]
        );
        assert_eq!(
            rows(&dilate(&m, 2, Connectivity::Four)),
            ["..#..", ".###.", "#####", ".###.", "..#.."]
        );
    }

    #[test]
    fn dilation_eight_connected_is_a_square() {
        let m = mask(&[".....", ".....", "..#..", ".....", "....."]);
        assert_eq!(
            rows(&dilate(&m, 1, Connectivity::Eight)),
            [".....", ".###.", ".###.", ".###.", "....."]
        );
        assert_eq!(rows(&dilate(&m, 2, Connectivity::Eight)), vec!["#####"; 5]);
    }

    #[test]
    fn outline_is_the_dilation_minus_the_letter() {
        let m = mask(&["....", ".##.", "...."]);
        let ring4 = dilate(&m, 1, Connectivity::Four).minus(&m);
        assert_eq!(rows(&ring4), [".##.", "#..#", ".##."]);
        let ring8 = dilate(&m, 1, Connectivity::Eight).minus(&m);
        assert_eq!(rows(&ring8), ["####", "#..#", "####"]);
    }

    #[test]
    fn extrusion_steps_count_from_the_nearest_copy() {
        let m = mask(&["#...", "....", "...."]);
        let steps = extrusion_steps(&m, 1, 1, 2);
        assert_eq!(steps, [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2, 0]);
        let m = mask(&[".#...", "....."]);
        assert_eq!(
            extrusion_steps(&m, -1, 0, 1),
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let wide = mask(&["##...."]);
        assert_eq!(extrusion_steps(&wide, 1, 0, 3), [0, 1, 1, 2, 3, 0]);
    }

    #[test]
    fn extrusion_colors_darken_by_a_fixed_step_and_stop_at_the_target() {
        let (base, target) = (Rgb(200, 100, 0), Rgb(0, 0, 0));
        let got: Vec<Rgb> = (1..=4)
            .map(|k| extrusion_color(base, target, 25, k))
            .collect();
        assert_eq!(
            got,
            [
                Rgb(150, 75, 0),
                Rgb(100, 50, 0),
                Rgb(50, 25, 0),
                Rgb(0, 0, 0)
            ]
        );
        assert_eq!(extrusion_color(base, target, 60, 9), target);
    }

    #[test]
    fn bevel_lights_top_left_and_shades_bottom_right() {
        let m = mask(&["###", "###", "###"]);
        let edges: Vec<Vec<Edge>> = (0..3)
            .map(|y| (0..3).map(|x| bevel_edge(&m, x, y)).collect())
            .collect();
        use Edge::{Dark as D, Light as L, None as N};
        assert_eq!(edges, [[L, L, N], [L, N, D], [N, D, D]]);
    }

    #[test]
    fn bevel_leaves_the_middle_of_thin_strokes_alone() {
        let m = mask(&["#", "#", "#"]);
        let got: Vec<Edge> = (0..3).map(|y| bevel_edge(&m, 0, y)).collect();
        assert_eq!(got, [Edge::Light, Edge::None, Edge::Dark]);
        let bar = mask(&["###"]);
        assert_eq!(bevel_edge(&bar, 1, 0), Edge::None);
    }

    #[test]
    fn bevel_amount_mixes_toward_white_or_black() {
        let c = Rgb(100, 100, 100);
        assert_eq!(apply_bevel(c, Edge::Light, 50), Rgb(178, 178, 178));
        assert_eq!(apply_bevel(c, Edge::Dark, 50), Rgb(50, 50, 50));
        assert_eq!(apply_bevel(c, Edge::None, 50), c);
    }

    #[test]
    fn bayer_two_by_two_pattern_for_a_half_way_position() {
        let pick = |x, y| bayer_pick(Dither::Bayer2, 1, 2, x, y);
        let pattern: Vec<Vec<bool>> = (0..2)
            .map(|y| (0..2).map(|x| pick(x, y)).collect())
            .collect();
        assert_eq!(pattern, [[true, false], [false, true]]);
    }

    #[test]
    fn bayer_four_by_four_picks_a_quarter_of_the_cells_at_a_quarter() {
        for (rem, den, want) in [(0, 4, 0), (1, 4, 4), (2, 4, 8), (3, 4, 12), (4, 4, 16)] {
            let on = (0..4)
                .flat_map(|y| (0..4).map(move |x| (x, y)))
                .filter(|&(x, y)| bayer_pick(Dither::Bayer4, rem, den, x, y))
                .count();
            assert_eq!(on, want, "{rem}/{den}");
        }
        assert!(!bayer_pick(Dither::None, 1, 1, 0, 0));
    }

    #[test]
    fn dithered_gradient_only_uses_the_two_stops_around_the_position() {
        let stops = [Rgb(0, 0, 0), Rgb(100, 0, 0), Rgb(200, 0, 0)];
        for pos in 0..9 {
            let lo = (pos as usize * 2 / 8).min(1);
            let hi = lo + 1;
            for x in 0..4 {
                for y in 0..4 {
                    let c = gradient_color(&stops, pos, 9, Dither::Bayer4, x, y);
                    assert!(c == stops[lo] || c == stops[hi] || pos == 8, "pos {pos}");
                }
            }
        }
        assert_eq!(gradient_color(&stops, 0, 9, Dither::Bayer4, 3, 3), stops[0]);
        assert_eq!(gradient_color(&stops, 4, 9, Dither::Bayer4, 3, 3), stops[1]);
        assert_eq!(gradient_color(&stops, 8, 9, Dither::Bayer4, 0, 0), stops[2]);
    }

    #[test]
    fn dithered_gradient_between_two_stops_makes_a_checkerboard_halfway() {
        let stops = [Rgb(0, 0, 0), Rgb(255, 255, 255)];
        let at = |x, y| gradient_color(&stops, 2, 5, Dither::Bayer2, x, y);
        assert_eq!(at(0, 0), stops[1]);
        assert_eq!(at(1, 0), stops[0]);
        assert_eq!(at(0, 1), stops[0]);
        assert_eq!(at(1, 1), stops[1]);
    }

    #[test]
    fn textures_are_deterministic_and_use_few_shades() {
        let c = Rgb(200, 200, 200);
        let scan: Vec<bool> = (0..6)
            .map(|y| texture_color(c, TextureKind::Scanlines, 50, 3, 0, 0, y) != c)
            .collect();
        assert_eq!(scan, [false, false, true, false, false, true]);
        let chk: Vec<bool> = (0..4)
            .map(|x| texture_color(c, TextureKind::Checker, 50, 2, 0, x, 1) != c)
            .collect();
        assert_eq!(chk, [true, false, true, false]);

        let mut shades = std::collections::HashSet::new();
        for y in 0..40 {
            for x in 0..40 {
                let a = texture_color(c, TextureKind::Noise, 20, 2, 7, x, y);
                assert_eq!(a, texture_color(c, TextureKind::Noise, 20, 2, 7, x, y));
                shades.insert(a);
            }
        }
        assert_eq!(shades.len(), 3);
        let differs = (0..40).any(|x| {
            texture_color(c, TextureKind::Noise, 20, 2, 7, x, 0)
                != texture_color(c, TextureKind::Noise, 20, 2, 8, x, 0)
        });
        assert!(differs, "the seed must change the pattern");
    }

    #[test]
    fn plate_border_rings_and_cut_corners() {
        let draw = |border, cut| -> Vec<String> {
            (0..5)
                .map(|y| {
                    (0..6)
                        .map(|x| match plate_cell(x, y, 6, 5, border, cut) {
                            None => ' ',
                            Some(PlateCell::Border) => 'b',
                            Some(PlateCell::Body) => 'p',
                        })
                        .collect()
                })
                .collect()
        };
        assert_eq!(draw(0, false), vec!["pppppp"; 5]);
        assert_eq!(
            draw(1, false),
            ["bbbbbb", "bppppb", "bppppb", "bppppb", "bbbbbb"]
        );
        assert_eq!(
            draw(1, true),
            [" bbbb ", "bppppb", "bppppb", "bppppb", " bbbb "]
        );
        assert_eq!(
            draw(2, false),
            ["bbbbbb", "bbbbbb", "bbppbb", "bbbbbb", "bbbbbb"]
        );
    }
}
