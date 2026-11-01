use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use pixban_core::{
    Align, Bevel, Connectivity, Direction, Dither, Extrude, Outline, Plate, Rgb, Shadow, Spec,
    Texture, TextureKind,
};

#[derive(Parser)]
#[command(
    name = "pixban",
    version,
    about = "Pixel-art logos and banners from text",
    after_help = "Examples:\n  pixban \"brazier\" -f block -p fire --shadow 1,1 -o logo.svg\n  pixban \"Pixel\\nBanner\" --align center --outline 1 --extrude 4 -o banner.png\n  pixban \"hi\" -p \"#ff0066,#ffcc00/#330011\" --plate --bevel 30"
)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Text to render; a literal \n starts a new line
    pub text: Option<String>,

    /// Font name (see `pixban fonts`)
    #[arg(short, long, conflicts_with = "font_file")]
    pub font: Option<String>,

    /// Font file: .pxf, BDF or PCF, optionally gzipped; replaces -f
    #[arg(long, value_name = "FILE")]
    pub font_file: Option<PathBuf>,

    /// Palette name (see `pixban palettes --all`) or colors: "#hex,#hex,..." with an optional
    /// "/#shadow" at the end
    #[arg(short, long, help_heading = "Fill")]
    pub palette: Option<String>,

    /// Direction of the palette gradient
    #[arg(long, value_enum, help_heading = "Fill")]
    pub gradient: Option<GradientArg>,

    /// Ordered dithering between neighbouring palette stops
    #[arg(long, value_enum, help_heading = "Fill")]
    pub dither: Option<DitherArg>,

    /// Solid fill color, #rrggbb
    #[arg(long, value_name = "HEX", help_heading = "Fill")]
    pub color: Option<Rgb>,

    /// One palette stop per letter, spread over the whole text
    #[arg(long, help_heading = "Fill")]
    pub per_letter: bool,

    /// Colors cycled over the letters: "#hex,#hex,..." (up to 64)
    #[arg(
        long,
        value_name = "HEXES",
        value_delimiter = ',',
        help_heading = "Fill"
    )]
    pub letter_colors: Vec<Rgb>,

    /// Hard shadow offset in cells
    #[arg(long, value_name = "DX,DY", value_parser = parse_offset, allow_hyphen_values = true, help_heading = "Effects")]
    pub shadow: Option<(i32, i32)>,

    /// Shadow color, #rrggbb (defaults to the palette's shadow)
    #[arg(long, value_name = "HEX", help_heading = "Effects")]
    pub shadow_color: Option<Rgb>,

    /// Outline width in cells, 1 to 4
    #[arg(long, value_name = "N", help_heading = "Effects")]
    pub outline: Option<u32>,

    /// Outline color, #rrggbb (defaults to the palette's shadow)
    #[arg(long, value_name = "HEX", help_heading = "Effects")]
    pub outline_color: Option<Rgb>,

    /// Outline neighbourhood: 4 gives a diamond, 8 a square (default 8)
    #[arg(long, value_enum, value_name = "4|8", help_heading = "Effects")]
    pub outline_conn: Option<ConnArg>,

    /// Extrusion length in cells, 1 to 32
    #[arg(long, value_name = "N", help_heading = "Effects")]
    pub extrude: Option<u32>,

    /// Extrusion direction, each from -1 to 1 (default 1,1)
    #[arg(long, value_name = "DX,DY", value_parser = parse_offset, allow_hyphen_values = true, help_heading = "Effects")]
    pub extrude_dir: Option<(i32, i32)>,

    /// Color the extrusion darkens toward (defaults to the palette's shadow)
    #[arg(long, value_name = "HEX", help_heading = "Effects")]
    pub extrude_color: Option<Rgb>,

    /// Percent each extrusion step moves toward that color, 1 to 100 (default 30)
    #[arg(long, value_name = "PCT", help_heading = "Effects")]
    pub extrude_step: Option<u32>,

    /// Bevel strength in percent, 1 to 100: light top-left edge, dark bottom-right edge
    #[arg(long, value_name = "PCT", help_heading = "Effects")]
    pub bevel: Option<u32>,

    /// Texture on the fill
    #[arg(long, value_enum, help_heading = "Effects")]
    pub texture: Option<TextureArg>,

    /// Texture strength in percent, 1 to 100 (default 25)
    #[arg(long, value_name = "PCT", help_heading = "Effects")]
    pub texture_strength: Option<u32>,

    /// Scanlines: every Nth row is darker, 2 to 16 (default 2)
    #[arg(long, value_name = "N", help_heading = "Effects")]
    pub texture_period: Option<u32>,

    /// Seed for noise; the same seed always gives the same file
    #[arg(long, value_name = "N", help_heading = "Effects")]
    pub seed: Option<u32>,

    /// Put the text on a pixel plate
    #[arg(long, help_heading = "Plate and background")]
    pub plate: bool,

    /// Plate color, #rrggbb (default #0d1117)
    #[arg(long, value_name = "HEX", help_heading = "Plate and background")]
    pub plate_color: Option<Rgb>,

    /// Plate border width in cells, 0 to 8 (default 1)
    #[arg(long, value_name = "N", help_heading = "Plate and background")]
    pub plate_border: Option<u32>,

    /// Plate border color, #rrggbb (defaults to the middle color of the fill)
    #[arg(long, value_name = "HEX", help_heading = "Plate and background")]
    pub plate_border_color: Option<Rgb>,

    /// Cut the four corner cells off the plate
    #[arg(long, help_heading = "Plate and background")]
    pub cut_corners: bool,

    /// Empty cells around the artwork, 0 to 64 (2 with a plate)
    #[arg(long, value_name = "N", help_heading = "Plate and background")]
    pub padding: Option<u32>,

    /// Solid background color, #rrggbb (default transparent)
    #[arg(long, value_name = "HEX", help_heading = "Plate and background")]
    pub bg: Option<Rgb>,

    /// Alignment of lines in multi-line text
    #[arg(long, value_enum, help_heading = "Layout")]
    pub align: Option<AlignArg>,

    /// Empty rows between lines, 0 to 32 (default 1)
    #[arg(long, value_name = "N", help_heading = "Layout")]
    pub line_gap: Option<u32>,

    /// Letter spacing in cells, -2 to 32; replaces the font's own
    #[arg(
        long,
        value_name = "N",
        allow_hyphen_values = true,
        help_heading = "Layout"
    )]
    pub spacing: Option<i32>,

    /// Pixels per cell, an integer (default depends on the font)
    #[arg(long, value_name = "N", help_heading = "Layout")]
    pub scale: Option<u32>,

    /// Output file; the extension picks the format. Default is SVG on stdout
    #[arg(short, long, value_name = "FILE", help_heading = "Output")]
    pub output: Option<PathBuf>,

    /// Output format, overrides the file extension
    #[arg(long, value_enum, help_heading = "Output")]
    pub format: Option<FormatArg>,

    /// Load settings from a TOML file; flags override it
    #[arg(long, value_name = "FILE", help_heading = "Output")]
    pub spec: Option<PathBuf>,

    /// Print the resulting settings as TOML and exit
    #[arg(long, help_heading = "Output")]
    pub dump_spec: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// List built-in fonts
    Fonts,
    /// List built-in palettes
    Palettes {
        /// Also list the extended group: hardware and themed ramps
        #[arg(long)]
        all: bool,
    },
    /// Check a font file or a built-in font name and print glyph coverage
    LintFont {
        /// Built-in font name or path to a .pxf file
        target: String,
    },
    /// Render a specimen sheet of a font as PNG
    Specimen {
        /// Font name (see `pixban fonts`), default block
        #[arg(short, long, conflicts_with = "font_file")]
        font: Option<String>,
        /// Font file: .pxf, BDF or PCF, optionally gzipped
        #[arg(long, value_name = "FILE")]
        font_file: Option<PathBuf>,
        /// Output PNG file
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
        /// Pixels per cell (default is the font's own scale)
        #[arg(long, value_name = "N")]
        scale: Option<u32>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum GradientArg {
    Vertical,
    Horizontal,
    Diagonal,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum DitherArg {
    None,
    Bayer2,
    Bayer4,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ConnArg {
    #[value(name = "4")]
    Four,
    #[value(name = "8")]
    Eight,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum TextureArg {
    Scanlines,
    Checker,
    Noise,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AlignArg {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Svg,
    Png,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Svg,
    Png,
}

fn parse_offset(s: &str) -> Result<(i32, i32), String> {
    let (dx, dy) = s
        .split_once(',')
        .ok_or_else(|| "expected DX,DY, for example 1,1".to_string())?;
    let num = |v: &str| {
        v.trim()
            .parse::<i32>()
            .map_err(|_| format!("'{v}' is not an integer"))
    };
    Ok((num(dx)?, num(dy)?))
}

fn need<'a, T>(slot: &'a mut Option<T>, flag: &str, add: &str) -> Result<&'a mut T, String> {
    slot.as_mut().ok_or_else(|| format!("{flag} needs {add}"))
}

impl Cli {
    pub fn apply(&self, spec: &mut Spec) -> Result<(), String> {
        if let Some(text) = &self.text {
            spec.text = text.replace("\\n", "\n");
        }
        if let Some(font) = &self.font {
            spec.font = font.clone();
        }
        if let Some(palette) = &self.palette {
            spec.fill.palette = Some(palette.clone());
        }
        if let Some(g) = self.gradient {
            spec.fill.direction = match g {
                GradientArg::Vertical => Direction::Vertical,
                GradientArg::Horizontal => Direction::Horizontal,
                GradientArg::Diagonal => Direction::Diagonal,
            };
        }
        if let Some(d) = self.dither {
            spec.fill.dither = match d {
                DitherArg::None => Dither::None,
                DitherArg::Bayer2 => Dither::Bayer2,
                DitherArg::Bayer4 => Dither::Bayer4,
            };
        }
        if let Some(color) = self.color {
            spec.fill.color = Some(color);
        }
        if self.per_letter {
            spec.fill.per_letter = true;
        }
        if !self.letter_colors.is_empty() {
            spec.fill.letter_colors = self.letter_colors.clone();
        }

        if let Some((dx, dy)) = self.shadow {
            let color = spec.shadow.as_ref().and_then(|s| s.color);
            spec.shadow = Some(Shadow { dx, dy, color });
        }
        if let Some(color) = self.shadow_color {
            need(
                &mut spec.shadow,
                "--shadow-color",
                "a shadow, add --shadow DX,DY",
            )?
            .color = Some(color);
        }

        if let Some(width) = self.outline {
            spec.outline.get_or_insert_with(Outline::default).width = width;
        }
        let outline_hint = "an outline, add --outline N";
        if let Some(color) = self.outline_color {
            need(&mut spec.outline, "--outline-color", outline_hint)?.color = Some(color);
        }
        if let Some(conn) = self.outline_conn {
            need(&mut spec.outline, "--outline-conn", outline_hint)?.connectivity = match conn {
                ConnArg::Four => Connectivity::Four,
                ConnArg::Eight => Connectivity::Eight,
            };
        }

        if let Some(length) = self.extrude {
            spec.extrude.get_or_insert_with(Extrude::default).length = length;
        }
        let extrude_hint = "an extrusion, add --extrude N";
        if let Some((dx, dy)) = self.extrude_dir {
            let e = need(&mut spec.extrude, "--extrude-dir", extrude_hint)?;
            e.dx = dx;
            e.dy = dy;
        }
        if let Some(color) = self.extrude_color {
            need(&mut spec.extrude, "--extrude-color", extrude_hint)?.color = Some(color);
        }
        if let Some(step) = self.extrude_step {
            need(&mut spec.extrude, "--extrude-step", extrude_hint)?.step = step;
        }

        if let Some(amount) = self.bevel {
            spec.bevel = Some(Bevel { amount });
        }

        if let Some(kind) = self.texture {
            spec.texture.get_or_insert_with(Texture::default).kind = match kind {
                TextureArg::Scanlines => TextureKind::Scanlines,
                TextureArg::Checker => TextureKind::Checker,
                TextureArg::Noise => TextureKind::Noise,
            };
        }
        let texture_hint = "a texture, add --texture scanlines|checker|noise";
        if let Some(strength) = self.texture_strength {
            need(&mut spec.texture, "--texture-strength", texture_hint)?.strength = strength;
        }
        if let Some(period) = self.texture_period {
            need(&mut spec.texture, "--texture-period", texture_hint)?.period = period;
        }
        if let Some(seed) = self.seed {
            spec.seed = seed;
        }

        if self.plate {
            spec.plate.get_or_insert_with(Plate::default);
        }
        let plate_hint = "a plate, add --plate";
        if let Some(color) = self.plate_color {
            need(&mut spec.plate, "--plate-color", plate_hint)?.color = Some(color);
        }
        if let Some(border) = self.plate_border {
            need(&mut spec.plate, "--plate-border", plate_hint)?.border = border;
        }
        if let Some(color) = self.plate_border_color {
            need(&mut spec.plate, "--plate-border-color", plate_hint)?.border_color = Some(color);
        }
        if self.cut_corners {
            need(&mut spec.plate, "--cut-corners", plate_hint)?.cut_corners = true;
        }
        if let Some(padding) = self.padding {
            spec.padding = Some(padding);
        }
        if let Some(bg) = self.bg {
            spec.bg = Some(bg);
        }

        if let Some(a) = self.align {
            spec.align = match a {
                AlignArg::Left => Align::Left,
                AlignArg::Center => Align::Center,
                AlignArg::Right => Align::Right,
            };
        }
        if let Some(gap) = self.line_gap {
            spec.line_gap = gap;
        }
        if let Some(spacing) = self.spacing {
            spec.spacing = Some(spacing);
        }
        if let Some(scale) = self.scale {
            spec.scale = Some(scale);
        }
        Ok(())
    }

    pub fn format(&self) -> Result<Format, String> {
        if let Some(f) = self.format {
            return Ok(match f {
                FormatArg::Svg => Format::Svg,
                FormatArg::Png => Format::Png,
            });
        }
        match &self.output {
            Some(path) if path.as_os_str() != "-" => {
                match path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("svg") => Ok(Format::Svg),
                    Some("png") => Ok(Format::Png),
                    _ => Err(format!(
                        "cannot tell the format from '{}', use a .svg or .png name or --format",
                        path.display()
                    )),
                }
            }
            _ => Ok(Format::Svg),
        }
    }
}
