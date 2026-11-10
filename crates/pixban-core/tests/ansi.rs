use pixban_core::{render_ansi, AnsiCaps, Blocks, ColorMode, Grid, Rgba};

const R: Rgba = Rgba(255, 0, 0, 255);
const B: Rgba = Rgba(0, 0, 255, 255);
const T: Rgba = Rgba::TRANSPARENT;

fn grid() -> Grid {
    Grid::from_cells(3, 3, vec![R, T, B, B, T, B, R, R, T])
}

fn caps(color: ColorMode, blocks: Blocks) -> AnsiCaps {
    AnsiCaps { color, blocks }
}

#[test]
fn truecolor_half_blocks() {
    let got = render_ansi(&grid(), caps(ColorMode::TrueColor, Blocks::Half));
    let want =
        "\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m\u{2580}\x1b[0m \x1b[38;2;0;0;255m\u{2588}\x1b[0m\n\
                \x1b[38;2;255;0;0m\u{2580}\u{2580}\x1b[0m \x1b[0m\n";
    assert_eq!(got, want);
}

#[test]
fn palette_modes_pick_nearest_colors() {
    let g = Grid::from_cells(1, 1, vec![R]);
    assert_eq!(
        render_ansi(&g, caps(ColorMode::Ansi256, Blocks::Half)),
        "\x1b[38;5;196m\u{2580}\x1b[0m\n"
    );
    assert_eq!(
        render_ansi(&g, caps(ColorMode::Ansi16, Blocks::Half)),
        "\x1b[91m\u{2580}\x1b[0m\n"
    );
}

#[test]
fn none_mode_has_no_escapes() {
    let got = render_ansi(&grid(), caps(ColorMode::None, Blocks::Half));
    assert_eq!(got, "\u{2588} \u{2588}\n\u{2580}\u{2580} \n");
}

#[test]
fn ascii_and_full_use_two_columns_per_cell() {
    let g = Grid::from_cells(2, 2, vec![R, T, T, R]);
    assert_eq!(
        render_ansi(&g, caps(ColorMode::None, Blocks::Ascii)),
        "##  \n  ##\n"
    );
    assert_eq!(
        render_ansi(&g, caps(ColorMode::None, Blocks::Full)),
        "\u{2588}\u{2588}  \n  \u{2588}\u{2588}\n"
    );
}

#[test]
fn every_colored_line_ends_with_reset_and_transparent_cells_are_spaces() {
    for color in [ColorMode::TrueColor, ColorMode::Ansi256, ColorMode::Ansi16] {
        for blocks in [Blocks::Half, Blocks::Full, Blocks::Ascii] {
            let out = render_ansi(&grid(), caps(color, blocks));
            assert!(out.lines().all(|l| l.ends_with("\x1b[0m")));
            assert!(!out.contains("48;2;0;0;0"));
        }
    }
}
