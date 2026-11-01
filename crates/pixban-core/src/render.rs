//! Rendering pipeline. Layers are painted back to front in this fixed order:
//! background (the solid `bg`, then the plate with its border), shadow, extrusion, outline,
//! letter fill (gradient or dither, per-letter color, then texture), bevel edges on the fill.
//! A layer only ever overwrites the layers before it, so the order never depends on the options.

use crate::color::{Rgb, Rgba};
use crate::effects::{self, Mask, PlateCell};
use crate::error::Error;
use crate::font::Font;
use crate::layout::{layout, LayoutOptions};
use crate::limits::{MAX_GRID_SIDE, MAX_OUTPUT_PIXELS, MAX_OUTPUT_SIDE};
use crate::palette::{self, stop_index};
use crate::spec::{Direction, Dither, Spec, Texture};

const PLATE_COLOR: Rgb = Rgb::hex(0x0d1117);
const PLATE_PADDING: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    width: u32,
    height: u32,
    cells: Vec<Rgba>,
    background: Option<Rgb>,
}

impl Grid {
    pub(crate) fn new(width: u32, height: u32, background: Option<Rgb>) -> Grid {
        let fill = background.map_or(Rgba::TRANSPARENT, Rgba::from);
        Grid {
            width,
            height,
            cells: vec![fill; width as usize * height as usize],
            background,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn background(&self) -> Option<Rgb> {
        self.background
    }

    pub fn get(&self, x: u32, y: u32) -> Rgba {
        self.cells[y as usize * self.width as usize + x as usize]
    }

    pub fn row(&self, y: u32) -> &[Rgba] {
        let w = self.width as usize;
        &self.cells[y as usize * w..(y as usize + 1) * w]
    }

    pub(crate) fn set(&mut self, x: u32, y: u32, c: Rgb) {
        self.cells[y as usize * self.width as usize + x as usize] = c.into();
    }

    pub(crate) fn set_cell(&mut self, x: u32, y: u32, c: Rgba) {
        self.cells[y as usize * self.width as usize + x as usize] = c;
    }

    /// A transparent grid of the same size, without a background color.
    pub(crate) fn blank_like(&self) -> Grid {
        Grid::new(self.width, self.height, None)
    }

    /// Paints every opaque cell of `top` over this grid.
    pub(crate) fn overlay(&mut self, top: &Grid) {
        for (cell, over) in self.cells.iter_mut().zip(&top.cells) {
            if !over.is_transparent() {
                *cell = *over;
            }
        }
    }

    /// Like `overlay`, with `top` moved by (dx, dy); cells moved off the grid are dropped.
    pub(crate) fn overlay_shifted(&mut self, top: &Grid, dx: i32, dy: i32) {
        for y in 0..top.height {
            for x in 0..top.width {
                let cell = top.get(x, y);
                let (nx, ny) = (i64::from(x) + i64::from(dx), i64::from(y) + i64::from(dy));
                if !cell.is_transparent()
                    && nx >= 0
                    && ny >= 0
                    && nx < i64::from(self.width)
                    && ny < i64::from(self.height)
                {
                    self.set_cell(nx as u32, ny as u32, cell);
                }
            }
        }
    }

    /// The part of the grid left after cutting the given number of cells off each side.
    pub fn cropped(&self, left: u32, top: u32, right: u32, bottom: u32) -> Grid {
        let (w, h) = (self.width - left - right, self.height - top - bottom);
        let mut out = Grid::new(w, h, self.background);
        for y in 0..h {
            for x in 0..w {
                out.set_cell(x, y, self.get(x + left, y + top));
            }
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct Rendered {
    pub grid: Grid,
    pub scale: u32,
    pub title: String,
    pub missing: Vec<char>,
}

impl Rendered {
    pub fn pixel_size(&self) -> (u32, u32) {
        (self.grid.width * self.scale, self.grid.height * self.scale)
    }

    pub fn to_svg(&self) -> String {
        crate::svg::to_svg(&self.grid, self.scale, &self.title)
    }

    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        crate::png::to_png(&self.grid, self.scale)
    }
}

enum Paint {
    Solid(Rgb),
    Gradient {
        stops: Vec<Rgb>,
        direction: Direction,
        dither: Dither,
    },
    PerLetter(Vec<Rgb>),
    Letters(Vec<Rgb>),
}

struct Margins {
    left: u64,
    right: u64,
    top: u64,
    bottom: u64,
}

fn margins(spec: &Spec) -> Margins {
    let outline = i64::from(spec.outline.as_ref().map_or(0, |o| o.width));
    let (ex, ey) = spec.extrude.as_ref().map_or((0, 0), |e| {
        (
            i64::from(e.dx) * i64::from(e.length),
            i64::from(e.dy) * i64::from(e.length),
        )
    });
    let (sx, sy) = spec
        .shadow
        .as_ref()
        .map_or((0, 0), |s| (i64::from(s.dx), i64::from(s.dy)));
    let border = spec.plate.as_ref().map_or(0, |p| i64::from(p.border));
    let padding = i64::from(spec.padding.unwrap_or(if spec.plate.is_some() {
        PLATE_PADDING
    } else {
        0
    }));
    let ring = outline + padding + border;
    let side = |extrude: i64, shadow: i64| {
        (
            (ring + (-extrude).max(0) + (-shadow).max(0)) as u64,
            (ring + extrude.max(0) + shadow.max(0)) as u64,
        )
    };
    let ((left, right), (top, bottom)) = (side(ex, sx), side(ey, sy));
    Margins {
        left,
        right,
        top,
        bottom,
    }
}

struct LineGeom {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct Ink {
    paint: Paint,
    layout_w: u32,
    lines: Vec<LineGeom>,
    /// Letter number of every layout cell with ink, `u32::MAX` elsewhere.
    letters: Vec<u32>,
    total_letters: u32,
    /// Index into `lines` for every layout row.
    row_line: Vec<usize>,
    texture: Option<(Texture, u32)>,
}

impl Ink {
    fn base_color(&self, x: u32, y: u32) -> Rgb {
        let letter = self.letters[y as usize * self.layout_w as usize + x as usize];
        match &self.paint {
            Paint::Solid(c) => *c,
            Paint::Letters(colors) => colors[letter as usize % colors.len()],
            Paint::PerLetter(stops) => stops[stop_index(letter, self.total_letters, stops.len())],
            Paint::Gradient {
                stops,
                direction,
                dither,
            } => {
                let line = &self.lines[self.row_line[y as usize]];
                let (lx, ly) = (x - line.x, y - line.y);
                let (pos, len) = match direction {
                    Direction::Vertical => (ly, line.height),
                    Direction::Horizontal => (lx, line.width),
                    Direction::Diagonal => (lx + ly, line.width + line.height - 1),
                };
                effects::gradient_color(stops, pos, len, *dither, x, y)
            }
        }
    }

    fn color_at(&self, x: u32, y: u32) -> Rgb {
        let base = self.base_color(x, y);
        match &self.texture {
            Some((t, seed)) => {
                effects::texture_color(base, t.kind, t.strength, t.period, *seed, x, y)
            }
            None => base,
        }
    }
}

/// Cells added around the static canvas so that moving parts never leave it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Extra {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

/// One letter with ink: its cells in layout coordinates and the box it occupies.
pub(crate) struct Letter {
    pub cells: Vec<(u32, u32)>,
    pub x: u32,
    pub width: u32,
    pub line: usize,
}

/// How the ink of a frame is recolored after the normal fill, texture and bevel.
#[derive(Clone, Debug, Default)]
pub(crate) enum Tint {
    #[default]
    None,
    /// Cells whose x + y lies in `start..start + width` move `percent` toward white.
    Band {
        start: i32,
        width: u32,
        percent: u32,
    },
    /// Hue of every letter turned by its own number of steps out of `hues`.
    Hue { steps: Vec<u32>, hues: u32 },
}

/// What differs between a frame and the still logo. The default is the still logo.
#[derive(Default)]
pub(crate) struct Pose {
    /// Per letter move in cells.
    pub shift: Vec<(i32, i32)>,
    /// Letters left out, per letter.
    pub hidden: Vec<bool>,
    /// Cells painted right after the plate, below every other layer.
    pub underlay: Vec<(u32, u32, Rgb)>,
    /// Copies of the lettering moved by (dx, dy), painted right after the shadow.
    pub ghosts: Vec<(i32, i32, Rgb)>,
    pub tint: Tint,
    /// Cells painted last, over everything.
    pub overlay: Vec<(u32, u32, Rgb)>,
}

/// Masks of the lettering and the layers derived from it.
pub(crate) struct Layers {
    pub text: Mask,
    source: Vec<u32>,
    outline: Option<(Mask, Rgb)>,
    extrusion: Option<(Vec<u8>, Vec<Rgb>)>,
    /// Lettering, outline and extrusion together.
    pub body: Mask,
}

/// A composed frame in two parts, so that rows of the artwork can move without the plate.
pub(crate) struct Parts {
    pub backdrop: Grid,
    pub front: Grid,
}

impl Parts {
    pub fn flatten(self) -> Grid {
        let Parts {
            mut backdrop,
            front,
        } = self;
        backdrop.overlay(&front);
        backdrop
    }
}

pub(crate) struct Scene {
    pub spec: Spec,
    ink: Ink,
    pub letters: Vec<Letter>,
    lay_w: u32,
    lay_h: u32,
    base_w: u32,
    base_h: u32,
    margin_left: u32,
    margin_top: u32,
    extra: Extra,
    pub scale: u32,
    pub missing: Vec<char>,
    pub font_height: u32,
    pub body_color: Rgb,
    darkest: Rgb,
    shadow_default: Rgb,
    /// Ramp stops and shadow color the flames and sparks are drawn from.
    pub flame_stops: Vec<Rgb>,
    pub flame_shadow: Rgb,
}

pub(crate) fn render_scene(spec: &Spec, font: &Font) -> Result<Scene, Error> {
    spec.validate()?;
    let named = match &spec.fill.palette {
        Some(name) => Some(palette::resolve(name)?),
        None => None,
    };
    let fire = palette::find("fire").expect("fire palette exists");
    let stops: Vec<Rgb> = named
        .as_ref()
        .map_or_else(|| fire.stops.to_vec(), |p| p.stops.clone());
    let flame_stops = stops.clone();
    let flame_shadow = named.as_ref().map_or(fire.shadow, |p| p.shadow);

    let paint = if !spec.fill.letter_colors.is_empty() {
        Paint::Letters(spec.fill.letter_colors.clone())
    } else if spec.fill.per_letter {
        Paint::PerLetter(stops)
    } else if let Some(c) = spec.fill.color {
        Paint::Solid(c)
    } else {
        Paint::Gradient {
            stops,
            direction: spec.fill.direction,
            dither: spec.fill.dither,
        }
    };
    let body_color = match &paint {
        Paint::Solid(c) => *c,
        Paint::Gradient { stops, .. } | Paint::PerLetter(stops) => stops[stops.len() / 2],
        Paint::Letters(colors) => {
            let n = colors.len() as u32;
            let mean =
                |f: fn(&Rgb) -> u8| (colors.iter().map(|c| u32::from(f(c))).sum::<u32>() / n) as u8;
            Rgb(mean(|c| c.0), mean(|c| c.1), mean(|c| c.2))
        }
    };
    let darkest = match &paint {
        Paint::Solid(c) => *c,
        Paint::Gradient { stops, .. } | Paint::PerLetter(stops) | Paint::Letters(stops) => stops
            .iter()
            .copied()
            .min_by_key(|c| c.luma())
            .unwrap_or(body_color),
    };
    let shadow_default = match &named {
        Some(p) => p.shadow,
        None => match &paint {
            Paint::Solid(c) => c.darken(30),
            Paint::Letters(_) => body_color.darken(30),
            _ => fire.shadow,
        },
    };

    let scale = spec.scale.unwrap_or_else(|| font.default_scale());
    let options = LayoutOptions {
        align: spec.align,
        line_gap: spec.line_gap,
        spacing: spec.spacing,
    };
    let lay = layout(font, &spec.text, &options)?;

    let m = margins(spec);
    let canvas_w = u64::from(lay.width) + m.left + m.right;
    let canvas_h = u64::from(lay.height) + m.top + m.bottom;
    check_canvas(canvas_w, canvas_h, scale)?;

    let lw = lay.width as usize;
    let mut letter_ids = vec![u32::MAX; lw * lay.height as usize];
    let mut letters: Vec<Letter> = Vec::new();
    for (line_no, line) in lay.lines.iter().enumerate() {
        for p in &line.glyphs {
            if !p.glyph.has_ink() {
                continue;
            }
            let id = letters.len() as u32;
            let mut cells = Vec::new();
            for gy in 0..p.glyph.height() {
                for gx in 0..p.glyph.width() {
                    if p.glyph.get(gx, gy) {
                        cells.push((p.x + gx, p.y + gy));
                        letter_ids[(p.y + gy) as usize * lw + (p.x + gx) as usize] = id;
                    }
                }
            }
            letters.push(Letter {
                cells,
                x: p.x,
                width: p.glyph.width(),
                line: line_no,
            });
        }
    }
    let mut row_line = vec![0usize; lay.height as usize];
    for (i, line) in lay.lines.iter().enumerate() {
        for row in line.y..line.y + line.height {
            row_line[row as usize] = i;
        }
    }
    let lines: Vec<LineGeom> = lay
        .lines
        .iter()
        .map(|l| LineGeom {
            x: l.x,
            y: l.y,
            width: l.width,
            height: l.height,
        })
        .collect();
    let ink = Ink {
        paint,
        layout_w: lay.width,
        lines,
        letters: letter_ids,
        total_letters: letters.len() as u32,
        row_line,
        texture: spec.texture.clone().map(|t| (t, spec.seed)),
    };

    Ok(Scene {
        spec: spec.clone(),
        ink,
        letters,
        lay_w: lay.width,
        lay_h: lay.height,
        base_w: canvas_w as u32,
        base_h: canvas_h as u32,
        margin_left: m.left as u32,
        margin_top: m.top as u32,
        extra: Extra::default(),
        scale,
        missing: lay.missing,
        font_height: font.height(),
        body_color,
        darkest,
        shadow_default,
        flame_stops,
        flame_shadow,
    })
}

fn check_canvas(canvas_w: u64, canvas_h: u64, scale: u32) -> Result<(), Error> {
    if canvas_w > u64::from(MAX_GRID_SIDE) || canvas_h > u64::from(MAX_GRID_SIDE) {
        return Err(Error::CanvasTooLarge {
            width: canvas_w,
            height: canvas_h,
        });
    }
    let (out_w, out_h) = (canvas_w * u64::from(scale), canvas_h * u64::from(scale));
    if out_w > MAX_OUTPUT_SIDE || out_h > MAX_OUTPUT_SIDE || out_w * out_h > MAX_OUTPUT_PIXELS {
        return Err(Error::OutputTooLarge {
            width: out_w,
            height: out_h,
        });
    }
    Ok(())
}

impl Scene {
    pub fn set_extra(&mut self, extra: Extra) -> Result<(), Error> {
        let w = u64::from(self.base_w) + u64::from(extra.left) + u64::from(extra.right);
        let h = u64::from(self.base_h) + u64::from(extra.top) + u64::from(extra.bottom);
        check_canvas(w, h, self.scale)?;
        self.extra = extra;
        Ok(())
    }

    pub fn extra(&self) -> Extra {
        self.extra
    }

    pub fn canvas(&self) -> (u32, u32) {
        (
            self.base_w + self.extra.left + self.extra.right,
            self.base_h + self.extra.top + self.extra.bottom,
        )
    }

    /// Canvas position of layout cell (0, 0).
    pub fn origin(&self) -> (u32, u32) {
        (
            self.margin_left + self.extra.left,
            self.margin_top + self.extra.top,
        )
    }

    /// Canvas rows (first, past the last) taken by the lines of text.
    pub fn text_rows(&self) -> (u32, u32) {
        let oy = self.origin().1;
        (oy, oy + self.lay_h)
    }

    /// Canvas rectangle (x, y, width, height) of the static canvas inside the grown one.
    pub fn static_rect(&self) -> (u32, u32, u32, u32) {
        (self.extra.left, self.extra.top, self.base_w, self.base_h)
    }

    /// Canvas rectangle (x, y, width, height) of the line, with the glyph band as its height.
    pub fn line_rect(&self, line: usize) -> (u32, u32, u32, u32) {
        let (ox, oy) = self.origin();
        let l = &self.ink.lines[line];
        (ox + l.x, oy + l.y, l.width, l.height)
    }

    /// The masks a pose produces: only letter moves and hidden letters matter here.
    pub fn layers(&self, pose: &Pose) -> Layers {
        let (cw, ch) = self.canvas();
        let (ox, oy) = self.origin();
        let spec = &self.spec;

        let mut text = Mask::new(cw, ch);
        let mut source = vec![u32::MAX; cw as usize * ch as usize];
        for (j, letter) in self.letters.iter().enumerate() {
            if pose.hidden.get(j).copied().unwrap_or(false) {
                continue;
            }
            let (dx, dy) = pose.shift.get(j).copied().unwrap_or((0, 0));
            for &(lx, ly) in &letter.cells {
                let x = i64::from(ox + lx) + i64::from(dx);
                let y = i64::from(oy + ly) + i64::from(dy);
                if x < 0 || y < 0 || x >= i64::from(cw) || y >= i64::from(ch) {
                    continue;
                }
                text.set(x as u32, y as u32);
                source[y as usize * cw as usize + x as usize] = ly * self.lay_w + lx;
            }
        }

        let outline = spec.outline.as_ref().map(|o| {
            (
                effects::dilate(&text, o.width, o.connectivity).minus(&text),
                o.color.unwrap_or(self.shadow_default),
            )
        });
        let mut silhouette = text.clone();
        if let Some((ring, _)) = &outline {
            silhouette.union_with(ring);
        }
        let extrusion = spec.extrude.as_ref().map(|e| {
            let steps = effects::extrusion_steps(&silhouette, e.dx, e.dy, e.length);
            let target = e.color.unwrap_or(self.shadow_default);
            let colors: Vec<Rgb> = (0..=e.length)
                .map(|k| effects::extrusion_color(self.darkest, target, e.step, k))
                .collect();
            (steps, colors)
        });
        let mut body = silhouette;
        if let Some((steps, _)) = &extrusion {
            for (i, k) in steps.iter().enumerate() {
                if *k > 0 {
                    body.set((i % cw as usize) as u32, (i / cw as usize) as u32);
                }
            }
        }
        Layers {
            text,
            source,
            outline,
            extrusion,
            body,
        }
    }

    pub fn paint(&self, layers: &Layers, pose: &Pose) -> Parts {
        let (cw, ch) = self.canvas();
        let spec = &self.spec;
        let lw = self.lay_w as usize;
        let Layers {
            text,
            source,
            outline,
            extrusion,
            body,
        } = layers;

        let mut backdrop = Grid::new(cw, ch, spec.bg);
        if let Some(plate) = &spec.plate {
            let body_c = plate.color.unwrap_or(PLATE_COLOR);
            let border_c = plate.border_color.unwrap_or(self.body_color);
            let (el, et) = (self.extra.left, self.extra.top);
            for y in 0..self.base_h {
                for x in 0..self.base_w {
                    match effects::plate_cell(
                        x,
                        y,
                        self.base_w,
                        self.base_h,
                        plate.border,
                        plate.cut_corners,
                    ) {
                        Some(PlateCell::Border) => backdrop.set(x + el, y + et, border_c),
                        Some(PlateCell::Body) => backdrop.set(x + el, y + et, body_c),
                        None => {}
                    }
                }
            }
        }

        let mut front = Grid::new(cw, ch, None);
        for &(x, y, c) in &pose.underlay {
            front.set(x, y, c);
        }
        if let Some(shadow) = &spec.shadow {
            let color = shadow.color.unwrap_or(self.shadow_default);
            let moved = body.shifted(i64::from(shadow.dx), i64::from(shadow.dy));
            for (x, y) in moved.cells() {
                front.set(x, y, color);
            }
        }
        for &(dx, dy, color) in &pose.ghosts {
            for (x, y) in text.shifted(i64::from(dx), i64::from(dy)).cells() {
                front.set(x, y, color);
            }
        }
        if let Some((steps, colors)) = extrusion {
            for (i, k) in steps.iter().enumerate() {
                if *k > 0 {
                    let (x, y) = ((i % cw as usize) as u32, (i / cw as usize) as u32);
                    front.set(x, y, colors[*k as usize]);
                }
            }
        }
        if let Some((ring, color)) = outline {
            for (x, y) in ring.cells() {
                front.set(x, y, *color);
            }
        }
        let bevel = spec.bevel.as_ref().map(|b| b.amount);
        for (x, y) in text.cells() {
            let at = source[y as usize * cw as usize + x as usize] as usize;
            let (lx, ly) = ((at % lw) as u32, (at / lw) as u32);
            let color = self.ink.color_at(lx, ly);
            let color = match bevel {
                Some(amount) => {
                    effects::apply_bevel(color, effects::bevel_edge(text, x, y), amount)
                }
                None => color,
            };
            let color = match &pose.tint {
                Tint::None => color,
                Tint::Band {
                    start,
                    width,
                    percent,
                } => {
                    let along = i64::from(x) + i64::from(y) - i64::from(*start);
                    if (0..i64::from(*width)).contains(&along) {
                        color.mix(Rgb::WHITE, *percent)
                    } else {
                        color
                    }
                }
                Tint::Hue { steps, hues } => {
                    let letter = self.ink.letters[at] as usize;
                    color.turn_hue(steps[letter], *hues)
                }
            };
            front.set(x, y, color);
        }
        for &(x, y, c) in &pose.overlay {
            front.set(x, y, c);
        }

        Parts { backdrop, front }
    }

    pub fn compose(&self, pose: &Pose) -> Parts {
        self.paint(&self.layers(pose), pose)
    }
}

pub fn render(spec: &Spec, font: &Font) -> Result<Rendered, Error> {
    let scene = render_scene(spec, font)?;
    let grid = scene.compose(&Pose::default()).flatten();
    Ok(Rendered {
        grid,
        scale: scene.scale,
        title: spec.text.clone(),
        missing: scene.missing.clone(),
    })
}
