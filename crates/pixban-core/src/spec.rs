use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::error::Error;
use crate::limits::{MAX_SCALE, MAX_SHADOW_OFFSET};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fill {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
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
    pub fill: Fill,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            text: String::new(),
            font: "block".to_string(),
            scale: None,
            bg: None,
            fill: Fill::default(),
            shadow: None,
        }
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
        Ok(())
    }
}
