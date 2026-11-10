use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pixban_core::{render_ansi, AnsiCaps, Blocks, ColorMode, Grid};

use crate::args::{BlocksArg, ColorModeArg, SnippetFor, TermArgs};

static STOP: AtomicBool = AtomicBool::new(false);

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

fn detect() -> ColorMode {
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    if colorterm == "truecolor" || colorterm == "24bit" {
        ColorMode::TrueColor
    } else if term.contains("256color") {
        ColorMode::Ansi256
    } else if !term.is_empty() && term != "dumb" {
        ColorMode::Ansi16
    } else {
        ColorMode::None
    }
}

/// `tty` says the text goes to a terminal; piped text gets no color unless `--ansi` asks for it.
pub fn caps(t: &TermArgs, tty: bool) -> AnsiCaps {
    let color = match t.color_mode {
        ColorModeArg::Truecolor => ColorMode::TrueColor,
        ColorModeArg::Ansi256 => ColorMode::Ansi256,
        ColorModeArg::Ansi16 => ColorMode::Ansi16,
        ColorModeArg::None => ColorMode::None,
        ColorModeArg::Auto if (tty || t.ansi) && !env_set("NO_COLOR") => detect(),
        ColorModeArg::Auto => ColorMode::None,
    };
    let blocks = match t.blocks {
        BlocksArg::Half => Blocks::Half,
        BlocksArg::Full => Blocks::Full,
        BlocksArg::Ascii => Blocks::Ascii,
    };
    AnsiCaps { color, blocks }
}

#[cfg(unix)]
fn ioctl_width() -> Option<usize> {
    // SAFETY: TIOCGWINSZ fills a plain struct and fails with an error code on a non-terminal.
    let ws = unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        (libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) == 0).then_some(ws)
    }?;
    (ws.ws_col > 0).then_some(ws.ws_col as usize)
}

#[cfg(not(unix))]
fn ioctl_width() -> Option<usize> {
    None
}

/// Terminal width in columns: COLUMNS first, then the terminal itself.
pub fn width() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|w| *w > 0)
        .or_else(ioctl_width)
}

pub fn fit(grid: &Grid, caps: AnsiCaps) -> Result<String, String> {
    let cols = grid.width() as usize * caps.columns_per_cell() as usize;
    match width() {
        Some(w) if cols > w => Err(format!(
            "the drawing is {cols} columns wide but the terminal has {w}; use shorter text, a smaller font or a wider terminal"
        )),
        _ => Ok(render_ansi(grid, caps)),
    }
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[?25h\x1b[0m");
        let _ = out.flush();
    }
}

#[cfg(unix)]
fn catch_interrupt() {
    extern "C" fn stop(_: libc::c_int) {
        STOP.store(true, Ordering::Relaxed);
    }
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
    }
}

#[cfg(not(unix))]
fn catch_interrupt() {}

fn nap(ms: u32) {
    let end = Instant::now() + Duration::from_millis(u64::from(ms));
    while !STOP.load(Ordering::Relaxed) {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        std::thread::sleep(left.min(Duration::from_millis(25)));
    }
}

/// Plays the frames in place; `loops` 0 runs until Ctrl-C. The cursor and colors are restored on
/// every way out.
pub fn play(frames: &[Grid], caps: AnsiCaps, frame_ms: u32, loops: u32) -> Result<(), String> {
    let texts: Vec<String> = frames.iter().map(|g| render_ansi(g, caps)).collect();
    let rows = texts[0].lines().count();
    catch_interrupt();
    let mut out = std::io::stdout().lock();
    let _restore = Restore;
    let io = |e: std::io::Error| e.to_string();
    out.write_all(b"\x1b[?25l").map_err(io)?;
    let mut first = true;
    let mut round = 0;
    while !STOP.load(Ordering::Relaxed) && (loops == 0 || round < loops) {
        for text in &texts {
            if STOP.load(Ordering::Relaxed) {
                break;
            }
            if !first {
                write!(out, "\x1b[{rows}A").map_err(io)?;
            }
            first = false;
            out.write_all(text.as_bytes()).map_err(io)?;
            out.flush().map_err(io)?;
            nap(frame_ms);
        }
        round += 1;
    }
    Ok(())
}

pub fn stdout_is_terminal() -> bool {
    std::io::stdout().is_terminal()
}

fn quote_arg(a: &str) -> String {
    let safe = !a.is_empty()
        && a.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if safe {
        a.to_string()
    } else {
        format!("'{}'", a.replace('\'', "'\\''"))
    }
}

fn literal(text: &str, quote: char) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\x1b' => out.push_str("\\x1b"),
            '\n' => out.push_str("\\n"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

pub fn snippet(kind: SnippetFor, args: &[String], color: ColorMode, ansi: &str) -> String {
    let mut line = Vec::new();
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        if a == "--color-mode" {
            rest.next();
        } else if a != "--ansi" && !a.starts_with("--color-mode=") {
            line.push(quote_arg(a));
        }
    }
    let mode = match color {
        ColorMode::TrueColor => "truecolor",
        ColorMode::Ansi256 => "256",
        ColorMode::Ansi16 => "16",
        ColorMode::None => "none",
    };
    line.push(format!("--ansi --color-mode {mode}"));
    let mark = if matches!(kind, SnippetFor::Rust) {
        "//"
    } else {
        "#"
    };
    let head = format!("{mark} regenerate: pixban {}\n", line.join(" "));
    match kind {
        SnippetFor::Shell => format!("{head}printf '%s' $'{}'\n", literal(ansi, '\'')),
        SnippetFor::Python => format!("{head}print(\"{}\", end=\"\")\n", literal(ansi, '"')),
        SnippetFor::Rust => format!("{head}print!(\"{}\");\n", literal(ansi, '"')),
    }
}
