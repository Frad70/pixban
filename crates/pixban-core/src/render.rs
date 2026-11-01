use crate::color::{Rgb, Rgba};
use crate::error::Error;
use crate::font::Font;
use crate::layout::{layout, Layout, LayoutOptions};
use crate::limits::{MAX_GRID_SIDE, MAX_OUTPUT_PIXELS, MAX_OUTPUT_SIDE};
use crate::palette::{self, stop_index};
use crate::spec::{Direction, Spec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    width: u32,
    height: u32,
    cells: Vec<Rgba>,
    background: Option<Rgb>,
}

impl Grid {
    fn new(width: u32, height: u32, background: Option<Rgb>) -> Grid {
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

    fn set(&mut self, x: u32, y: u32, c: Rgb) {
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
    Gradient(&'static [Rgb], Direction),
}

impl Paint {
    fn color_at(&self, x: u32, y: u32, width: u32, height: u32) -> Rgb {
        match self {
            Paint::Solid(c) => *c,
            Paint::Gradient(stops, Direction::Vertical) => {
                stops[stop_index(y, height, stops.len())]
            }
            Paint::Gradient(stops, Direction::Horizontal) => {
                stops[stop_index(x, width, stops.len())]
            }
        }
    }
}

fn mask_of(layout: &Layout<'_>) -> Vec<bool> {
    let w = layout.width as usize;
    let mut mask = vec![false; w * layout.height as usize];
    for line in &layout.lines {
        for p in &line.glyphs {
            for gy in 0..p.glyph.height() {
                for gx in 0..p.glyph.width() {
                    if p.glyph.get(gx, gy) {
                        mask[(line.y + gy) as usize * w + (p.x + gx) as usize] = true;
                    }
                }
            }
        }
    }
    mask
}

pub fn render(spec: &Spec, font: &Font) -> Result<Rendered, Error> {
    spec.validate()?;
    let named = match &spec.fill.palette {
        Some(name) => Some(palette::find(name).ok_or_else(|| Error::UnknownPalette(name.clone()))?),
        None => None,
    };
    let default_palette = palette::find("fire").expect("fire palette exists");
    let gradient = named.unwrap_or(default_palette);
    let (paint, shadow_default) = match spec.fill.color {
        Some(c) => (
            Paint::Solid(c),
            named.map_or_else(|| c.darken(30), |p| p.shadow),
        ),
        None => (
            Paint::Gradient(gradient.stops, spec.fill.direction),
            gradient.shadow,
        ),
    };

    let scale = spec.scale.unwrap_or_else(|| font.default_scale());
    let lay = layout(font, &spec.text, &LayoutOptions::default())?;

    let (dx, dy) = spec.shadow.as_ref().map_or((0, 0), |s| (s.dx, s.dy));
    let canvas_w = u64::from(lay.width) + u64::from(dx.unsigned_abs());
    let canvas_h = u64::from(lay.height) + u64::from(dy.unsigned_abs());
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

    let mut grid = Grid::new(canvas_w as u32, canvas_h as u32, spec.bg);
    let mask = mask_of(&lay);
    let (fx, fy) = (dx.min(0).unsigned_abs(), dy.min(0).unsigned_abs());

    if let Some(shadow) = &spec.shadow {
        let color = shadow.color.unwrap_or(shadow_default);
        let (sx, sy) = (
            (i64::from(fx) + i64::from(dx)) as u32,
            (i64::from(fy) + i64::from(dy)) as u32,
        );
        for y in 0..lay.height {
            for x in 0..lay.width {
                if mask[(y * lay.width + x) as usize] {
                    grid.set(sx + x, sy + y, color);
                }
            }
        }
    }
    for y in 0..lay.height {
        for x in 0..lay.width {
            if mask[(y * lay.width + x) as usize] {
                grid.set(fx + x, fy + y, paint.color_at(x, y, lay.width, lay.height));
            }
        }
    }

    Ok(Rendered {
        grid,
        scale,
        title: spec.text.clone(),
        missing: lay.missing,
    })
}
