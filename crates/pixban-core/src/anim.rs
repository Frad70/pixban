//! Animations. Every frame is a pure function of the settings, the frame index and the frame
//! count; frame `n` is frame 0, so loops close without a jump. Motion is in whole cells.
//! The canvas grows once, up front, by the most any frame needs, so all frames share a size.

use crate::color::{Rgb, Rgba};
use crate::error::Error;
use crate::fire::{self, FlameMap};
use crate::font::Font;
use crate::limits::{MAX_ANIM_CELLS, MAX_ANIM_PIXELS};
use crate::render::{render_scene, Extra, Grid, Layers, Pose, Scene, Tint};
use crate::spec::{AnimKind, Spec};

const GLITCH_COLORS: [Rgb; 2] = [Rgb::hex(0x05d9e8), Rgb::hex(0xff2a6d)];
const CURSOR_WIDTH: u32 = 2;
const CURSOR_GAP: u32 = 1;
const BAND_PERCENT: u32 = 50;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Params {
    pub kind: AnimKind,
    pub frames: u32,
    pub frame_ms: u32,
    pub flame: u32,
    pub amplitude: u32,
    pub band: u32,
    pub count: u32,
    pub hues: u32,
}

impl Params {
    pub fn resolve(spec: &Spec, scene: &Scene) -> Params {
        let a = spec.anim.as_ref().expect("animation is set");
        let h = scene.font_height;
        let letters = scene.letters.len() as u32;
        let (frames, frame_ms) = match a.kind {
            AnimKind::Burn => (24, 80),
            AnimKind::Shimmer => (30, 70),
            AnimKind::Wave => (16, 90),
            AnimKind::Type => ((2 * (letters + 1) + 4).clamp(8, 120), 110),
            AnimKind::Glitch => (24, 70),
            AnimKind::Rainbow => (a.hues.unwrap_or(12), 100),
            AnimKind::Sparkle => (16, 100),
        };
        Params {
            kind: a.kind,
            frames: a.frames.unwrap_or(frames),
            frame_ms: a.frame_ms.unwrap_or(frame_ms),
            flame: a.flame.unwrap_or(h.max(3)),
            amplitude: a.amplitude.unwrap_or(match a.kind {
                AnimKind::Glitch => (h / 3).max(2),
                _ => (h / 3).clamp(1, 4),
            }),
            band: a.band.unwrap_or((h * 4 / 7).max(3)),
            count: a.count.unwrap_or(match a.kind {
                AnimKind::Glitch => 3,
                _ => 6,
            }),
            hues: a.hues.unwrap_or(12),
        }
    }

    fn glitch_ghost(&self) -> u32 {
        (self.amplitude / 2).max(1)
    }

    fn extra(&self) -> Extra {
        let mut e = Extra::default();
        match self.kind {
            AnimKind::Burn => {
                e.top = self.flame;
                e.left = 1;
                e.right = 1;
            }
            AnimKind::Wave => {
                e.top = self.amplitude;
                e.bottom = self.amplitude;
            }
            AnimKind::Type => e.right = CURSOR_GAP + CURSOR_WIDTH,
            AnimKind::Glitch => {
                e.left = self.amplitude + self.glitch_ghost();
                e.right = e.left;
            }
            AnimKind::Sparkle => {
                e = Extra {
                    left: 1,
                    top: 1,
                    right: 1,
                    bottom: 1,
                }
            }
            AnimKind::Shimmer | AnimKind::Rainbow => {}
        }
        e
    }
}

/// How an animated SVG is put together; the default rebuilds the picture from frame to frame.
#[derive(Debug)]
pub(crate) enum Plan {
    Frames,
    Shimmer {
        /// Start of the band in x + y, per frame, `None` while the band is away.
        band: Vec<Option<i32>>,
        width: u32,
        percent: u32,
        /// The cells of the lettering.
        ink: Grid,
    },
    Wave {
        base: Grid,
        letters: Vec<Grid>,
        /// Offset per letter and frame.
        offsets: Vec<Vec<(i32, i32)>>,
    },
    Type {
        base: Grid,
        reveal: Vec<Grid>,
        /// Letters shown per frame.
        counts: Vec<u32>,
        /// Top-left cell of the cursor per frame and whether it is lit.
        cursor: Vec<(u32, u32, bool)>,
        cursor_size: (u32, u32),
        cursor_color: Rgb,
    },
}

pub struct Animated {
    pub(crate) frames: Vec<Grid>,
    pub(crate) rest: Grid,
    pub(crate) extra: Extra,
    pub(crate) scale: u32,
    pub(crate) frame_ms: u32,
    pub(crate) title: String,
    pub(crate) missing: Vec<char>,
    pub(crate) kind: AnimKind,
    pub(crate) plan: Plan,
}

impl std::fmt::Debug for Animated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animated")
            .field("kind", &self.kind)
            .field("frames", &self.frames.len())
            .field("size", &(self.rest.width(), self.rest.height()))
            .field("scale", &self.scale)
            .field("frame_ms", &self.frame_ms)
            .finish()
    }
}

impl Animated {
    pub fn frames(&self) -> &[Grid] {
        &self.frames
    }

    /// The still logo on the grown canvas: what a viewer sees when motion is switched off.
    pub fn rest(&self) -> &Grid {
        &self.rest
    }

    /// Cells added left, top, right and bottom of the still logo to make room for motion.
    pub fn margins(&self) -> (u32, u32, u32, u32) {
        (
            self.extra.left,
            self.extra.top,
            self.extra.right,
            self.extra.bottom,
        )
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn frame_ms(&self) -> u32 {
        self.frame_ms
    }

    pub fn kind(&self) -> AnimKind {
        self.kind
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn missing(&self) -> &[char] {
        &self.missing
    }

    pub fn pixel_size(&self) -> (u32, u32) {
        let g = &self.rest;
        (g.width() * self.scale, g.height() * self.scale)
    }

    pub fn to_svg(&self) -> String {
        crate::svg::animated_svg(self)
    }

    pub fn to_gif(&self) -> Result<Vec<u8>, Error> {
        crate::gif::to_gif(self)
    }

    pub fn to_apng(&self) -> Result<Vec<u8>, Error> {
        crate::apng::to_apng(self)
    }

    /// One frame as a still PNG at the output scale.
    pub fn frame_png(&self, index: usize) -> Result<Vec<u8>, Error> {
        crate::png::to_png(&self.frames[index], self.scale)
    }

    /// Frame time as stored in a GIF: multiples of 10 ms, never below 20 ms.
    pub fn gif_frame_ms(&self) -> u32 {
        crate::gif::delay_ms(self.frame_ms)
    }
}

/// Everything about a frame that does not change between frames.
struct Kit<'a> {
    scene: &'a Scene,
    p: Params,
    seed: u32,
    rest_layers: Layers,
    flames: Option<(FlameMap, Vec<Rgb>)>,
    sparks: Vec<(u32, u32, u32)>,
    /// Type: the lettering with 0, 1, 2 ... letters shown, without the cursor.
    shown: Vec<Grid>,
    letters: u32,
}

fn tag(kind: AnimKind) -> i32 {
    kind as i32 + 1
}

impl<'a> Kit<'a> {
    fn new(scene: &'a Scene, p: Params) -> Kit<'a> {
        let rest_layers = scene.layers(&Pose::default());
        let seed = scene.spec.seed;
        let flames = (p.kind == AnimKind::Burn).then(|| {
            (
                FlameMap::new(&rest_layers.body, p.flame),
                fire::ramp(&scene.flame_stops, scene.flame_shadow),
            )
        });
        let mut sparks = Vec::new();
        if p.kind == AnimKind::Sparkle {
            let (sx, sy, sw, sh) = scene.static_rect();
            let text = &rest_layers.text;
            let mut spots: Vec<(u32, u32)> = Vec::new();
            for y in sy..sy + sh {
                for x in sx..sx + sw {
                    let (xi, yi) = (i64::from(x), i64::from(y));
                    let near = text.get(xi, yi)
                        || text.get(xi - 1, yi)
                        || text.get(xi + 1, yi)
                        || text.get(xi, yi - 1)
                        || text.get(xi, yi + 1);
                    if near {
                        spots.push((x, y));
                    }
                }
            }
            if !spots.is_empty() {
                let n = p.frames as i32;
                for k in 0..p.count as i32 {
                    let (x, y) = spots[fire::hash(seed, tag(p.kind), k, 1) as usize % spots.len()];
                    let start = fire::hash(seed, tag(p.kind), k, 2) % n as u32;
                    sparks.push((x, y, start));
                }
            }
        }
        let letters = scene.letters.len() as u32;
        let mut kit = Kit {
            scene,
            p,
            seed,
            rest_layers,
            flames,
            sparks,
            shown: Vec::new(),
            letters,
        };
        if p.kind == AnimKind::Type {
            kit.shown = (0..=letters).map(|c| kit.letters_shown(c)).collect();
        }
        kit
    }

    fn rest(&self) -> Grid {
        self.scene
            .paint(&self.rest_layers, &Pose::default())
            .flatten()
    }

    fn frame(&self, index: u32) -> Grid {
        let n = self.p.frames;
        let i = index % n;
        let scene = self.scene;
        match self.p.kind {
            AnimKind::Burn => {
                let (map, ramp) = self.flames.as_ref().expect("burn has flames");
                let pose = Pose {
                    underlay: map.frame(ramp, scene.flame_shadow, self.seed, i, n),
                    ..Pose::default()
                };
                scene.paint(&self.rest_layers, &pose).flatten()
            }
            AnimKind::Shimmer => {
                let pose = Pose {
                    tint: match self.band_start(i) {
                        Some(start) => Tint::Band {
                            start,
                            width: self.p.band,
                            percent: BAND_PERCENT,
                        },
                        None => Tint::None,
                    },
                    ..Pose::default()
                };
                scene.paint(&self.rest_layers, &pose).flatten()
            }
            AnimKind::Wave => {
                let pose = Pose {
                    shift: (0..self.letters).map(|j| (0, self.wave_dy(i, j))).collect(),
                    ..Pose::default()
                };
                scene.compose(&pose).flatten()
            }
            AnimKind::Type => {
                let (count, _) = self.type_state(i);
                let mut grid = self.shown[count as usize].clone();
                let (x, y, lit) = self.type_cursor(i);
                if lit {
                    self.paint_cursor(&mut grid, x, y);
                }
                grid
            }
            AnimKind::Glitch => self.glitch_frame(i),
            AnimKind::Rainbow => {
                let steps = (0..self.letters)
                    .map(|j| self.hue_step(i, j))
                    .collect::<Vec<_>>();
                let pose = Pose {
                    tint: Tint::Hue {
                        steps,
                        hues: self.p.hues,
                    },
                    ..Pose::default()
                };
                scene.paint(&self.rest_layers, &pose).flatten()
            }
            AnimKind::Sparkle => {
                let pose = Pose {
                    overlay: self.sparkle_cells(i),
                    ..Pose::default()
                };
                scene.paint(&self.rest_layers, &pose).flatten()
            }
        }
    }

    // shimmer

    fn sweep_frames(&self) -> u32 {
        (self.p.frames * 2 / 3).max(1)
    }

    fn band_start(&self, i: u32) -> Option<i32> {
        let sweep = self.sweep_frames();
        if i >= sweep {
            return None;
        }
        let (cw, ch) = self.scene.canvas();
        let span = i64::from(cw) + i64::from(ch) - 1 + i64::from(self.p.band);
        let start = -i64::from(self.p.band) + i64::from(i) * span / i64::from(sweep);
        Some(start as i32)
    }

    // wave

    fn wave_dy(&self, i: u32, letter: u32) -> i32 {
        let turn = 4096i64;
        let phase = (i64::from(i) * turn / i64::from(self.p.frames) + i64::from(letter) * turn / 8)
            .rem_euclid(turn);
        let half = phase % (turn / 2);
        let peak = 4 * half * (turn / 2 - half) / (turn / 2);
        let signed = if phase < turn / 2 { peak } else { -peak };
        let scaled = i64::from(self.p.amplitude) * signed;
        let rounded = (scaled.abs() + turn / 4) / (turn / 2);
        -(rounded * scaled.signum()) as i32
    }

    // type

    /// Letters shown and whether the cursor is lit in frame `i`.
    fn type_state(&self, i: u32) -> (u32, bool) {
        let n = self.p.frames;
        let letters = self.letters;
        let clear = (n / 10).max(1);
        let rest = n - clear;
        let typing = (rest * 6 / 10).clamp(1, letters + 1);
        if i < typing {
            ((i * (letters + 1) / typing).min(letters), true)
        } else if i < rest {
            (letters, ((i - typing) / 2).is_multiple_of(2))
        } else {
            (0, ((i - rest) / 2).is_multiple_of(2))
        }
    }

    fn letters_shown(&self, count: u32) -> Grid {
        let pose = Pose {
            hidden: (0..self.letters).map(|j| j >= count).collect(),
            ..Pose::default()
        };
        self.scene.compose(&pose).flatten()
    }

    /// Cursor position: right before the next letter, or after the last one.
    fn cursor_cell(&self, count: u32) -> (u32, u32, u32) {
        let scene = self.scene;
        let (ox, _) = scene.origin();
        let letters = &scene.letters;
        let (x, line) = match letters.get(count as usize) {
            Some(l) => (ox + l.x, l.line),
            None => match letters.last() {
                Some(l) => (ox + l.x + l.width + CURSOR_GAP, l.line),
                None => (ox, 0),
            },
        };
        let (_, y, _, h) = scene.line_rect(line);
        (x, y, h)
    }

    /// Cursor cell and whether it is lit in frame `i`.
    fn type_cursor(&self, i: u32) -> (u32, u32, bool) {
        let (count, lit) = self.type_state(i);
        let (x, y, _) = self.cursor_cell(count);
        (x, y, lit)
    }

    fn cursor_size(&self) -> (u32, u32) {
        let (_, _, h) = self.cursor_cell(0);
        (CURSOR_WIDTH, h)
    }

    fn paint_cursor(&self, grid: &mut Grid, x: u32, y: u32) {
        let (w, h) = self.cursor_size();
        for yy in y..(y + h).min(grid.height()) {
            for xx in x..(x + w).min(grid.width()) {
                grid.set(xx, yy, self.scene.body_color);
            }
        }
    }

    // glitch

    fn bursts(&self) -> Vec<(u32, u32)> {
        let n = self.p.frames;
        (0..self.p.count as i32)
            .map(|b| {
                let start = fire::hash(self.seed, tag(self.p.kind), b, 1) % n;
                let len = 1 + fire::hash(self.seed, tag(self.p.kind), b, 2) % 2;
                (start, len)
            })
            .collect()
    }

    fn in_burst(&self, i: u32) -> bool {
        let n = self.p.frames;
        self.bursts()
            .iter()
            .any(|&(start, len)| (i + n - start) % n < len)
    }

    fn glitch_frame(&self, i: u32) -> Grid {
        if !self.in_burst(i) {
            return self.rest();
        }
        let ghost = self.p.glitch_ghost() as i32;
        let pose = Pose {
            ghosts: vec![(-ghost, 0, GLITCH_COLORS[0]), (ghost, 0, GLITCH_COLORS[1])],
            ..Pose::default()
        };
        let mut parts = self.scene.paint(&self.rest_layers, &pose);
        let (top, bottom) = self.scene.text_rows();
        let rows = bottom - top;
        let t = tag(self.p.kind);
        let slices = 1 + fire::hash(self.seed, t, i as i32, 10) % 3;
        for s in 0..slices as i32 {
            let h = |k: i32| fire::hash(self.seed, t, i as i32, 20 + s * 4 + k);
            let height = 1 + h(0) % (rows / 3).max(1);
            let y0 = top + h(1) % rows;
            let size = 1 + h(2) % self.p.amplitude;
            let dx = if h(3) % 2 == 0 {
                size as i32
            } else {
                -(size as i32)
            };
            shift_rows(&mut parts.front, y0, (y0 + height).min(bottom), dx);
        }
        parts.flatten()
    }

    // rainbow

    fn hue_step(&self, i: u32, letter: u32) -> u32 {
        let hues = self.p.hues;
        let turn = i * hues / self.p.frames;
        (turn + letter * hues / self.letters.max(1)) % hues
    }

    // sparkle

    fn sparkle_cells(&self, i: u32) -> Vec<(u32, u32, Rgb)> {
        let n = self.p.frames;
        let stage = (n / 16).max(1);
        let bright = self
            .scene
            .flame_stops
            .iter()
            .copied()
            .max_by_key(|c| c.luma())
            .unwrap_or(Rgb::WHITE);
        let core = bright.mix(Rgb::WHITE, 85);
        let arm = bright.mix(Rgb::WHITE, 55);
        let (cw, ch) = self.scene.canvas();
        let mut out = Vec::new();
        for &(x, y, start) in &self.sparks {
            let age = (i + n - start) % n;
            if age >= 3 * stage {
                continue;
            }
            out.push((x, y, core));
            if age / stage == 1 {
                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (ax, ay) = (i64::from(x) + dx, i64::from(y) + dy);
                    if ax >= 0 && ay >= 0 && ax < i64::from(cw) && ay < i64::from(ch) {
                        out.push((ax as u32, ay as u32, arm));
                    }
                }
            }
        }
        out
    }
}

fn shift_rows(grid: &mut Grid, y0: u32, y1: u32, dx: i32) {
    let w = grid.width() as i64;
    for y in y0..y1 {
        let row: Vec<Rgba> = grid.row(y).to_vec();
        for x in 0..w {
            let from = x - i64::from(dx);
            let cell = if (0..w).contains(&from) {
                row[from as usize]
            } else {
                Rgba::TRANSPARENT
            };
            grid.set_cell(x as u32, y, cell);
        }
    }
}

fn prepare(spec: &Spec, font: &Font) -> Result<(Scene, Params), Error> {
    if spec.anim.is_none() {
        return Err(Error::InvalidSetting(
            "no animation set, choose one with --anim or the anim table".to_string(),
        ));
    }
    let mut scene = render_scene(spec, font)?;
    let p = Params::resolve(spec, &scene);
    scene.set_extra(p.extra())?;
    let (cw, ch) = scene.canvas();
    let cells = u64::from(cw) * u64::from(ch);
    let pixels = cells * u64::from(scene.scale) * u64::from(scene.scale);
    if cells * u64::from(p.frames) > MAX_ANIM_CELLS
        || pixels * u64::from(p.frames) > MAX_ANIM_PIXELS
    {
        return Err(Error::AnimationTooLarge {
            frames: p.frames,
            cells,
            pixels,
        });
    }
    Ok((scene, p))
}

/// Renders every frame of the animation set in `spec`.
pub fn render_animation(spec: &Spec, font: &Font) -> Result<Animated, Error> {
    let (scene, p) = prepare(spec, font)?;
    let kit = Kit::new(&scene, p);
    let frames: Vec<Grid> = (0..p.frames).map(|i| kit.frame(i)).collect();
    let rest = kit.rest();
    let plan = kit.plan(&frames, &rest);
    Ok(Animated {
        frames,
        rest,
        extra: scene.extra(),
        scale: scene.scale,
        frame_ms: p.frame_ms,
        title: spec.text.clone(),
        missing: scene.missing.clone(),
        kind: p.kind,
        plan,
    })
}

/// One frame of the animation set in `spec`. The index counts on: frame `n` is frame 0.
pub fn render_frame(spec: &Spec, font: &Font, index: u32) -> Result<Grid, Error> {
    let (scene, p) = prepare(spec, font)?;
    Ok(Kit::new(&scene, p).frame(index))
}

impl Kit<'_> {
    fn plan(&self, frames: &[Grid], rest: &Grid) -> Plan {
        match self.p.kind {
            AnimKind::Shimmer => {
                let mut ink = rest.blank_like();
                for (x, y) in self.rest_layers.text.cells() {
                    ink.set(x, y, Rgb::WHITE);
                }
                Plan::Shimmer {
                    band: (0..self.p.frames).map(|i| self.band_start(i)).collect(),
                    width: self.p.band,
                    percent: BAND_PERCENT,
                    ink,
                }
            }
            AnimKind::Wave => self.wave_plan(frames).unwrap_or(Plan::Frames),
            AnimKind::Type => self.type_plan().unwrap_or(Plan::Frames),
            _ => Plan::Frames,
        }
    }

    fn wave_plan(&self, frames: &[Grid]) -> Option<Plan> {
        let all_hidden = Pose {
            hidden: vec![true; self.letters as usize],
            ..Pose::default()
        };
        let base = self.scene.compose(&all_hidden).flatten();
        let letters: Vec<Grid> = (0..self.letters)
            .map(|j| {
                let pose = Pose {
                    hidden: (0..self.letters).map(|k| k != j).collect(),
                    ..Pose::default()
                };
                self.scene.compose(&pose).front
            })
            .collect();
        let offsets: Vec<Vec<(i32, i32)>> = (0..self.letters)
            .map(|j| {
                (0..self.p.frames)
                    .map(|i| (0, self.wave_dy(i, j)))
                    .collect()
            })
            .collect();
        for (i, frame) in frames.iter().enumerate() {
            let mut grid = base.clone();
            for (j, layer) in letters.iter().enumerate() {
                let (dx, dy) = offsets[j][i];
                grid.overlay_shifted(layer, dx, dy);
            }
            if grid != *frame {
                return None;
            }
        }
        Some(Plan::Wave {
            base,
            letters,
            offsets,
        })
    }

    fn type_plan(&self) -> Option<Plan> {
        let shown = &self.shown;
        let mut reveal = Vec::new();
        for c in 0..self.letters as usize {
            reveal.push(crate::svg::added_cells(&shown[c], &shown[c + 1])?);
        }
        let counts: Vec<u32> = (0..self.p.frames).map(|i| self.type_state(i).0).collect();
        let cursor: Vec<(u32, u32, bool)> =
            (0..self.p.frames).map(|i| self.type_cursor(i)).collect();
        Some(Plan::Type {
            base: shown[0].clone(),
            reveal,
            counts,
            cursor,
            cursor_size: self.cursor_size(),
            cursor_color: self.scene.body_color,
        })
    }
}
