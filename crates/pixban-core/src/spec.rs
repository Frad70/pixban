use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::error::Error;
use crate::layout::Align;
use crate::limits::{
    MAX_EXTRUDE, MAX_LETTER_COLORS, MAX_OUTLINE_WIDTH, MAX_PADDING, MAX_PLATE_BORDER, MAX_SCALE,
    MAX_SHADOW_OFFSET, MAX_TEXTURE_PERIOD,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
    Diagonal,
}

/// Ordered dithering between the two neighbouring palette stops of a gradient.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dither {
    #[default]
    None,
    Bayer2,
    Bayer4,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fill {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
    #[serde(skip_serializing_if = "is_default")]
    pub dither: Dither,
    /// One palette stop per letter, spread over the whole text.
    #[serde(skip_serializing_if = "is_default")]
    pub per_letter: bool,
    /// Explicit colors cycled over the letters; wins over every other fill.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub letter_colors: Vec<Rgb>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Shadow {
    pub dx: i32,
    pub dy: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
}

impl Default for Shadow {
    fn default() -> Self {
        Shadow {
            dx: 1,
            dy: 1,
            color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Connectivity {
    #[serde(rename = "4")]
    Four,
    #[default]
    #[serde(rename = "8")]
    Eight,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Outline {
    pub width: u32,
    pub connectivity: Connectivity,
    /// Defaults to the palette's shadow color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
}

impl Default for Outline {
    fn default() -> Self {
        Outline {
            width: 1,
            connectivity: Connectivity::Eight,
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Extrude {
    /// Number of cells the extrusion reaches.
    pub length: u32,
    /// Direction of the extrusion, each of dx and dy from -1 to 1.
    pub dx: i32,
    pub dy: i32,
    /// Percent of the way toward the target color that every step adds, counted from the
    /// darkest fill color.
    pub step: u32,
    /// Color the last steps approach; defaults to the palette's shadow color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
}

impl Default for Extrude {
    fn default() -> Self {
        Extrude {
            length: 4,
            dx: 1,
            dy: 1,
            step: 30,
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bevel {
    /// Percent toward white on the top-left edge and toward black on the bottom-right edge.
    pub amount: u32,
}

impl Default for Bevel {
    fn default() -> Self {
        Bevel { amount: 25 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextureKind {
    #[default]
    Scanlines,
    Checker,
    Noise,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Texture {
    pub kind: TextureKind,
    /// Percent the fill color moves toward black (or white, for noise).
    pub strength: u32,
    /// Scanlines only: every `period`-th row is darker.
    pub period: u32,
}

impl Default for Texture {
    fn default() -> Self {
        Texture {
            kind: TextureKind::Scanlines,
            strength: 25,
            period: 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Plate {
    /// Defaults to #0d1117.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
    pub border: u32,
    /// Defaults to the middle color of the fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Rgb>,
    /// Drops the four corner cells of the plate.
    pub cut_corners: bool,
}

impl Default for Plate {
    fn default() -> Self {
        Plate {
            color: None,
            border: 1,
            border_color: None,
            cut_corners: false,
        }
    }
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn is_one(v: &u32) -> bool {
    *v == 1
}

/// Every setting of a rendering. Scalars come before tables so the TOML form is valid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Spec {
    pub text: String,
    pub font: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<Rgb>,
    /// Seeds noise; the same settings always give the same bytes.
    #[serde(skip_serializing_if = "is_default")]
    pub seed: u32,
    /// Empty cells around the artwork; 2 when a plate is on and nothing is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<u32>,
    #[serde(skip_serializing_if = "is_default")]
    pub align: Align,
    #[serde(skip_serializing_if = "is_one")]
    pub line_gap: u32,
    /// Letter spacing in cells, replaces the font's own; may be negative down to -2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<i32>,
    pub fill: Fill,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<Outline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extrude: Option<Extrude>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bevel: Option<Bevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<Texture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plate: Option<Plate>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            text: String::new(),
            font: "block".to_string(),
            scale: None,
            bg: None,
            seed: 0,
            padding: None,
            align: Align::Left,
            line_gap: 1,
            spacing: None,
            fill: Fill::default(),
            shadow: None,
            outline: None,
            extrude: None,
            bevel: None,
            texture: None,
            plate: None,
        }
    }
}

fn check(what: &'static str, value: u32, min: u32, max: u32) -> Result<(), Error> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(Error::out_of_range(
            what,
            i64::from(value),
            i64::from(min),
            i64::from(max),
        ))
    }
}

impl Spec {
    pub fn validate(&self) -> Result<(), Error> {
        if let Some(scale) = self.scale {
            if !(1..=MAX_SCALE).contains(&scale) {
                return Err(Error::InvalidScale {
                    scale,
                    max: MAX_SCALE,
                });
            }
        }
        if let Some(s) = &self.shadow {
            let ok = |v: i32| (-MAX_SHADOW_OFFSET..=MAX_SHADOW_OFFSET).contains(&v);
            if !ok(s.dx) || !ok(s.dy) {
                return Err(Error::InvalidShadowOffset {
                    dx: s.dx,
                    dy: s.dy,
                    max: MAX_SHADOW_OFFSET,
                });
            }
        }
        if let Some(p) = self.padding {
            check("padding", p, 0, MAX_PADDING)?;
        }
        if self.fill.letter_colors.len() > MAX_LETTER_COLORS {
            return Err(Error::out_of_range(
                "number of letter colors",
                self.fill.letter_colors.len() as i64,
                1,
                MAX_LETTER_COLORS as i64,
            ));
        }
        if let Some(o) = &self.outline {
            check("outline width", o.width, 1, MAX_OUTLINE_WIDTH)?;
        }
        if let Some(e) = &self.extrude {
            check("extrusion length", e.length, 1, MAX_EXTRUDE)?;
            check("extrusion step", e.step, 1, 100)?;
            for (what, v) in [("extrusion dx", e.dx), ("extrusion dy", e.dy)] {
                if !(-1..=1).contains(&v) {
                    return Err(Error::out_of_range(what, i64::from(v), -1, 1));
                }
            }
            if e.dx == 0 && e.dy == 0 {
                return Err(Error::InvalidSetting(
                    "extrusion direction 0,0 points nowhere, set dx or dy to -1 or 1".to_string(),
                ));
            }
        }
        if let Some(b) = &self.bevel {
            check("bevel amount", b.amount, 1, 100)?;
        }
        if let Some(t) = &self.texture {
            check("texture strength", t.strength, 1, 100)?;
            check("texture period", t.period, 2, MAX_TEXTURE_PERIOD)?;
        }
        if let Some(p) = &self.plate {
            check("plate border", p.border, 0, MAX_PLATE_BORDER)?;
        }
        Ok(())
    }
}
