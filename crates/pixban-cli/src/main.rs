mod args;
mod term;

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use pixban_core::{
    lint, palette, render, render_animation, render_ansi, Font, Grid, Rgb, Rgba, Spec,
};

use args::{Cli, Command, Format, TermArgs};

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
    output: Option<&Path>,
    scale: Option<u32>,
    term: &TermArgs,
) -> Result<(), String> {
    let font = match font_file {
        Some(path) => {
            pixban_fonts::load_file(path).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => load_font(name)?,
    };
    let Some(output) = output else {
        let caps = term::caps(term, term::stdout_is_terminal());
        let columns = term::width().unwrap_or(80) as u32 / caps.columns_per_cell();
        let sheet = pixban_core::specimen_fit(&font, columns).map_err(|e| e.to_string())?;
        warn_specimen_missing(&sheet.missing);
        print!("{}", term::fit(&sheet.grid, caps)?);
        return Ok(());
    };
    let is_png = output
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    if !is_png {
        return Err("specimen writes PNG only, use an output file ending in .png".to_string());
    }
    let scale = scale.unwrap_or_else(|| font.default_scale());
    let sheet = pixban_core::specimen(&font, scale).map_err(|e| e.to_string())?;
    warn_specimen_missing(&sheet.missing);
    let bytes = sheet.to_png().map_err(|e| e.to_string())?;
    std::fs::write(output, &bytes)
        .map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    let (w, h) = sheet.pixel_size();
    println!("{}: {w}x{h} pixels", output.display());
    Ok(())
}

fn warn_specimen_missing(missing: &[char]) {
    if !missing.is_empty() {
        let list: String = missing.iter().collect();
        eprintln!("warning: specimen uses characters missing from the font: {list}");
    }
}

fn preview_fonts(term: &TermArgs) -> Result<(), String> {
    let caps = term::caps(term, term::stdout_is_terminal());
    let mut skipped = 0;
    for name in pixban_fonts::names() {
        let font = load_font(name)?;
        let spec = Spec {
            text: name.to_string(),
            font: name.to_string(),
            ..Spec::default()
        };
        let drawn = render(&spec, &font).map_err(|e| e.to_string())?;
        match term::fit(&drawn.grid, caps) {
            Ok(text) => print!("{text}"),
            Err(_) => {
                skipped += 1;
                println!("{name}");
            }
        }
        println!();
    }
    if skipped > 0 {
        eprintln!("warning: {skipped} previews are wider than the terminal and shown as names");
    }
    Ok(())
}

fn preview_palettes(names: Vec<&'static str>, caps: pixban_core::AnsiCaps) {
    const SWATCH: usize = 3;
    for name in names {
        let Some(p) = palette::find(name) else {
            continue;
        };
        let mut colors: Vec<Rgb> = p.stops.to_vec();
        colors.push(p.shadow);
        let width = colors.len() * SWATCH + 1;
        let mut cells = Vec::new();
        for _ in 0..2 {
            for (i, c) in colors.iter().enumerate() {
                if i + 1 == colors.len() {
                    cells.push(Rgba::TRANSPARENT);
                }
                cells.extend(std::iter::repeat_n(Rgba::from(*c), SWATCH));
            }
        }
        let strip = render_ansi(&Grid::from_cells(width as u32, 2, cells), caps);
        println!("{} {name}", strip.trim_end_matches('\n'));
    }
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

fn prepare(cli: &Cli) -> Result<(Spec, Font), String> {
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
    Ok((spec, font))
}

fn warn_missing(font: &Font, missing: &[char]) {
    if !missing.is_empty() {
        let shown: Vec<String> = missing.iter().take(8).map(|c| format!("'{c}'")).collect();
        let more = if missing.len() > 8 { ", ..." } else { "" };
        eprintln!(
            "warning: font '{}' has no glyph for {}{}; drawn as the fallback glyph",
            font.name(),
            shown.join(", "),
            more
        );
    }
}

fn snippet(kind: args::SnippetFor, raw: Vec<String>) -> Result<(), String> {
    let mut cli = Cli::try_parse_from(std::iter::once("pixban".to_string()).chain(raw.clone()))
        .map_err(|e| e.to_string())?;
    if cli.command.is_some() {
        return Err("snippet takes a logo's text and flags, not a subcommand".to_string());
    }
    cli.term.ansi = true;
    let (spec, font) = prepare(&cli)?;
    let grid = if spec.anim.is_some() {
        render_animation(&spec, &font)
            .map_err(|e| e.to_string())?
            .frames()[0]
            .clone()
    } else {
        render(&spec, &font).map_err(|e| e.to_string())?.grid
    };
    let caps = term::caps(&cli.term, false);
    let text = render_ansi(&grid, caps);
    print!("{}", term::snippet(kind, &raw, caps.color, &text));
    Ok(())
}

fn ansi_output(cli: &Cli, spec: &Spec, font: &Font) -> Result<(), String> {
    let tty = term::stdout_is_terminal();
    let caps = term::caps(&cli.term, tty);
    let (grids, frame_ms, missing) = if spec.anim.is_some() {
        let anim = render_animation(spec, font).map_err(|e| e.to_string())?;
        if let Some(dir) = &cli.frames_dir {
            write_frames(&anim, dir)?;
        }
        (
            anim.frames().to_vec(),
            anim.frame_ms(),
            anim.missing().to_vec(),
        )
    } else {
        let rendered = render(spec, font).map_err(|e| e.to_string())?;
        (vec![rendered.grid], 0, rendered.missing)
    };
    warn_missing(font, &missing);
    let text = term::fit(&grids[0], caps)?;
    if tty && grids.len() > 1 {
        return term::play(&grids, caps, frame_ms, cli.loops);
    }
    std::io::stdout()
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Some(Command::Fonts { preview, term }) => {
            if preview {
                return preview_fonts(&term);
            }
            for name in pixban_fonts::names() {
                println!("{name}");
            }
            return Ok(());
        }
        Some(Command::Palettes { all, preview, term }) => {
            let caps = term::caps(&term, term::stdout_is_terminal());
            let show = |names: Vec<&'static str>| {
                if preview {
                    preview_palettes(names, caps);
                } else {
                    names.iter().for_each(|n| println!("{n}"));
                }
            };
            show(palette::names().collect());
            if all {
                println!();
                println!("extended:");
                show(palette::extended_names().collect());
            }
            return Ok(());
        }
        Some(Command::LintFont { target }) => return lint_font(&target),
        Some(Command::Specimen {
            font,
            font_file,
            output,
            scale,
            term,
        }) => {
            return specimen(
                font.as_deref().unwrap_or("block"),
                font_file.as_deref(),
                output.as_deref(),
                scale,
                &term,
            )
        }
        Some(Command::Snippet { kind, args }) => return snippet(kind, args),
        None => {}
    }

    let (spec, font) = prepare(&cli)?;

    if cli.dump_spec {
        let out = toml::to_string(&spec).map_err(|e| e.to_string())?;
        print!("{out}");
        return Ok(());
    }

    let stdout_default = cli.output.as_ref().is_none_or(|p| p.as_os_str() == "-");
    if cli.term.ansi && cli.format.is_some() {
        return Err("--ansi cannot be combined with --format".to_string());
    }
    if cli.term.ansi && !stdout_default {
        return Err("--ansi cannot be combined with -o; redirect stdout instead".to_string());
    }
    if cli.term.ansi
        || (stdout_default
            && cli.format.is_none()
            && cli.frames_dir.is_none()
            && term::stdout_is_terminal())
    {
        return ansi_output(&cli, &spec, &font);
    }

    let animated = spec.anim.is_some();
    let format = cli.format(animated)?;
    let (bytes, missing) = if animated {
        let anim = render_animation(&spec, &font).map_err(|e| e.to_string())?;
        if let Some(dir) = &cli.frames_dir {
            write_frames(&anim, dir)?;
        }
        let bytes = match format {
            Format::Svg => anim.to_svg().into_bytes(),
            Format::Gif => {
                if anim.gif_frame_ms() != anim.frame_ms() {
                    eprintln!(
                        "warning: frame time {} ms is stored as {} ms in a GIF, which counts in steps of 10 ms and plays nothing faster than 20 ms",
                        anim.frame_ms(),
                        anim.gif_frame_ms()
                    );
                }
                anim.to_gif().map_err(|e| e.to_string())?
            }
            Format::Png | Format::Apng => anim.to_apng().map_err(|e| e.to_string())?,
        };
        (bytes, anim.missing().to_vec())
    } else {
        let rendered = render(&spec, &font).map_err(|e| e.to_string())?;
        let bytes = match format {
            Format::Svg => rendered.to_svg().into_bytes(),
            _ => rendered.to_png().map_err(|e| e.to_string())?,
        };
        (bytes, rendered.missing)
    };
    warn_missing(&font, &missing);

    if cli.frames_dir.is_some() && cli.output.is_none() && cli.format.is_none() {
        return Ok(());
    }
    match &cli.output {
        Some(path) if path.as_os_str() != "-" => std::fs::write(path, &bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display())),
        _ => {
            if format != Format::Svg && std::io::stdout().is_terminal() {
                return Err(
                    "refusing to write binary image data to a terminal; use -o FILE".to_string(),
                );
            }
            std::io::stdout()
                .write_all(&bytes)
                .map_err(|e| e.to_string())
        }
    }
}

fn write_frames(anim: &pixban_core::Animated, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let digits = anim.frames().len().to_string().len().max(3);
    for i in 0..anim.frames().len() {
        let path = dir.join(format!("frame-{i:0digits$}.png"));
        let bytes = anim.frame_png(i).map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    let (w, h) = anim.pixel_size();
    println!(
        "{}: {} frames of {w}x{h} pixels",
        dir.display(),
        anim.frames().len()
    );
    Ok(())
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
