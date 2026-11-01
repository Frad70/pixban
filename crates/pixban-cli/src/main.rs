mod args;

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use pixban_core::{lint, palette, render, Font, Spec};

use args::{Cli, Command, Format};

fn load_font(target: &str) -> Result<Font, String> {
    if let Some(font) = pixban_fonts::get(target) {
        return Ok(font);
    }
    let path = Path::new(target);
    if path.is_file() {
        return pixban_fonts::load_file(path).map_err(|e| format!("{}: {e}", path.display()));
    }
    Err(format!(
        "unknown font '{target}', available: {} (or a path to a font file)",
        pixban_fonts::names().collect::<Vec<_>>().join(", ")
    ))
}

fn specimen(
    name: &str,
    font_file: Option<&Path>,
    output: &Path,
    scale: Option<u32>,
) -> Result<(), String> {
    let is_png = output
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    if !is_png {
        return Err("specimen writes PNG only, use an output file ending in .png".to_string());
    }
    let font = match font_file {
        Some(path) => {
            pixban_fonts::load_file(path).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => load_font(name)?,
    };
    let scale = scale.unwrap_or_else(|| font.default_scale());
    let sheet = pixban_core::specimen(&font, scale).map_err(|e| e.to_string())?;
    if !sheet.missing.is_empty() {
        let list: String = sheet.missing.iter().collect();
        eprintln!("warning: specimen uses characters missing from the font: {list}");
    }
    let bytes = sheet.to_png().map_err(|e| e.to_string())?;
    std::fs::write(output, &bytes)
        .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    let (w, h) = sheet.pixel_size();
    println!("{}: {w}x{h} pixels", output.display());
    Ok(())
}

fn lint_font(target: &str) -> Result<(), String> {
    let font = load_font(target)?;
    let report = lint(&font);
    println!(
        "{}: {} glyphs, band {} rows, max width {}",
        font.name(),
        font.chars().len(),
        font.height(),
        font.max_width()
    );
    for e in &report.errors {
        println!("error: {e}");
    }
    for w in &report.warnings {
        println!("warning: {w}");
    }
    println!("{:<16}{:>10}", "script", "covered");
    for c in &report.coverage {
        let shown = if c.expected == 0 {
            "-".to_string()
        } else {
            format!("{}/{}", c.covered, c.expected)
        };
        println!("{:<16}{:>10}", c.script.label(), shown);
        if !c.missing.is_empty() {
            let list: String = c.missing.iter().collect();
            println!("  missing: {list}");
        }
    }
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{} lint errors", report.errors.len()))
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Some(Command::Fonts) => {
            for name in pixban_fonts::names() {
                println!("{name}");
            }
            return Ok(());
        }
        Some(Command::Palettes { all }) => {
            for name in palette::names() {
                println!("{name}");
            }
            if all {
                println!();
                println!("extended:");
                for name in palette::extended_names() {
                    println!("{name}");
                }
            }
            return Ok(());
        }
        Some(Command::LintFont { target }) => return lint_font(&target),
        Some(Command::Specimen {
            font,
            font_file,
            output,
            scale,
        }) => {
            return specimen(
                font.as_deref().unwrap_or("block"),
                font_file.as_deref(),
                &output,
                scale,
            )
        }
        None => {}
    }

    let mut spec = match &cli.spec {
        Some(path) => {
            let src = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            toml::from_str::<Spec>(&src).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => Spec::default(),
    };
    cli.apply(&mut spec)?;

    if spec.text.is_empty() {
        return Err("no text given; pass TEXT or set it in a spec file".to_string());
    }
    if cli.dump_spec && cli.font_file.is_some() {
        return Err(
            "--dump-spec cannot describe --font-file, use -f with a built-in font".to_string(),
        );
    }
    let font = match &cli.font_file {
        Some(path) => {
            pixban_fonts::load_file(path).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => pixban_fonts::get(&spec.font).ok_or_else(|| {
            format!(
                "unknown font '{}', available: {}",
                spec.font,
                pixban_fonts::names().collect::<Vec<_>>().join(", ")
            )
        })?,
    };

    if cli.dump_spec {
        let out = toml::to_string(&spec).map_err(|e| e.to_string())?;
        print!("{out}");
        return Ok(());
    }

    let rendered = render(&spec, &font).map_err(|e| e.to_string())?;
    if !rendered.missing.is_empty() {
        let shown: Vec<String> = rendered
            .missing
            .iter()
            .take(8)
            .map(|c| format!("'{c}'"))
            .collect();
        let more = if rendered.missing.len() > 8 {
            ", ..."
        } else {
            ""
        };
        eprintln!(
            "warning: font '{}' has no glyph for {}{}; drawn as the fallback glyph",
            font.name(),
            shown.join(", "),
            more
        );
    }

    let format = cli.format()?;
    let bytes = match format {
        Format::Svg => rendered.to_svg().into_bytes(),
        Format::Png => rendered.to_png().map_err(|e| e.to_string())?,
    };
    match &cli.output {
        Some(path) if path.as_os_str() != "-" => std::fs::write(path, &bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display())),
        _ => {
            if format == Format::Png && std::io::stdout().is_terminal() {
                return Err("refusing to write PNG data to a terminal; use -o FILE".to_string());
            }
            std::io::stdout()
                .write_all(&bytes)
                .map_err(|e| e.to_string())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}
