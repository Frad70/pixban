mod args;

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;
use pixban_core::{palette, render, Spec};

use args::{Cli, Command, Format};

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Some(Command::Fonts) => {
            for name in pixban_fonts::names() {
                println!("{name}");
            }
            return Ok(());
        }
        Some(Command::Palettes) => {
            for name in palette::names() {
                println!("{name}");
            }
            return Ok(());
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
    let font = pixban_fonts::get(&spec.font).ok_or_else(|| {
        format!(
            "unknown font '{}', available: {}",
            spec.font,
            pixban_fonts::names().collect::<Vec<_>>().join(", ")
        )
    })?;

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
