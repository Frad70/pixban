mod anim;
mod apng;
mod color;
mod effects;
mod error;
mod fire;
pub mod font;
mod gif;
mod layout;
pub mod limits;
mod lint;
pub mod palette;
mod png;
mod render;
mod spec;
mod specimen;
mod svg;

pub use anim::{render_animation, render_frame, Animated};
pub use color::{ParseColorError, Rgb, Rgba};
pub use error::{Error, ParseError};
pub use font::{CaseMode, Font, Glyph};
pub use layout::{layout, Align, Layout, LayoutLine, LayoutOptions, PlacedGlyph};
pub use lint::{lint, Coverage, Report, Script};
pub use palette::{Palette, Resolved};
pub use render::{render, Grid, Rendered};
pub use spec::{
    Anim, AnimKind, Bevel, Connectivity, Direction, Dither, Extrude, Fill, Outline, Plate, Shadow,
    Spec, Texture, TextureKind,
};
pub use specimen::specimen;
