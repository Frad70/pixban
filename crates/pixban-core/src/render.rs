//! Rendering pipeline. Layers are painted back to front in this fixed order:
//! background (the solid `bg`, then the plate with its border), shadow, extrusion, outline,
//! letter fill (gradient or dither, per-letter color, then texture), bevel edges on the fill.
//! A layer only ever overwrites the layers before it, so the order never depends on the options.

use crate::color::{Rgb, Rgba};
use crate::effects::{self, Mask, PlateCell};
use crate::error::Error;
use crate::font::Font;
use crate::layout::{layout, Layout, LayoutOptions};
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

struct Ink<'a> {
    paint: Paint,
    lay: &'a Layout<'a>,
    /// Letter number of every layout cell with ink, `u32::MAX` elsewhere.
    letters: Vec<u32>,
    total_letters: u32,
    /// Index into `lay.lines` for every layout row.
    row_line: Vec<usize>,
    texture: Option<(Texture, u32)>,
}

impl Ink<'_> {
    fn base_color(&self, x: u32, y: u32) -> Rgb {
        let letter = self.letters[y as usize * self.lay.width as usize + x as usize];
        match &self.paint {
            Paint::Solid(c) => *c,
            Paint::Letters(colors) => colors[letter as usize % colors.len()],
            Paint::PerLetter(stops) => stops[stop_index(letter, self.total_letters, stops.len())],
            Paint::Gradient {
                stops,
                direction,
                dither,
            } => {
                let line = &self.lay.lines[self.row_line[y as usize]];
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

pub fn render(spec: &Spec, font: &Font) -> Result<Rendered, Error> {
    spec.validate()?;
    let named = match &spec.fill.palette {
        Some(name) => Some(palette::resolve(name)?),
        None => None,
    };
    let fire = palette::find("fire").expect("fire palette exists");
    let stops: Vec<Rgb> = named
        .as_ref()
        .map_or_else(|| fire.stops.to_vec(), |p| p.stops.clone());

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
    let (cw, ch) = (canvas_w as u32, canvas_h as u32);
    let (ox, oy) = (m.left as u32, m.top as u32);

    let lw = lay.width as usize;
    let mut letters = vec![u32::MAX; lw * lay.height as usize];
    let mut letter = 0u32;
    let mut text = Mask::new(cw, ch);
    for line in &lay.lines {
        for p in &line.glyphs {
            for gy in 0..p.glyph.height() {
                for gx in 0..p.glyph.width() {
                    if p.glyph.get(gx, gy) {
                        text.set(ox + p.x + gx, oy + p.y + gy);
                        letters[(p.y + gy) as usize * lw + (p.x + gx) as usize] = letter;
                    }
                }
            }
            if p.glyph.has_ink() {
                letter += 1;
            }
        }
    }
    let mut row_line = vec![0usize; lay.height as usize];
    for (i, line) in lay.lines.iter().enumerate() {
        for row in line.y..line.y + line.height {
            row_line[row as usize] = i;
        }
    }
    let ink = Ink {
        paint,
        lay: &lay,
        letters,
        total_letters: letter,
        row_line,
        texture: spec.texture.clone().map(|t| (t, spec.seed)),
    };

    let outline = spec.outline.as_ref().map(|o| {
        (
            effects::dilate(&text, o.width, o.connectivity).minus(&text),
            o.color.unwrap_or(shadow_default),
        )
    });
    let mut silhouette = text.clone();
    if let Some((ring, _)) = &outline {
        silhouette.union_with(ring);
    }
    let extrusion = spec.extrude.as_ref().map(|e| {
        let steps = effects::extrusion_steps(&silhouette, e.dx, e.dy, e.length);
        let target = e.color.unwrap_or(shadow_default);
        let colors: Vec<Rgb> = (0..=e.length)
            .map(|k| effects::extrusion_color(darkest, target, e.step, k))
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

    let mut grid = Grid::new(cw, ch, spec.bg);
    if let Some(plate) = &spec.plate {
        let body_c = plate.color.unwrap_or(PLATE_COLOR);
        let border_c = plate.border_color.unwrap_or(body_color);
        for y in 0..ch {
            for x in 0..cw {
                match effects::plate_cell(x, y, cw, ch, plate.border, plate.cut_corners) {
                    Some(PlateCell::Border) => grid.set(x, y, border_c),
                    Some(PlateCell::Body) => grid.set(x, y, body_c),
                    None => {}
                }
            }
        }
    }
    if let Some(shadow) = &spec.shadow {
        let color = shadow.color.unwrap_or(shadow_default);
        let moved = body.shifted(i64::from(shadow.dx), i64::from(shadow.dy));
        for (x, y) in moved.cells() {
            grid.set(x, y, color);
        }
    }
    if let Some((steps, colors)) = &extrusion {
        for (i, k) in steps.iter().enumerate() {
            if *k > 0 {
                let (x, y) = ((i % cw as usize) as u32, (i / cw as usize) as u32);
                grid.set(x, y, colors[*k as usize]);
            }
        }
    }
    if let Some((ring, color)) = &outline {
        for (x, y) in ring.cells() {
            grid.set(x, y, *color);
        }
    }
    let bevel = spec.bevel.as_ref().map(|b| b.amount);
    for (x, y) in text.cells() {
        let color = ink.color_at(x - ox, y - oy);
        let color = match bevel {
            Some(amount) => effects::apply_bevel(color, effects::bevel_edge(&text, x, y), amount),
            None => color,
        };
        grid.set(x, y, color);
    }

    Ok(Rendered {
        grid,
        scale,
        title: spec.text.clone(),
        missing: lay.missing,
    })
}
