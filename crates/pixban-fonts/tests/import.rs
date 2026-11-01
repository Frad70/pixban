mod common;

use std::path::{Path, PathBuf};

use pixban_core::{render, CaseMode, Font, Grid, LayoutOptions, Spec};
use pixban_fonts::LoadError;

const UPPER: &str = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG 0123456789";
const LOWER: &str = "the quick brown fox jumps over the lazy dog";

fn dir() -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("import");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn grid(font: &Font, text: &str) -> Grid {
    let spec = Spec {
        text: text.to_string(),
        scale: Some(1),
        ..Spec::default()
    };
    render(&spec, font).expect("text renders").grid
}

fn texts_for(font: &Font) -> Vec<&'static str> {
    match font.case() {
        CaseMode::Upper => vec![UPPER],
        CaseMode::Mixed => vec![UPPER, LOWER],
    }
}

fn assert_same_rendering(builtin: &Font, imported: &Font, how: &str) {
    for text in texts_for(builtin) {
        assert_eq!(
            grid(builtin, text),
            grid(imported, text),
            "{} via {how} renders {text:?} differently",
            builtin.name()
        );
    }
}

#[test]
fn bdf_round_trip_renders_like_the_builtin_font() {
    for name in pixban_fonts::names() {
        let builtin = pixban_fonts::get(name).unwrap();
        let path = dir().join(format!("{name}.bdf"));
        std::fs::write(&path, common::to_bdf(&builtin)).unwrap();
        let imported = pixban_fonts::load_file(&path).unwrap();
        assert_same_rendering(&builtin, &imported, "BDF");
    }
}

#[test]
fn pcf_round_trip_renders_like_the_builtin_font_in_both_byte_orders() {
    for name in pixban_fonts::names() {
        let builtin = pixban_fonts::get(name).unwrap();
        for msb in [false, true] {
            let path = dir().join(format!("{name}-{msb}.pcf"));
            std::fs::write(&path, common::to_pcf(&builtin, msb)).unwrap();
            let imported = pixban_fonts::load_file(&path).unwrap();
            assert_same_rendering(&builtin, &imported, "PCF");
        }
    }
}

#[test]
fn gzipped_bdf_and_pcf_load_like_plain_files() {
    let builtin = pixban_fonts::get("slim").unwrap();
    let bdf = dir().join("slim.bdf.gz");
    std::fs::write(&bdf, common::gzip(common::to_bdf(&builtin).as_bytes())).unwrap();
    assert_same_rendering(
        &builtin,
        &pixban_fonts::load_file(&bdf).unwrap(),
        "gzip BDF",
    );
    let pcf = dir().join("slim.pcf.gz");
    std::fs::write(&pcf, common::gzip(&common::to_pcf(&builtin, false))).unwrap();
    assert_same_rendering(
        &builtin,
        &pixban_fonts::load_file(&pcf).unwrap(),
        "gzip PCF",
    );
}

#[test]
fn imported_fonts_keep_the_band_and_extra_rows() {
    let builtin = pixban_fonts::get("slim").unwrap();
    let path = dir().join("slim-rows.bdf");
    std::fs::write(&path, common::to_bdf(&builtin)).unwrap();
    let imported = pixban_fonts::load_file(&path).unwrap();
    assert_eq!(imported.height(), builtin.height());
    let g = imported.glyph('g').unwrap();
    assert_eq!((g.top(), g.bottom()), (0, 2), "descender rows are declared");
    let lay = pixban_core::layout(&imported, "gY", &LayoutOptions::default()).unwrap();
    assert_eq!(lay.height, 7 + 2);
}

fn malformed(err: LoadError) -> String {
    match err {
        LoadError::Malformed { format, reason } => format!("{format}: {reason}"),
        other => panic!("expected a malformed font error, got {other}"),
    }
}

#[test]
fn malformed_bdf_reports_the_line() {
    let src = "STARTFONT 2.1\nFONTBOUNDINGBOX 1 1 0 0\nCHARS 1\nSTARTCHAR a\nENCODING 65\nDWIDTH 1 0\nBBX 1 1 0 0\nBITMAP\nZZ\nENDCHAR\nENDFONT\n";
    let path = dir().join("bad-hex.bdf");
    std::fs::write(&path, src).unwrap();
    let msg = malformed(pixban_fonts::load_file(&path).unwrap_err());
    assert!(msg.contains("line 9"), "{msg}");
    assert!(
        msg.contains("hexadecimal") || msg.contains("hex digits"),
        "{msg}"
    );
}

#[test]
fn truncated_pcf_is_an_error_not_a_panic() {
    let builtin = pixban_fonts::get("tiny").unwrap();
    let mut bytes = common::to_pcf(&builtin, false);
    bytes.truncate(bytes.len() / 2);
    let path = dir().join("truncated.pcf");
    std::fs::write(&path, bytes).unwrap();
    let msg = malformed(pixban_fonts::load_file(&path).unwrap_err());
    assert!(msg.starts_with("PCF:"), "{msg}");
}

#[test]
fn fonts_taller_than_64_rows_are_refused() {
    let src = "STARTFONT 2.1\nFONTBOUNDINGBOX 1 70 0 0\nSTARTPROPERTIES 2\nFONT_ASCENT 40\nFONT_DESCENT 30\nENDPROPERTIES\nCHARS 1\nSTARTCHAR a\nENCODING 65\nDWIDTH 1 0\nBBX 1 1 0 0\nBITMAP\n80\nENDCHAR\nENDFONT\n";
    let path = dir().join("tall.bdf");
    std::fs::write(&path, src).unwrap();
    match pixban_fonts::load_file(&path).unwrap_err() {
        LoadError::TooTall { rows, max } => assert_eq!((rows, max), (70, 64)),
        other => panic!("expected TooTall, got {other}"),
    }
}

#[test]
fn unknown_formats_are_refused() {
    let path = dir().join("garbage.bin");
    std::fs::write(&path, [0u8, 159, 146, 150]).unwrap();
    assert!(pixban_fonts::load_file(&path).is_err());
    let missing = dir().join("does-not-exist.pcf");
    assert!(matches!(
        pixban_fonts::load_file(&missing),
        Err(LoadError::Io(_))
    ));
}
