use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use pixban_core::{Direction, Rgb, Shadow, Spec};

#[derive(Parser)]
#[command(
    name = "pixban",
    version,
    about = "Pixel-art logos and banners from text"
)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Text to render
    pub text: Option<String>,

    /// Font name (see `pixban fonts`)
    #[arg(short, long)]
    pub font: Option<String>,

    /// Palette name (see `pixban palettes`)
    #[arg(short, long)]
    pub palette: Option<String>,

    /// Direction of the palette gradient
    #[arg(long, value_enum)]
    pub gradient: Option<GradientArg>,

    /// Solid fill color, #rrggbb
    #[arg(long, value_name = "HEX")]
    pub color: Option<Rgb>,

    /// Hard shadow offset in cells
    #[arg(long, value_name = "DX,DY", value_parser = parse_offset, allow_hyphen_values = true)]
    pub shadow: Option<(i32, i32)>,

    /// Shadow color, #rrggbb (defaults to the palette's shadow)
    #[arg(long, value_name = "HEX")]
    pub shadow_color: Option<Rgb>,

    /// Pixels per cell, an integer (default depends on the font)
    #[arg(long, value_name = "N")]
    pub scale: Option<u32>,

    /// Solid background color, #rrggbb (default transparent)
    #[arg(long, value_name = "HEX")]
    pub bg: Option<Rgb>,

    /// Output file; the extension picks the format. Default is SVG on stdout
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Output format, overrides the file extension
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,

    /// Load settings from a TOML file; flags override it
    #[arg(long, value_name = "FILE")]
    pub spec: Option<PathBuf>,

    /// Print the resulting settings as TOML and exit
    #[arg(long)]
    pub dump_spec: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// List built-in fonts
    Fonts,
    /// List built-in palettes
    Palettes,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum GradientArg {
    Vertical,
    Horizontal,
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

impl Cli {
    pub fn apply(&self, spec: &mut Spec) -> Result<(), String> {
        if let Some(text) = &self.text {
            spec.text = text.clone();
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
            };
        }
        if let Some(color) = self.color {
            spec.fill.color = Some(color);
        }
        if let Some((dx, dy)) = self.shadow {
            let color = spec.shadow.as_ref().and_then(|s| s.color);
            spec.shadow = Some(Shadow { dx, dy, color });
        }
        if let Some(color) = self.shadow_color {
            match spec.shadow.as_mut() {
                Some(s) => s.color = Some(color),
                None => return Err("--shadow-color needs a shadow, add --shadow DX,DY".to_string()),
            }
        }
        if let Some(scale) = self.scale {
            spec.scale = Some(scale);
        }
        if let Some(bg) = self.bg {
            spec.bg = Some(bg);
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
