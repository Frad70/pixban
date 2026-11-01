mod common;

use pixban_core::{layout, Error, LayoutOptions};

fn width_of(text: &str) -> u32 {
    let f = common::test_font();
    layout(&f, text, &LayoutOptions::default()).unwrap().width
}

#[test]
fn widths_add_glyphs_and_spacing() {
    assert_eq!(width_of("A"), 6);
    assert_eq!(width_of("II"), 6 + 2 + 6);
    assert_eq!(width_of("I I"), 6 + 2 + 3 + 2 + 6);
}

#[test]
fn kerning_pulls_pairs_together_in_one_direction_only() {
    assert_eq!(width_of("AI"), 6 + 2 - 1 + 6);
    assert_eq!(width_of("IA"), 6 + 2 + 6);
}

#[test]
fn positions_follow_the_pen() {
    let f = common::test_font();
    let l = layout(&f, "AIA", &LayoutOptions::default()).unwrap();
    let xs: Vec<u32> = l.lines[0].glyphs.iter().map(|g| g.x).collect();
    assert_eq!(xs, vec![0, 7, 15]);
    assert_eq!((l.width, l.height), (21, 7));
}

#[test]
fn case_upper_maps_lowercase_including_cyrillic() {
    assert_eq!(width_of("ai"), width_of("AI"));
    let f = common::test_font();
    let l = layout(&f, "д", &LayoutOptions::default()).unwrap();
    assert!(l.missing.is_empty());
    assert_eq!(l.lines[0].glyphs[0].ch, 'Д');
}

#[test]
fn unknown_characters_use_the_fallback_and_are_reported_once() {
    let f = common::test_font();
    let l = layout(&f, "A?😀?A", &LayoutOptions::default()).unwrap();
    assert_eq!(l.missing, vec!['?', '😀']);
    assert_eq!(l.width, 6 + 2 + 5 + 2 + 5 + 2 + 5 + 2 + 6);
    let flags: Vec<bool> = l.lines[0].glyphs.iter().map(|g| g.is_fallback).collect();
    assert_eq!(flags, vec![false, true, true, true, false]);
}

#[test]
fn rejects_empty_multiline_and_huge_text() {
    let f = common::test_font();
    let o = LayoutOptions::default();
    assert_eq!(layout(&f, "", &o).unwrap_err(), Error::EmptyText);
    assert_eq!(layout(&f, "A\nI", &o).unwrap_err(), Error::MultiLine);
    let long = "A".repeat(pixban_core::limits::MAX_TEXT_CHARS + 1);
    assert!(matches!(
        layout(&f, &long, &o).unwrap_err(),
        Error::TextTooLong { .. }
    ));
    let max = "A".repeat(pixban_core::limits::MAX_TEXT_CHARS);
    assert!(layout(&f, &max, &o).is_ok());
}

#[test]
fn text_wider_than_the_canvas_limit_is_an_error() {
    let src = format!(
        "name: W\nheight: 1\nbaseline: 1\nspacing: 32\ncase: mixed\n---\nchar: A\n{}\n",
        "#".repeat(64)
    );
    let f = pixban_core::font::parse(&src).unwrap();
    let text = "A".repeat(300);
    assert!(matches!(
        layout(&f, &text, &LayoutOptions::default()).unwrap_err(),
        Error::CanvasTooLarge { .. }
    ));
}
